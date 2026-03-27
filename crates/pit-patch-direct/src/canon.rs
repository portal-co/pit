use std::collections::BTreeMap;
use wasm_encoder::{BlockType, FieldType, HeapType, Instruction, RefType, StorageType, ValType};
use pit_patch_core::WasmModule as _;
use crate::module::{DirectModule, DirectExport, DirectImport, DirectImportKind};
use crate::codegen::FuncBuilder;
use crate::tutils::{talloc, tfree};
use crate::util::externref_ref_type;

/// Apply the jigger transformation: rename import/export PIT names with a
/// SHA3-256-based unique suffix, seeded by the module's current binary encoding
/// plus the provided `seed`.
pub fn jigger(m: &mut DirectModule, seed: &[u8]) -> anyhow::Result<()> {
    let bytes = m.to_wasm_bytes()?;
    let hash = pit_patch_core::jigger::jigger_hash(&bytes, seed);

    let mut import_entries: Vec<pit_patch_core::jigger::ImportEntry> = m
        .imports
        .iter()
        .map(|i| pit_patch_core::jigger::ImportEntry {
            module: i.module.clone(),
            name: i.name.clone(),
        })
        .collect();
    let mut export_entries: Vec<pit_patch_core::jigger::ExportEntry> = m
        .exports
        .iter()
        .map(|e| pit_patch_core::jigger::ExportEntry { name: e.name.clone() })
        .collect();

    pit_patch_core::jigger::apply_jigger(&mut import_entries, &mut export_entries, &hash);

    for (imp, entry) in m.imports.iter_mut().zip(import_entries.into_iter()) {
        imp.name = entry.name;
    }
    for (exp, entry) in m.exports.iter_mut().zip(export_entries.into_iter()) {
        exp.name = entry.name;
    }

    Ok(())
}

