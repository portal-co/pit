#![no_std]
#[doc(hidden)]
pub extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::format;
use alloc::vec::Vec;
use core::{
    cell::UnsafeCell,
    iter::{empty, once},
};

use pit_core::{Arg, Interface};
use wasm_runtime_layer::{
    backend::WasmEngine, AsContext, AsContextMut, Extern, ExternRef, Func, FuncType, Imports,
    Instance, Module, Store, StoreContext, StoreContextMut, Value, ValueType,
};
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
pub struct Wrapped<U: 'static, E: wasm_runtime_layer::backend::WasmEngine> {
    pub rid: Arc<Interface>,
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
pub type RWrapped<U, E> = ::alloc::sync::Arc<Wrapped<U, E>>;
pub extern crate anyhow;
pub extern crate wasm_runtime_layer;

pub struct W<X, U: 'static, E: WasmEngine> {
    pub r: X,
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
pub struct StoreCell<U, E: WasmEngine> {
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
