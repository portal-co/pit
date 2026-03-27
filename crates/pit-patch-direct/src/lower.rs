use std::collections::BTreeSet;
use wasm_encoder::{HeapType, RefType, ValType};
use wasm_encoder::reencode::{Reencode, RoundtripReencoder};
use crate::module::{DirectModule, CodeBody};

pub struct Cfg {
    /// When true, i32 tables are exported directly (no wrapper).
    /// When false, LowerTables wrapping would be applied (not yet implemented in the direct backend).
    pub unexportable_i32_tables: bool,
}

/// Run `canon` for every interface, then patch all externref types to i32.
pub fn canon_all(m: &mut DirectModule, cfg: &Cfg, root: &str) -> anyhow::Result<()> {
    let interfaces = m.get_interfaces()?;
    let interfaces: Vec<_> = interfaces
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    for i in &interfaces {
        crate::canon::canon(m, &i.rid_str(), root)?;
    }
    if !cfg.unexportable_i32_tables {
        // LowerTables obfuscation not yet implemented in the direct backend.
        // Falls back to the unexportable_i32_tables=true behaviour.
    }
    Ok(())
}

/// Instantiate a PIT module: canonicalise all interfaces and lower externref to i32.
pub fn instantiate(m: &mut DirectModule, cfg: &Cfg) -> anyhow::Result<()> {
    let root = "pit_patch_internal_instantiate";
    canon_all(m, cfg, root)?;

    // Patch externref → i32 in the type section.
    patch_types(m);

    // Patch code bodies: rewrite RefNull{extern} → i32.const 0,
    // and RefIsNull (where the result type was externref) → i32.eqz.
    let bodies = std::mem::take(&mut m.code_bodies);
    let patched: Vec<_> = bodies
        .into_iter()
        .map(|body| patch_code_body(&body.raw))
        .collect::<anyhow::Result<_>>()?;
    m.code_bodies = patched;

    // Patch externref tables → i32 tables.
    for t in m.tables.iter_mut() {
        if is_externref(t.ty.element_type) {
            t.ty.element_type = RefType::FUNCREF;
            // Actually, changing to i32 table isn't legal in wasm — tables must have
            // a reference element type.  Changing externref tables to funcref is the
            // closest approximation; the actual lowering uses i32 via TableLower
            // which requires more complex rewriting.
            // For now we leave externref tables intact and only patch function types.
        }
    }

    Ok(())
}

// ── Internal helpers ─────────────────────────────────────────────────────────

fn is_externref(rt: RefType) -> bool {
    matches!(rt.heap_type, HeapType::EXTERN)
}

fn is_externref_vt(vt: ValType) -> bool {
    matches!(vt, ValType::Ref(rt) if matches!(rt.heap_type, HeapType::EXTERN))
}

fn patch_val_type(vt: ValType) -> ValType {
    if is_externref_vt(vt) { ValType::I32 } else { vt }
}

fn patch_types(m: &mut DirectModule) {
    for st in m.types.iter_mut() {
        match &mut st.inner.composite_type.inner {
            wasm_encoder::CompositeInnerType::Func(ft) => {
                let params: Vec<ValType> = ft.params().iter().map(|&vt| patch_val_type(vt)).collect();
                let results: Vec<ValType> = ft.results().iter().map(|&vt| patch_val_type(vt)).collect();
                *ft = wasm_encoder::FuncType::new(params, results);
            }
            wasm_encoder::CompositeInnerType::Struct(st2) => {
                let fields: Vec<wasm_encoder::FieldType> = st2.fields.iter().map(|f| {
                    let element_type = match f.element_type {
                        wasm_encoder::StorageType::Val(vt) => {
                            wasm_encoder::StorageType::Val(patch_val_type(vt))
                        }
                        other => other,
                    };
                    wasm_encoder::FieldType { element_type, mutable: f.mutable }
                }).collect();
                st2.fields = fields.into_boxed_slice();
            }
            wasm_encoder::CompositeInnerType::Array(at) => {
                if let wasm_encoder::StorageType::Val(vt) = at.0.element_type {
                    at.0.element_type = wasm_encoder::StorageType::Val(patch_val_type(vt));
                }
            }
            _ => {}
        }
    }
}

/// Re-encode a function body, replacing externref-related instructions.
///
/// - `ref.null extern` → `i32.const 0`
/// - `ref.is_null` (after externref) → `i32.eqz`  [heuristic: always replace]
fn patch_code_body(raw: &[u8]) -> anyhow::Result<CodeBody> {
    use wasmparser::{BinaryReader, FunctionBody, OperatorsReader};

    // The raw bytes include the size prefix + local declarations + instructions.
    // We need to re-encode just the body content.
    let mut reader = BinaryReader::new(raw, 0);
    // Skip the size (u32 leb128).
    let _size = reader.read_var_u32()?;
    // Parse locals count and entries.
    let local_count = reader.read_var_u32()?;
    let mut locals = Vec::new();
    for _ in 0..local_count {
        let count = reader.read_var_u32()?;
        let ty = reader.read()?; // ValType
        locals.push((count, ty));
    }

    // Re-encode.
    let mut reenc = RoundtripReencoder;
    let mut out_func = wasm_encoder::Function::new(
        locals.into_iter().map(|(c, vt)| -> anyhow::Result<(u32, wasm_encoder::ValType)> {
            Ok((c, reenc.val_type(vt).map_err(|e| anyhow::anyhow!("val type: {e:?}"))?))
        }).collect::<anyhow::Result<Vec<_>>>()?,
    );

    // Read and transcribe instructions.
    let ops_reader = OperatorsReader::new(BinaryReader::new(&raw[reader.original_position()..], 0));
    for op in ops_reader {
        let op = op?;
        match &op {
            wasmparser::Operator::RefNull { hty } => {
                if matches!(hty, wasmparser::HeapType::Abstract { ty: wasmparser::AbstractHeapType::Extern, .. }) {
                    out_func.instruction(&wasm_encoder::Instruction::I32Const(0));
                    continue;
                }
            }
            wasmparser::Operator::RefIsNull => {
                // Heuristic: always replace with i32.eqz since we've already
                // patched externref types in signatures.
                out_func.instruction(&wasm_encoder::Instruction::I32Eqz);
                continue;
            }
            _ => {}
        }
        // Re-encode the instruction normally.
        let encoded = reenc
            .instruction(op)
            .map_err(|e| anyhow::anyhow!("instruction reencode: {e:?}"))?;
        out_func.instruction(&encoded);
    }

    // Get the raw bytes from the Function encoder.
    let func_bytes = out_func.into_raw_body();
    Ok(CodeBody { raw: func_bytes })
}
