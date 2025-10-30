use pit_core::{Arg, Interface, ResTy, Sig};
pub use pit_rust_host_core::*;
use proc_macro2::{Span, TokenStream};
use quasiquote::quasiquote;
use quote::{format_ident, quote, ToTokens};
use std::iter::once;
use syn::{spanned::Spanned, Ident, Index};
#[derive(Default)]
#[non_exhaustive]
pub struct Opts {
    pub guest: Option<pit_rust_guest::Opts>,
    pub core: pit_rust_host_core::Opts,
}
pub fn render(root: &TokenStream, i: &Interface, opts: &Opts) -> TokenStream {
    let p = match opts.guest.as_ref() {
        None => quote! {},
        Some(g) => proxy(root, i, opts, g),
    };
    quasiquote! {
        #{pit_rust_host_core::render(root, i, &opts.core)}
        #{p}
    }
}
pub fn proxy(
    root: &TokenStream,
    i: &Interface,
    opts: &Opts,
    g: &pit_rust_guest::Opts,
) -> TokenStream {
    let id = format_ident!("B{}", i.rid_str());
    let pid = format_ident!("R{}", i.rid_str());
    let root2 = &g.root;
    let res = if g.tpit {
        quote! {
            #root2::tpit_rt::Tpit
        }
    } else {
        quote! {
            #root2::externref::Resource
        }
    };
    let impl_guest = i.methods.iter().map(|(a, b)| {
        quasiquote! {
            fn #{format_ident!("{a}")}#{pit_rust_guest::render_sig(g,root,i,b,&quote! {&mut self},false)}{
                let ctx = unsafe{
                    self.get()
                };
                let r = self.r.#{format_ident!("{a}")}(ctx,#{
                    let params = b.params.iter().enumerate().map(|(a,b)|{
                        let mut c = format_ident!("p{a}");
                        let mut c = quote!{
                            #c
                        };
                        if let Arg::Resource { ty, nullable, take, ann } = b{
                            c = quote!{#root::alloc::boxed::Box::new(#root::W{
                                r: #root::RWrapped<U,E>::from(Arc::new(#c)),
                                store: #root::core::clone::clone(&self.store)
                            }).into()};
                            if !take{
                                c = quote!{&mut #c};
                            };
                        }
                        c
                });
                    quote! {
                        #(#params),*
                    }
                }).unwrap();
                return (#{
                    let xs = b.rets.iter().enumerate().map(|(a,b)|{
                        let mut c = Index{
                            index: a as u32,
                            span: Span::call_site()
                        };
                        let mut c = quote!{
                            r.#c
                        };
                        if let Arg::Resource { ty, nullable, take, ann } = b{
                            c = quote!{
                                #root::RWrapped<U,E>::from(Compat(#root::core::cell::UnsafeCell::new(#root::core::option::Option::Some(#c))))
                            }
                        }
                        c
                    });
                    quote!{
                        #(#xs),*
                    }
                })
            };
        }
    });
    let impl_host = i.methods.iter().enumerate().map(|(c, (a, b))| {
        let params = b.params.iter().enumerate().map(|(a,b)|{
            let mut c = format_ident!("p{a}");
            let mut c = quote!{
                #c
            };
            if let Arg::Resource { ty, nullable, take, ann } = b{
                    c = quote!{
                        #root::RWrapped<U,E>::from(Compat(#root::core::cell::UnsafeCell::new(#root::::core::option::Option::Some(#c))))
                    };
                if !take{
                    c = quote!{&mut #c};
                };
            }
            c
    });
    let rets = b.rets.iter().enumerate().map(|(a,b)|{
        let mut c = Index{
            index: a as u32,
            span: Span::call_site()
        };
        let mut c = quote!{
            r.#c
        };
        if let Arg::Resource { ty, nullable, take, ann } = b{
            c = quote!{#root::alloc::boxed::Box::new(#root::W{
                r: #root::RWrapped<U,E>::from(Arc::new(#c)),
                store: #root::alloc::sync::Arc::new(#root::StoreCell{
                    wrapped: #root::core::cell::UnsafeCell::new(#root::wasm_runtime_layer::Store::new(ctx.engine(),match ctx.data(){
                        d => #root::core::clone::Clone::clone(d),
                    }))
                })
            }).into()};
        }
        c
    });
        quasiquote! {
            fn #{format_ident!("{a}")}#{render_sig(root,b,&quote! {&self},quote!{
                ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>
            })}{
                let x = unsafe{
                    &mut *(self.0.get())
                }.as_mut().unwrap().#{format_ident!("{a}")}(#(#params),*);
                Ok((#(#rets),*))
            }
        }
    });
    quasiquote! {
        struct Compat<T>(#root::core::cell::UnsafeCell<Option<T>>);
        impl<U: 'static + #root::core::clone::Clone,E: #root::wasm_runtime_layer::backend::WasmEngine> #pid for #root::W<#root::RWrapped<U,E>,U,E>{
            #(#impl_guest)*
        }
        impl<U: 'static + #root::core::clone::Clone,E: #root::wasm_runtime_layer::backend::WasmEngine, X: #pid> #id<U,E> for Compat<X>{
            #(#impl_host)*
            unsafe fn finalize(&self, ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>) -> #root::anyhow::Result<()>{
                let x = unsafe{
                    &mut *(self.0.get())
                }.take();
                let #root::core::option::Option::Some(x) = x else{
                    return #root::core::result::Result::Err(#root::anyhow::anyhow!("double finalized"))
                }
                #root::core::result::Result::Ok(())
            }
        }
        impl From<Box<dyn #pid>> for Arc<dyn #id>{
            fn from(p: Box<dyn #pid>) -> Self{
                return Arc::new(Compat(#root::core::cell::UnsafeCell::new(Some(#res::from(p)))))
            }
        }
    }
}
