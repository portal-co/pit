use alloc::vec::Vec;
use pit_core::{Arg, Sig};

/// A backend-agnostic wasm value type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValTy {
    I32,
    I64,
    F32,
    F64,
    /// Nullable externref (used for PIT resources in non-tpit mode).
    ExternRef,
}

/// Map a PIT argument to its wasm value type.
///
/// When `tpit` is true, resource types are represented as `i32` (table indices).
/// When `tpit` is false, resource types are `externref`.
pub fn pit_arg_to_val_ty(arg: &Arg, tpit: bool) -> ValTy {
    match arg {
        Arg::I32 => ValTy::I32,
        Arg::I64 => ValTy::I64,
        Arg::F32 => ValTy::F32,
        Arg::F64 => ValTy::F64,
        Arg::Resource { .. } => {
            if tpit {
                ValTy::I32
            } else {
                ValTy::ExternRef
            }
        }
        _ => todo!("unsupported arg type"),
    }
}

/// Return `(params, rets)` for a PIT method signature, with the leading
/// self-argument prepended to params.
///
/// The self arg is `I32` in tpit mode, `ExternRef` otherwise.
pub fn sig_to_val_tys(sig: &Sig, tpit: bool) -> (Vec<ValTy>, Vec<ValTy>) {
    let self_ty = if tpit { ValTy::I32 } else { ValTy::ExternRef };
    let params: Vec<ValTy> = core::iter::once(self_ty)
        .chain(sig.params.iter().map(|a| pit_arg_to_val_ty(a, tpit)))
        .collect();
    let rets: Vec<ValTy> = sig.rets.iter().map(|a| pit_arg_to_val_ty(a, tpit)).collect();
    (params, rets)
}
