use wasm_encoder::{HeapType, RefType, ValType};
use pit_core::{Arg, Sig};
use crate::module::DirectModule;

/// Convert a PIT argument type to a wasm-encoder `ValType`.
///
/// When `tpit` is true, resource types are `i32`; otherwise they are `externref`.
pub fn pit_arg_to_val_type(arg: &Arg, tpit: bool) -> ValType {
    match arg {
        Arg::I32 => ValType::I32,
        Arg::I64 => ValType::I64,
        Arg::F32 => ValType::F32,
        Arg::F64 => ValType::F64,
        Arg::Resource { .. } => {
            if tpit {
                ValType::I32
            } else {
                externref_type()
            }
        }
        _ => todo!("unsupported arg type"),
    }
}

/// Returns the wasm-encoder `ValType` for a nullable `externref`.
pub fn externref_type() -> ValType {
    ValType::Ref(RefType {
        nullable: true,
        heap_type: HeapType::EXTERN,
    })
}

/// Returns the wasm-encoder `RefType` for a nullable `externref`.
pub fn externref_ref_type() -> RefType {
    RefType { nullable: true, heap_type: HeapType::EXTERN }
}

/// Returns `(params, results)` for a PIT method signature, with the leading
/// self-arg prepended.
pub fn sig_to_val_types(sig: &Sig, tpit: bool) -> (Vec<ValType>, Vec<ValType>) {
    let self_ty = if tpit { ValType::I32 } else { externref_type() };
    let params: Vec<ValType> = std::iter::once(self_ty)
        .chain(sig.params.iter().map(|a| pit_arg_to_val_type(a, tpit)))
        .collect();
    let results: Vec<ValType> = sig.rets.iter().map(|a| pit_arg_to_val_type(a, tpit)).collect();
    (params, results)
}

/// Find or create a func type for a PIT method signature; returns type index.
pub fn pit_sig_type_index(m: &mut DirectModule, sig: &Sig, tpit: bool) -> u32 {
    let (params, results) = sig_to_val_types(sig, tpit);
    m.add_func_type(&params, &results)
}

/// Find or add an import function for a PIT interface method.
/// Returns the func index.
pub fn ensure_pit_func(
    m: &mut DirectModule,
    rid_str: &str,
    method_name: &str,
    sig: &Sig,
    tpit: bool,
) -> u32 {
    let module_name = if tpit {
        format!("{}tpit", rid_str)
    } else {
        format!("pit/{}", rid_str)
    };
    let type_index = pit_sig_type_index(m, sig, tpit);
    m.ensure_import_func(&module_name, method_name, type_index)
}
