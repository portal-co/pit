//! # PIT Rust Host Library
//!
//! Runtime library for hosting PIT-enabled WebAssembly modules.
//!
//! This crate provides the runtime infrastructure for loading and executing
//! WebAssembly modules that use PIT interfaces. It handles:
//!
//! - Setting up import functions for PIT interfaces
//! - Managing resource lifecycles with proper drop semantics
//! - Bridging between Rust trait objects and WebAssembly externrefs
//!
//! ## Usage
//!
//! ```ignore
//! use pit_rust_host_lib::{init, emit, Wrapped};
//! use wasm_runtime_layer::{Imports, Store, Module};
//!
//! let mut imports = Imports::new();
//! init(&mut imports, &mut store);
//! emit(&mut imports, interface.into(), &module, &mut store);
//! ```
//!
//! ## no_std
//!
//! This crate is `no_std` compatible, using `alloc` for dynamic allocation
//! and `wasm_runtime_layer` as the WebAssembly runtime abstraction.

#![no_std]
#[doc(hidden)]
pub extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
#[doc(hidden)]
pub use core;
use core::{
    cell::UnsafeCell,
    iter::{empty, once},
};
use pit_core::{Arg, Interface};
use wasm_runtime_layer::{
    backend::WasmEngine, AsContext, AsContextMut, Extern, ExternRef, Func, FuncType, Imports,
    Instance, Module, Store, StoreContext, StoreContextMut, Value, ValueType,
};

