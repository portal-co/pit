//! # PIT Rust Host Core
//!
//! Core code generation for PIT host bindings.
//!
//! This crate provides the core functionality for generating Rust code that can
//! host PIT-enabled WebAssembly modules. It generates trait definitions and
//! implementations that bridge between Rust code and WebAssembly resources.
//!
//! ## Generated Code
//!
//! For each PIT interface, this crate generates:
//! - A trait definition with all interface methods
//! - Method dispatch implementations for wrapped resources
//! - Value conversion between Rust types and WebAssembly values
//!
//! ## Usage
//!
//! ```ignore
//! use pit_rust_host_core::{render, Opts};
//! use quote::quote;
//!
//! let opts = Opts::default();
//! let code = render(&quote! { crate }, &interface, &opts);
//! ```

use pit_core::{Arg, Interface, ResTy, Sig};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use std::iter::once;
use syn::{spanned::Spanned, Ident, Index};

/// Configuration options for host code generation.
#[derive(Default)]
#[non_exhaustive]
pub struct Opts {
    // pub guest: Option<pit_rust_guest::Opts>,
}

/// Renders a PIT interface as Rust host binding code.
///
/// # Arguments
///
/// * `root` - The crate path prefix for runtime types
/// * `i` - The PIT interface to render
/// * `opts` - Code generation options
///
/// # Returns
///
/// A `TokenStream` containing the generated Rust code.
pub fn render(root: &TokenStream, i: &Interface, opts: &Opts) -> TokenStream {
    let id = format_ident!("B{}", i.rid_str());
    // let internal = format_ident!("{id}_utils");
    let methods = i.methods.iter().map(|(a, b)| {
        let method_name = format_ident!("{a}");
        let sig = render_sig(root, b, &quote! {&self}, quote! {
            ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>
        });
        quote! {
            fn #method_name #sig
        }
    });
    let impls = i.methods.iter().enumerate().map(|(c, (a, b))| {
        let init = b
            .params
            .iter()
            .enumerate()
            .map(|(pi, a)| {
                let param = format_ident!("p{pi}");
                render_new_val(root, a, quote! {#param})
            });
        let init =
            once(quote! {#root::wasm_runtime_layer::Value::I32(self.base as #root::core::primitive::i32)}).chain(init);
        let fini = b.rets.iter().enumerate().map(|(ri, r)| {
            render_base_val(root, r, quote! {&rets[#ri]})
        });
        let method_name = format_ident!("{a}");
        let sig = render_sig(root, b, &quote! {&self}, quote! {
            ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>
        });
        let c1 = c + 1;
        quote! {
            fn #method_name #sig {
                let a = #root::core::clone::Clone::clone(&self.all[#c1]);
                let args = #root::alloc::vec![#(#init),*];
                let mut rets = a(ctx,args);
                return Ok((#(#fini),*))
            }
        }
    });
    let injects = i.methods.iter().map(|(a,b)|{
        let init = b.params.iter().enumerate().map(|(pi,a)| render_base_val(root, a, quote! {&args[#pi + 1]}));
        let fini = b.rets.iter().enumerate().map(|(ri,r)|{
            let idx = Index{index: ri as u32, span: root.span()};
            render_new_val(root, r, quote! {r . #idx})
        });
        let method_name = format_ident!("{a}");
        quote!{
            let r = a.clone();
            #root::alloc::sync::Arc::new(ctx,move|ctx,args|{
                let r = r.#method_name(ctx,#(#init),*)
                Ok(#root::alloc::vec![#(#fini),*])
            })
        }
    });
    // let p = match opts.guest.as_ref() {
    //     None => quote! {},
    //     Some(g) => proxy(root, i, opts, g),
    // };
    let i_str = i.to_string();
    let all_items = {
        let finalize_item = quote! {
            let r = #root::core::clone::Clone::clone(&a);
            unsafe{
                #root::alloc::sync::Arc::new(move|ctx,args|{r.finalize(ctx);Ok(vec![])})
            }
        };
        let all = once(finalize_item).chain(injects);
        quote! {
            #(#all),*
        }
    };
    quote! {
        pub trait #id<U: 'static,E: #root::wasm_runtime_layer::backend::WasmEngine>{
            #(#methods)*
            unsafe fn finalize(&self, ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>) -> #root::anyhow::Result<()>;
        }
        const _: () = {
            impl<U: 'static,E: #root::wasm_runtime_layer::backend::WasmEngine> #id<U,E> for #root::RWrapped<U,E>{
                #(#impls)*
                unsafe fn finalize(&self, ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>) -> #root::anyhow::Result<()>{
                    self.all[0](ctx,#root::alloc::vec![])?;
                    Ok(())
                }
            }
            impl<U: 'static,E: #root::wasm_runtime_layer::backend::WasmEngine> From<#root::alloc::sync::Arc<dyn $id<U,E>> for #root::RWrapped<U,E>{
                fn from(a: #root::alloc::sync::Arc<dyn $id<U,E>>) -> Self{
                    Self{
                        rid: #root::alloc::sync::Arc::new(#root::pit_core::Interface::parse_interface(#i_str).ok().unwrap()),
                        all: #root::alloc::vec![#all_items]
                    }
                }
            }
        }
    }
}
/// Renders a method signature as Rust code for host bindings.
///
/// # Arguments
///
/// * `root` - The crate path prefix
/// * `s` - The method signature
/// * `self_` - The self parameter
/// * `s2` - Additional context parameter (typically the store context)
///
/// # Returns
///
/// A `TokenStream` containing the function signature.
pub fn render_sig(
    root: &TokenStream,
    s: &Sig,
    self_: &TokenStream,
    s2: TokenStream,
) -> TokenStream {
    let params = s
        .params
        .iter()
        .map(|a| render_ty(root, a))
        .enumerate()
        .map(|(a, b)| {
            let name = format_ident!("p{a}");
            quote!(#name : #b)
        });
    let params = once(self_).cloned().chain(once(s2)).chain(params);
    let rets = s.rets.iter().map(|a| render_ty(root, a));
    quote! {
        (#(#params),*) -> #root::anyhow::Result<(#(#rets),*)>
    }
}
/// Renders a PIT argument type as a WebAssembly value type expression.
///
/// # Arguments
///
/// * `root` - The crate path prefix
/// * `p` - The argument type
///
/// # Returns
///
/// A `TokenStream` containing the `ValueType` variant.
pub fn render_blit(root: &TokenStream, p: &Arg) -> TokenStream {
    match p {
        Arg::I32 => quote! {
            #root::wasm_runtime_layer::ValueType::I32
        },
        Arg::I64 => quote! {
            #root::wasm_runtime_layer::ValueType::I64
        },
        Arg::F32 => quote! {
            #root::wasm_runtime_layer::ValueType::F32
        },
        Arg::F64 => quote! {
            #root::wasm_runtime_layer::ValueType::F64
        },
        Arg::Resource { .. } => quote! {
            #root::wasm_runtime_layer::ValueType::ExternRef
        },
        _ => todo!(),
    }
}
/// Renders a method signature as a WebAssembly `FuncType` expression.
///
/// # Arguments
///
/// * `root` - The crate path prefix
/// * `s` - The method signature
///
/// # Returns
///
/// A `TokenStream` containing the `FuncType::new` call.
pub fn render_blit_sig(root: &TokenStream, s: &Sig) -> TokenStream {
    let params = {
        let p = s.params.iter().map(|p| render_blit(root, p));
        let p = once(quote! { #root::wasm_runtime_layer::ValueType::ExternRef }).chain(p);
        quote! { #(#p),* }
    };
    let rets = {
        let p = s.rets.iter().map(|p| render_blit(root, p));
        quote! { #(#p),* }
    };
    quote! {
        #root::wasm_runtime_layer::FuncType::new([#params].into_iter(),[#rets].into_iter())
    }
}
/// Renders a PIT argument type as a Rust type for host bindings.
///
/// # Arguments
///
/// * `root` - The crate path prefix
/// * `p` - The argument type
///
/// # Returns
///
/// A `TokenStream` containing the Rust type.
pub fn render_ty(root: &TokenStream, p: &Arg) -> TokenStream {
    match p {
        Arg::I32 => quote! {
            #root::core::primitive::u32
        },
        Arg::I64 => quote! {
            #root::core::primitive::u64
        },
        Arg::F32 => quote! {
            #root::core::primitive::f32
        },
        Arg::F64 => quote! {
            #root::core::primitive::f64
        },
        Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => match ty {
            ResTy::None => quote! {
                #root::wasm_runtime_layer::ExternRef
            },
            _ => {
                let a = quote! {
                    #root::alloc::sync::Arc<#root::Wrapped<U,E>>
                };
                if *nullable {
                    quote! {#root::core::option::Option<#a>}
                } else {
                    a
                }
            }
        },
        _ => todo!(),
    }
}
/// Renders code to extract a value from a `wasm_runtime_layer::Value`.
///
/// # Arguments
///
/// * `root` - The crate path prefix
/// * `p` - The expected argument type
/// * `x` - The expression containing the `Value`
///
/// # Returns
///
/// A `TokenStream` containing the extraction code.
pub fn render_base_val(root: &TokenStream, p: &Arg, x: TokenStream) -> TokenStream {
    let v = match p {
        Arg::I32 => quote! {
            let #root::wasm_runtime_layer::Value::I32(t) = #x else{
                #root::anyhow::bail!("invalid param")
            }
        },
        Arg::I64 => quote! {
            let #root::wasm_runtime_layer::Value::I64(t) = #x else{
                #root::anyhow::bail!("invalid param")
            }
        },
        Arg::F32 => quote! {
            let #root::wasm_runtime_layer::Value::F32(t) = #x else{
                #root::anyhow::bail!("invalid param")
            }
        },
        Arg::F64 => quote! {
            let #root::wasm_runtime_layer::Value::F64(t) = #x else{
                #root::anyhow::bail!("invalid param")
            }
        },
        Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => {
            let mut a = quote! {
                let #root::wasm_runtime_layer::Value::ExternRef(t) = #x else{
                    #root::anyhow::bail!("invalid param")
                }
            };
            if !matches!(ty, ResTy::None) {
                quote!{
                    let t = match t.downcast::<'_,'_,                #root::alloc::sync::Arc<#root::Wrapped<U,E>>,U,E>(ctx){
                        #root::core::result::Result::Ok(t) => Arc::new(t.clone()),
                        #root::core::result::Result::Err(_) =>                     #root::anyhow::bail!("invalid param")
                        }
                }.to_tokens(&mut a);
            }
            if !*nullable {
                quote! {
                    let t = match t{
                        Some(a) => a,
                        None => #root::anyhow::bail!("invalid param")
                    }
                }
                .to_tokens(&mut a)
            }
            a
        }
        _ => todo!(),
    };
    quote! {
        {
            #v;t
        }
    }
}
/// Renders code to create a `wasm_runtime_layer::Value` from a Rust value.
///
/// # Arguments
///
/// * `root` - The crate path prefix
/// * `p` - The argument type
/// * `t` - The expression containing the Rust value
///
/// # Returns
///
/// A `TokenStream` containing the value creation code.
pub fn render_new_val(root: &TokenStream, p: &Arg, t: TokenStream) -> TokenStream {
    match p {
        Arg::I32 => quote! {
            #root::wasm_runtime_layer::Value::I32(#t)
        },
        Arg::I64 => quote! {
            #root::wasm_runtime_layer::Value::I64(#t)
        },
        Arg::F32 => quote! {
            #root::wasm_runtime_layer::Value::F32(#t)
        },
        Arg::F64 => quote! {
            #root::wasm_runtime_layer::Value::F64(#t)
        },
        Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => {
            let tq = |t: TokenStream| {
                let inner = match ty {
                    ResTy::None => quote! {
                        #root::wasm_runtime_layer::ExternRef::new(ctx,t)
                    },
                    _ => quote! {
                        match t.to_any().downcast_ref::<#root::alloc::sync::Arc<#root::Wrapped<U,E>>>().cloned(){
                            #root::core::option::Option::None =>                     #root::wasm_runtime_layer::ExternRef::new(ctx,t),
                            #root::core::option::Option::Some(t) => #root::wasm_runtime_layer::ExternRef::new(ctx,t),
                        }
                    },
                };
                quote! {
                    {
                        let t = #t;
                        #inner
                    }
                }
            };
            if !*nullable {
                let inner = match ty {
                    ResTy::None => t,
                    _ => tq(t),
                };
                quote! {
                    #root::wasm_runtime_layer::Value::ExternRef(Some(#inner))
                }
            } else {
                let inner = match ty {
                    ResTy::None => t,
                    _ => {
                        let tq_t = tq(quote! {t});
                        quote! {#t.map(|t|#tq_t)}
                    }
                };
                quote! {
                    #root::wasm_runtime_layer::Value::ExternRef(#inner)
                }
            }
        }
        _ => todo!(),
    }
}