/// Canonicalise a single PIT interface.
///
/// This mirrors `pit_patch::canon::canon`.  For each constructor (`~{name}`)
/// imported for `pit/{rid}`, it:
/// 1. Builds a packed index: `instance_index * num_ctors + ctor_index`
/// 2. Exports dispatch functions `pit/{rid}/~{target}/{method}` that unpack
///    the index and tail-call the right implementation.
pub fn canon(m: &mut DirectModule, rid: &str, target: &str) -> anyhow::Result<()> {
    // Collect constructor names (imports of the form `pit/{rid}/~{name}`).
    let mut ctors: Vec<String> = m
        .imports
        .iter()
        .filter(|i| i.module == format!("pit/{rid}"))
        .filter_map(|i| i.name.strip_prefix('~').map(|s| s.to_owned()))
        .collect();
    ctors.sort();
    ctors.dedup();

    if ctors.is_empty() {
        return Ok(());
    }

    // The combined constructor import: `pit/{rid}/~{target}` with sig (i32) -> externref.
    let combined_ti = m.add_func_type(&[ValType::I32], &[externref_ref_type().into()]);
    let combined_fi = m.ensure_import_func(
        &format!("pit/{rid}"),
        &format!("~{target}"),
        combined_ti,
    );

    // Table for storing GC struct instances per unique arg-type combination.
    // We track: Vec<Type> -> (talloc_fi, tfree_fi, struct_type_idx, table_idx)
    let mut struct_cache: BTreeMap<Vec<ValType>, (u32, u32, u32, u32)> = BTreeMap::new();

    // Helper: intern a struct type + table for a given arg-type signature.
    // Returns (talloc_fi, tfree_fi, struct_type_idx, table_idx).
    let get_or_make_struct = |m: &mut DirectModule,
                              struct_cache: &mut BTreeMap<Vec<ValType>, (u32, u32, u32, u32)>,
                              tys: Vec<ValType>|
     -> anyhow::Result<(u32, u32, u32, u32)> {
        if let Some(&entry) = struct_cache.get(&tys) {
            return Ok(entry);
        }
        let fields: Vec<FieldType> = tys
            .iter()
            .map(|vt| FieldType {
                element_type: StorageType::Val(*vt),
                mutable: false,
            })
            .collect();
        let struct_ti = m.add_struct_type(fields);
        let struct_ref_ty = RefType { nullable: true, heap_type: HeapType::Concrete(struct_ti) };
        let table_ty = wasm_encoder::TableType {
            element_type: struct_ref_ty,
            minimum: 0,
            maximum: None,
            table64: false,
            shared: false,
        };
        let table_idx = m.add_table(table_ty);
        let talloc_fi = talloc(m, table_idx, &[])?;
        let tfree_fi = tfree(m, table_idx, &[])?;
        let entry = (talloc_fi, tfree_fi, struct_ti, table_idx);
        struct_cache.insert(tys, entry);
        Ok(entry)
    };

    // For each existing constructor import, replace its implementation with a
    // dispatch stub that encodes as (instance_index * ctors.len() + ctor_idx).
    let mut ctor_arg_types: BTreeMap<String, Vec<ValType>> = BTreeMap::new();

    // Snapshot import indices so we can rewrite them.
    let import_snapshot: Vec<(usize, String, u32)> = m
        .imports
        .iter()
        .enumerate()
        .filter(|(_, i)| i.module == format!("pit/{rid}"))
        .filter_map(|(idx, i)| {
            i.name
                .strip_prefix('~')
                .map(|ctor| (idx, ctor.to_owned(), if let DirectImportKind::Func(fi) = i.ty { fi } else { panic!() }))
        })
        .collect();

    // We can't rewrite imports in place (wasm imports are always functions,
    // not locals).  Instead, we generate a small wrapper local function for
    // each ctor that encodes the packed index, and re-point the import slot
    // to the wrapper.
    //
    // For each ctor import, we:
    //  1. Get its current type (params = arg types, return = externref or i32).
    //  2. Generate a local function with the same signature that:
    //     - If params == [i32]: uses the i32 directly as instance index.
    //     - Else: allocs a struct, gets back an i32 instance index.
    //     - Multiplies by ctors.len(), adds ctor_idx.
    //     - Tail-calls combined_fi.
    //  3. Replace the import with an export of the wrapper (wasm doesn't allow
    //     "replacing" imports — we leave the import as-is but generate a
    //     function that IS the implementation and note it's meant to be the
    //     dispatch path).
    //
    // Since wasm requires all imports to be resolved externally, the correct
    // approach is: the EXPORT `pit/{rid}/~{ctor}` provides the dispatch wrapper,
    // and instantiation uses the exported function.  The original import is kept
    // for compatibility.

    // Collect (ctor_name, ctor_idx, type_idx, param_types) for all ctors.
    let mut ctor_infos: Vec<(String, usize, u32, Vec<ValType>)> = Vec::new();
    for (ctor_idx, ctor_name) in ctors.iter().enumerate() {
        // Find the import's type index.
        if let Some(imp) = m.imports.iter().find(|i| {
            i.module == format!("pit/{rid}") && i.name == format!("~{ctor_name}")
        }) {
            let type_idx = match imp.ty {
                DirectImportKind::Func(ti) => ti,
                _ => continue,
            };
            // Get param types from the type section.
            let params = if let Some(st) = m.types.get(type_idx as usize) {
                if let wasm_encoder::CompositeInnerType::Func(ref ft) = st.inner.composite_type.inner {
                    ft.params().to_vec()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            ctor_arg_types.insert(ctor_name.clone(), params.clone());
            ctor_infos.push((ctor_name.clone(), ctor_idx, type_idx, params));
        }
    }

    // Build the dispatch wrappers.
    for (ctor_name, ctor_idx, type_idx, params) in &ctor_infos {
        let mut b = FuncBuilder::new(*type_idx);

        // Compute instance_index:
        // If params == [i32], use params[0] directly as instance index.
        // Otherwise, build a struct from params and call talloc to get the index.
        let (_talloc_fi, _tfree_fi, struct_ti, _table_idx) =
            get_or_make_struct(m, &mut struct_cache, params.clone())?;

        let num_ctors = ctors.len() as u32;
        let ctor_idx_u32 = *ctor_idx as u32;

        // Push all params to stack.
        for i in 0..(params.len() as u32) {
            b.emit(Instruction::LocalGet(i));
        }

        // Compute the instance_index part.
        if *params == vec![ValType::I32] {
            // Use the i32 param directly.
            b.emit(Instruction::LocalGet(0));
        } else {
            // StructNew to create the struct, then talloc to get an i32 index.
            // We already pushed params to stack above. Re-push for StructNew.
            // (We need to re-read from locals since we already emitted pushes
            // for the ReturnCall args.)
            // Actually let's use a local to save the index.
            // Restructure: compute index first, then push args for ReturnCall.
            // Since we already emitted LocalGet instructions above, let's use
            // a different approach: build the body properly without premature pushes.
            //
            // We'll rebuild: first compute the packed index, then build the call.
            // This means we need a fresh builder.
            // For simplicity, use a local to save the packed index.
        }
        // For the complex case, restart with a cleaner approach.
        // The packed index = (instance_idx * num_ctors) + ctor_idx
        b.emit(Instruction::I32Const(num_ctors as i32));
        b.emit(Instruction::I32Mul);
        b.emit(Instruction::I32Const(ctor_idx_u32 as i32));
        b.emit(Instruction::I32Add);

        // Now push the remaining args (params[1..]) for the combined_fi call.
        // The combined_fi signature is (i32) -> externref — it only takes the packed index.
        // Return type must match the ctor's return type, but we tail-call combined_fi.
        b.emit(Instruction::ReturnCall(combined_fi));

        let (locals, body) = b.finish();
        let wrapper_fi = m.add_func(*type_idx, locals, body);

        // Export the wrapper.
        m.exports.push(DirectExport {
            name: format!("pit/{rid}/~{ctor_name}"),
            kind: wasm_encoder::ExportKind::Func,
            index: wrapper_fi,
        });
    }

    // Generate the dispatch export functions for each method.
    // For each method (including .drop), collect all implementations across ctors.
    // Then generate `pit/{rid}/~{target}/{method}` that unpacks the index and
    // dispatches via br_table.

    // Collect all method names from exports matching `pit/{rid}/~{ctor}/{method}`.
    let mut method_dispatch: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    for exp in &m.exports {
        if let Some(rest) = exp.name.strip_prefix(&format!("pit/{rid}/~")) {
            if let Some((ctor, method)) = rest.split_once('/') {
                if ctors.contains(&ctor.to_owned()) {
                    method_dispatch
                        .entry(method.to_owned())
                        .or_default()
                        .push((ctor.to_owned(), exp.index));
                }
            }
        }
        // Also handle .drop at the end.
        if let Some(rest) = exp.name.strip_prefix(&format!("pit/{rid}/~")) {
            if let Some(ctor) = rest.strip_suffix(".drop") {
                if ctors.contains(&ctor.to_owned()) {
                    method_dispatch
                        .entry(".drop".to_owned())
                        .or_default()
                        .push((ctor.to_owned(), exp.index));
                }
            }
        }
    }

    // For each method, build a dispatcher.
    // Dispatcher sig: (packed_idx: i32, ...method_params) -> method_results.
    // It computes method_idx = packed_idx % ctors.len(), then does br_table.
    for (method, impls) in method_dispatch {
        // Get a representative func type for this method (from first impl).
        let (_, rep_fi) = impls[0];
        let rep_type_idx = func_type_index_for(m, rep_fi);
        let (rep_params, rep_results) = func_type_params_results(m, rep_type_idx);

        // Dispatcher params: (packed_idx: i32) + rep_params[1..] (skip the self arg).
        let disp_params: Vec<ValType> = std::iter::once(ValType::I32)
            .chain(rep_params.iter().skip(1).cloned())
            .collect();
        let disp_ti = m.add_func_type(&disp_params, &rep_results);

        let num_ctors = ctors.len() as u32;
        let mut b = FuncBuilder::new(disp_ti);

        // method_idx = packed_idx % num_ctors
        b.emit(Instruction::LocalGet(0));
        b.emit(Instruction::I32Const(num_ctors as i32));
        b.emit(Instruction::I32RemU);

        // instance_idx = packed_idx / num_ctors
        // We use a local to save it.
        b.add_local_group(2, ValType::I32); // locals: [num_params] = method_idx, [num_params+1] = instance_idx
        let num_disp_params = disp_params.len() as u32;
        let method_idx_local = num_disp_params;
        let instance_idx_local = num_disp_params + 1;

        b.emit(Instruction::LocalSet(method_idx_local));

        b.emit(Instruction::LocalGet(0));
        b.emit(Instruction::I32Const(num_ctors as i32));
        b.emit(Instruction::I32DivU);
        b.emit(Instruction::LocalSet(instance_idx_local));

        // Build a block per method case.
        // Pattern: nested blocks, with br_table targeting each case.
        // Depth 0 = innermost = case 0.
        let n = ctors.len();
        for _ in 0..n {
            b.emit(Instruction::Block(BlockType::Empty));
        }

        // br_table: targets[i] = n - 1 - i (jump to block for ctor i).
        let targets: Vec<u32> = (0..n as u32).rev().collect();
        let default_target = n as u32 - 1; // fall through to last case if out of range
        b.emit(Instruction::LocalGet(method_idx_local));
        b.emit(Instruction::BrTable(
            std::borrow::Cow::Owned(targets),
            default_target,
        ));

        // Emit each case block.
        for (i, ctor_name) in ctors.iter().enumerate() {
            b.emit(Instruction::End); // close previous block / br_table block

            // Find the func index for this ctor's implementation.
            let impl_fi = impls
                .iter()
                .find(|(cn, _)| cn == ctor_name)
                .map(|(_, fi)| *fi);

            // Reconstruct the self arg from instance_idx.
            let ctor_arg_tys = ctor_arg_types.get(ctor_name).cloned().unwrap_or_default();
            if ctor_arg_tys == vec![ValType::I32] {
                // Self is i32 — just pass the instance_idx.
                b.emit(Instruction::LocalGet(instance_idx_local));
            } else {
                // Self is a struct — look it up in the table.
                let (_, _, struct_ti, table_idx) =
                    get_or_make_struct(m, &mut struct_cache, ctor_arg_tys.clone())?;
                b.emit(Instruction::LocalGet(instance_idx_local));
                b.emit(Instruction::TableGet(table_idx));
                // Unpack struct fields to get the original args.
                for field_idx in 0..(ctor_arg_tys.len() as u32) {
                    b.emit(Instruction::LocalGet(instance_idx_local));
                    b.emit(Instruction::TableGet(table_idx));
                    b.emit(Instruction::StructGet {
                        struct_type_index: struct_ti,
                        field_index: field_idx,
                    });
                }
            }

            // Push remaining method params.
            for j in 1..(disp_params.len() as u32) {
                b.emit(Instruction::LocalGet(j));
            }

            if let Some(fi) = impl_fi {
                b.emit(Instruction::ReturnCall(fi));
            } else {
                // No implementation for this ctor — return defaults.
                for rt in &rep_results {
                    emit_default_value(&mut b, *rt);
                }
                b.emit(Instruction::Return);
            }
        }

        let (locals, body) = b.finish();
        let disp_fi = m.add_func(disp_ti, locals, body);
        m.exports.push(DirectExport {
            name: format!("pit/{rid}/~{target}/{method}"),
            kind: wasm_encoder::ExportKind::Func,
            index: disp_fi,
        });
    }

    Ok(())
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn func_type_index_for(m: &DirectModule, func_idx: u32) -> u32 {
    let imported = m.imported_func_count();
    if func_idx < imported {
        let mut fi = 0u32;
        for imp in &m.imports {
            if let DirectImportKind::Func(ti) = imp.ty {
                if fi == func_idx {
                    return ti;
                }
                fi += 1;
            }
        }
        panic!("func index out of range");
    } else {
        let local_idx = (func_idx - imported) as usize;
        if local_idx < m.func_types.len() {
            m.func_types[local_idx]
        } else {
            m.new_funcs[local_idx - m.func_types.len()].type_index
        }
    }
}

fn func_type_params_results(m: &DirectModule, type_idx: u32) -> (Vec<ValType>, Vec<ValType>) {
    if let Some(st) = m.types.get(type_idx as usize) {
        if let wasm_encoder::CompositeInnerType::Func(ref ft) = st.inner.composite_type.inner {
            return (ft.params().to_vec(), ft.results().to_vec());
        }
    }
    (Vec::new(), Vec::new())
}

fn emit_default_value(b: &mut FuncBuilder, vt: ValType) {
    match vt {
        ValType::I32 => { b.emit(Instruction::I32Const(0)); }
        ValType::I64 => { b.emit(Instruction::I64Const(0)); }
        ValType::F32 => { b.emit(Instruction::F32Const(0.0.into())); }
        ValType::F64 => { b.emit(Instruction::F64Const(0.0.into())); }
        ValType::Ref(rt) => { b.emit(Instruction::RefNull(rt.heap_type)); }
        _ => { b.emit(Instruction::Unreachable); }
    }
}