/// Initializes the PIT drop function in the imports table.
///
/// This function sets up the `pit.drop` import which is called when a resource
/// handle is dropped. It must be called before instantiating any PIT module.
///
/// # Arguments
///
/// * `l` - The imports table to add the drop function to
/// * `ctx` - A mutable store context
pub fn init<U: AsRef<Instance> + 'static, E: WasmEngine>(
    l: &mut Imports,
    ctx: &mut impl AsContextMut<UserState = U, Engine = E>,
) {
    l.define(
        "pit",
        "drop",
        Extern::Func(Func::new(
            &mut *ctx,
            FuncType::new(once(ValueType::ExternRef), empty()),
            move |mut ctx, args, rets| {
                let Value::ExternRef(Some(a)) = args[0].clone() else {
                    anyhow::bail!("invalid type")
                };
                let Ok(x): Result<&Wrapped<U, E>, anyhow::Error> =
                    a.downcast::<'_, '_, Wrapped<U, E>, U, E>(ctx.as_context())
                else {
                    return Ok(());
                };
                let f = x.all[0].clone();
                f(ctx.as_context_mut(), vec![])?;
                Ok(())
            },
        )),
    );
}
/// Converts a PIT argument type to a WebAssembly value type.
///
/// # Arguments
///
/// * `a` - The PIT argument type
///
/// # Returns
///
/// The corresponding WebAssembly value type.
pub fn emit_ty(a: &Arg) -> ValueType {
    match a {
        Arg::I32 => ValueType::I32,
        Arg::I64 => ValueType::I64,
        Arg::F32 => ValueType::F32,
        Arg::F64 => ValueType::F64,
        Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => ValueType::ExternRef,
        _ => todo!(), // Arg::Func(f) => ValueType::FuncRef,
    }
}
/// Emits import functions for a PIT interface.
///
/// This function sets up all the import functions needed by a WebAssembly module
/// to consume a PIT interface. It creates:
/// - Method dispatch functions for each interface method
/// - Instance creation functions for each unique ID
///
/// # Arguments
///
/// * `l` - The imports table to add functions to
/// * `rid` - The interface definition
/// * `m` - The WebAssembly module being instantiated
/// * `ctx` - A mutable store context
pub fn emit<U: AsRef<Instance> + 'static, E: WasmEngine>(
    l: &mut Imports,
    rid: Arc<Interface>,
    m: &Module,
    ctx: &mut impl AsContextMut<UserState = U, Engine = E>,
) {
    let n = format!("pit/{}", rid.rid_str());
    for (j, (i, m)) in rid.methods.iter().enumerate() {
        l.define(
            &n,
            i.as_str(),
            Extern::Func(Func::new(
                &mut *ctx,
                FuncType::new(
                    once(ValueType::ExternRef).chain(m.params.iter().map(emit_ty)),
                    m.rets.iter().map(emit_ty),
                ),
                move |mut ctx, args, rets| {
                    let Value::ExternRef(Some(a)) = args[0].clone() else {
                        anyhow::bail!("invalid type")
                    };
                    let x: &Wrapped<U, E> = a.downcast(ctx.as_context())?;
                    let t = x.all[j + 1].clone();
                    let rets2 = t(ctx.as_context_mut(), args[1..].iter().cloned().collect())?;
                    for (r, s) in rets2.into_iter().zip(rets.iter_mut()) {
                        *s = r;
                    }
                    Ok(())
                },
            )),
        )
    }
    let i = m
        .imports(ctx.as_context().engine())
        .map(|a| (a.module.to_owned(), a.name.to_owned()))
        .collect::<Vec<_>>();
    for i in i {
        if i.0 == n {
            if let Some(t) = i.1.strip_prefix("~") {
                let t = t.to_owned();
                let rid = rid.clone();
                l.define(
                    &n,
                    &i.1,
                    Extern::Func(Func::new(
                        &mut *ctx,
                        FuncType::new(once(ValueType::I32), once(ValueType::ExternRef)),
                        move |mut ctx, args, rets| {
                            let i = ctx.data().as_ref().clone();
                            let object = Wrapped::new(
                                args.to_owned(),
                                rid.clone(),
                                t.to_owned(),
                                i,
                                ctx.as_context_mut(),
                            );
                            rets[0] = Value::ExternRef(Some(ExternRef::new(ctx, object)));
                            Ok(())
                        },
                    )),
                )
            };
        };
    }
}
/// A wrapped PIT resource.
///
/// This struct holds a PIT interface reference along with the closure functions
/// needed to implement each method and the drop handler.
///
/// # Type Parameters
///
/// * `U` - The user state type for the store
/// * `E` - The WebAssembly engine backend
pub struct Wrapped<U: 'static, E: wasm_runtime_layer::backend::WasmEngine> {
    /// The interface definition this resource implements.
    pub rid: Arc<Interface>,
    /// Method implementations: index 0 is the drop handler, followed by each method.
    pub all: Vec<
        Arc<
            dyn Fn(StoreContextMut<'_, U, E>, Vec<Value>) -> anyhow::Result<Vec<Value>>
                + Send
                + Sync,
        >,
    >,
    // },
}
impl<U: 'static, E: wasm_runtime_layer::backend::WasmEngine> Wrapped<U, E> {
    /// Creates a new wrapped resource from a WebAssembly instance.
    ///
    /// This function creates a `Wrapped` resource that delegates method calls
    /// to the corresponding export functions in the WebAssembly instance.
    ///
    /// # Arguments
    ///
    /// * `base` - The base arguments to pass to each method (typically the resource ID)
    /// * `rid` - The interface definition
    /// * `rs` - The unique ID of the implementation
    /// * `instance` - The WebAssembly instance containing the implementation
    /// * `store` - A mutable store context
    ///
    /// # Returns
    ///
    /// A new `Wrapped` instance ready for method dispatch.
    pub fn new(
        base: Vec<wasm_runtime_layer::Value>,
        rid: Arc<Interface>,
        rs: String,
        instance: ::wasm_runtime_layer::Instance,
        store: ::wasm_runtime_layer::StoreContextMut<'_, U, E>,
    ) -> Self {
        // impl<U: 'static, E: WasmEngine> Copy for X<U, E> {}
        // let ctx: X<U, E> = X {
        //     base: Arc::new(Mutex::new(unsafe { std::mem::transmute(store) })),
        // };
        let rid2 = rid.clone();
        let instance2 = instance.clone();
        // let ctx2 = ctx.clone();
        let rs2 = rs.clone();
        let base2 = base.clone();
        return Self {
            rid: rid.clone(),
            all: once(Arc::new(
                move |mut ctx: StoreContextMut<'_, U, E>,
                      vals: Vec<Value>|
                      -> anyhow::Result<Vec<Value>> {
                    // let _ = &ctx2;
                    // let mut b = ctx2.base.lock().unwrap();
                    let vals: Vec<_> = base.clone().into_iter().chain(vals.into_iter()).collect();
                    let mut rets = vec![];
                    let f = instance2.get_export(
                        ctx.as_context_mut(),
                        &format!("pit/{}/~{rs2}.drop", rid2.rid_str()),
                    );
                    let Some(Extern::Func(f)) = f else {
                        anyhow::bail!("invalid func")
                    };
                    f.call(ctx.as_context_mut(), &vals, &mut rets)?;
                    Ok(rets)
                },
            )
                as Arc<
                    dyn Fn(StoreContextMut<'_, U, E>, Vec<Value>) -> anyhow::Result<Vec<Value>>
                        + Send
                        + Sync,
                >)
            .chain(rid.methods.iter().map(|(a, b)| {
                let rid = rid.clone();
                let a = a.clone();
                let instance = instance.clone();
                let b = Arc::new(b.clone());
                // let ctx = ctx.clone();
                let rs = rs.clone();
                let base = base2.clone();
                (Arc::new(
                    move |mut ctx: StoreContextMut<'_, U, E>,
                          vals: Vec<Value>|
                          -> anyhow::Result<Vec<Value>> {
                        // let _ = &ctx;
                        // let mut bi = ctx.base.lock().unwrap();
                        let vals: Vec<_> =
                            base.clone().into_iter().chain(vals.into_iter()).collect();
                        let mut rets = vec![Value::I32(0); b.rets.len()];
                        let f = instance.get_export(
                            ctx.as_context_mut(),
                            &format!("pit/{}/~{rs}/{a}", rid.rid_str()),
                        );
                        let Some(Extern::Func(f)) = f else {
                            anyhow::bail!("invalid func")
                        };
                        f.call(ctx.as_context_mut(), &vals, &mut rets)?;
                        Ok(rets)
                    },
                )
                    as Arc<
                        dyn Fn(StoreContextMut<'_, U, E>, Vec<Value>) -> anyhow::Result<Vec<Value>>
                            + Send
                            + Sync,
                    >)
            }))
            .collect(),
        };
    }
}
// impl Drop for Wrapped {
//     fn drop(&mut self) {
//         self.all[0](vec![]).unwrap();
//     }
// }

/// Type alias for an Arc-wrapped resource.
pub type RWrapped<U, E> = ::alloc::sync::Arc<Wrapped<U, E>>;

#[doc(hidden)]
pub extern crate anyhow;
#[doc(hidden)]
pub extern crate wasm_runtime_layer;

/// A wrapper that combines a resource with its store reference.
///
/// This is used for bridging between the host's trait object representation
/// and the WebAssembly runtime.
///
/// # Type Parameters
///
/// * `X` - The wrapped value type
/// * `U` - The user state type for the store
/// * `E` - The WebAssembly engine backend
pub struct W<X, U: 'static, E: WasmEngine> {
    /// The wrapped value.
    pub r: X,
    /// Reference to the store cell for accessing the WebAssembly store.
    pub store: Arc<StoreCell<U, E>>,
}
impl<U: 'static, E: WasmEngine, X: Clone> Clone for W<X, U, E> {
    fn clone(&self) -> Self {
        Self {
            r: self.r.clone(),
            store: self.store.clone(),
        }
    }
}
/// An unsafe cell wrapper for storing a WebAssembly store.
///
/// This is needed because the store cannot be shared between threads normally,
/// but PIT needs to access it from multiple contexts.
///
/// # Safety
///
/// Users must ensure that the store is only accessed from one thread at a time.
pub struct StoreCell<U, E: WasmEngine> {
    /// The wrapped store in an unsafe cell.
    pub wrapped: UnsafeCell<Store<U, E>>,
}
unsafe impl<
        U: Send,
        E: Send + wasm_runtime_layer::backend::WasmEngine + wasm_runtime_layer::backend::WasmEngine,
    > Send for StoreCell<U, E>
{
}
unsafe impl<
        U: Sync,
        E: Sync + wasm_runtime_layer::backend::WasmEngine + wasm_runtime_layer::backend::WasmEngine,
    > Sync for StoreCell<U, E>
{
}
impl<U, E: WasmEngine> StoreCell<U, E> {
    pub unsafe fn get(&self) -> StoreContextMut<'_, U, E> {
        unsafe { &mut *self.wrapped.get() }.as_context_mut()
    }
}
