use wasm_encoder::{BlockType, HeapType, Instruction, RefType, ValType};
use pit_core::Arg;
use crate::module::{DirectModule, DirectExport, DirectImportKind};
use crate::codegen::FuncBuilder;
use crate::tutils::{talloc, tfree};
use crate::util::{externref_ref_type, pit_arg_to_val_type};

/// Convert a TPIT module (i32-based) to a PIT module (externref-based).
///
/// For each import/export under `tpit/{rid}`, generate shim functions that
/// translate between the two calling conventions.
pub fn wrap(m: &mut DirectModule) -> anyhow::Result<()> {
    let interfaces = m.get_interfaces()?;

    // Create a single externref table for the shim's resource storage.
    let ext_ref_ty = wasm_encoder::TableType {
        element_type: externref_ref_type(),
        minimum: 0,
        maximum: None,
        table64: false,
        shared: false,
    };
    let table_idx = m.add_table(ext_ref_ty);
    let talloc_fi = talloc(m, table_idx, &[])?;
    let tfree_fi = tfree(m, table_idx, &[])?;

    // Export the allocation helpers and the table.
    m.exports.push(DirectExport {
        name: "tpit_alloc".to_owned(),
        kind: wasm_encoder::ExportKind::Func,
        index: talloc_fi,
    });
    m.exports.push(DirectExport {
        name: "tpit_free".to_owned(),
        kind: wasm_encoder::ExportKind::Func,
        index: tfree_fi,
    });
    m.exports.push(DirectExport {
        name: "tpit_table".to_owned(),
        kind: wasm_encoder::ExportKind::Table,
        index: table_idx,
    });

    for iface in &interfaces {
        let rid = iface.rid_str();

        // For each `tpit/{rid}/{method}` import, generate a shim that:
        // - Translates externref args → i32 (via talloc for owned, TableGet for borrow)
        // - Calls the underlying tpit function
        // - Translates i32 results → externref (via TableGet)
        let tpit_imports: Vec<(String, u32)> = m
            .imports
            .iter()
            .filter_map(|imp| {
                if imp.module == format!("tpit/{rid}") {
                    if let DirectImportKind::Func(ti) = imp.ty {
                        return Some((imp.name.clone(), ti));
                    }
                }
                None
            })
            .collect();

        for (method_name, type_idx) in tpit_imports {
            let (params, results) = func_type_params_results(m, type_idx);

            // Build the shim function.
            // The shim has the PIT (externref) signature.
            let pit_params: Vec<ValType> = params
                .iter()
                .enumerate()
                .map(|(i, &vt)| {
                    if is_i32(vt) && i == 0 {
                        // First param is the 'self' i32 → externref.
                        ValType::Ref(externref_ref_type())
                    } else if is_i32(vt) {
                        // Other i32 params that correspond to resource args stay as-is
                        // (we don't have arg type info here, so we keep i32).
                        vt
                    } else {
                        vt
                    }
                })
                .collect();
            let pit_results: Vec<ValType> = results
                .iter()
                .map(|&vt| {
                    if is_i32(vt) {
                        // i32 result → externref.
                        ValType::Ref(externref_ref_type())
                    } else {
                        vt
                    }
                })
                .collect();

            let shim_ti = m.add_func_type(&pit_params, &pit_results);
            let mut b = FuncBuilder::new(shim_ti);

            // Find the tpit import func index.
            let mut tpit_fi = 0u32;
            let mut fi = 0u32;
            for imp in &m.imports {
                if let DirectImportKind::Func(_) = imp.ty {
                    if imp.module == format!("tpit/{rid}") && imp.name == method_name {
                        tpit_fi = fi;
                        break;
                    }
                    fi += 1;
                }
            }

            // For each param, translate externref → i32 via talloc if needed.
            // We save i32 indices in extra locals.
            let num_params = pit_params.len() as u32;
            b.add_local_group(num_params, ValType::I32); // i32 versions of params

            let tpit_self_local = num_params; // first extra local = tpit self index
            // Translate self (param 0, externref) → i32 via talloc.
            b.emit(Instruction::LocalGet(0)); // externref self
            b.emit(Instruction::Call(talloc_fi));
            b.emit(Instruction::LocalTee(tpit_self_local));

            // Push remaining params to call the tpit function.
            for i in 1..num_params {
                b.emit(Instruction::LocalGet(i));
            }

            b.emit(Instruction::Call(tpit_fi));

            // If the result is i32 (a table index), translate back to externref.
            for &rt in &pit_results {
                if is_externref_vt(rt) {
                    // Result is on stack as i32 — get the externref from table.
                    b.emit(Instruction::TableGet(table_idx));
                }
            }

            b.emit(Instruction::Return);
            let (locals, body) = b.finish();
            let shim_fi = m.add_func(shim_ti, locals, body);

            // Export the shim as the PIT method.
            m.exports.push(DirectExport {
                name: format!("pit/{rid}/{method_name}"),
                kind: wasm_encoder::ExportKind::Func,
                index: shim_fi,
            });
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn func_type_params_results(m: &DirectModule, type_idx: u32) -> (Vec<ValType>, Vec<ValType>) {
    if let Some(st) = m.types.get(type_idx as usize) {
        if let wasm_encoder::CompositeInnerType::Func(ref ft) = st.inner.composite_type.inner {
            return (ft.params().to_vec(), ft.results().to_vec());
        }
    }
    (Vec::new(), Vec::new())
}

fn is_i32(vt: ValType) -> bool {
    matches!(vt, ValType::I32)
}

fn is_externref_vt(vt: ValType) -> bool {
    matches!(vt, ValType::Ref(rt) if matches!(rt.heap_type, HeapType::EXTERN))
}

/// Generate a shim for a single arg/result: translate between i32 (tpit) and externref (pit).
///
/// - `retref=true`: i32 → externref (look up in table, or 0 → null)
/// - `retref=false`: externref → i32 (store in table via talloc, or null → 0)
pub fn shim(
    b: &mut FuncBuilder,
    retref: bool,
    local_idx: u32,
    talloc_fi: u32,
    tfree_fi: u32,
    take: bool,
    table_idx: u32,
) {
    if retref {
        // i32 → externref
        // if local_idx == 0: RefNull; else: local_idx - 1 → TableGet
        b.emit(Instruction::LocalGet(local_idx));
        b.emit(Instruction::I32Const(0));
        b.emit(Instruction::I32Eq);
        b.emit(Instruction::If(BlockType::Result(ValType::Ref(externref_ref_type()))));
        b.emit(Instruction::RefNull(HeapType::EXTERN));
        b.emit(Instruction::Else);
        b.emit(Instruction::LocalGet(local_idx));
        b.emit(Instruction::I32Const(1));
        b.emit(Instruction::I32Sub);
        b.emit(Instruction::TableGet(table_idx));
        b.emit(Instruction::End);
    } else {
        // externref → i32
        // if null: 0; else: talloc(val) + 1
        b.emit(Instruction::LocalGet(local_idx));
        b.emit(Instruction::RefIsNull);
        b.emit(Instruction::If(BlockType::Result(ValType::I32)));
        b.emit(Instruction::I32Const(0));
        b.emit(Instruction::Else);
        b.emit(Instruction::LocalGet(local_idx));
        b.emit(Instruction::Call(talloc_fi));
        b.emit(Instruction::I32Const(1));
        b.emit(Instruction::I32Add);
        b.emit(Instruction::End);
    }
}
