use std::iter::once;

use pit_core::{Arg, Interface, ResTy, Sig};
use proc_macro2::{Span, TokenStream};
use quasiquote::quasiquote;
use quote::{format_ident, quote, ToTokens};
use syn::{spanned::Spanned, Ident, Index};
pub struct Opts {
    pub guest: Option<pit_rust_guest::Opts>,
}
pub fn render(root: &TokenStream, i: &Interface, opts: &Opts) -> TokenStream {
    let id = format_ident!("B{}", i.rid_str());
    // let internal = format_ident!("{id}_utils");
    let methods = i.methods.iter().map(|(a, b)| {
        quasiquote! {
            fn #{format_ident!("{a}")}#{render_sig(root,b,&quote! {&self},quote!{
                ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>
            })}
        }
    });
    let impls = i.methods.iter().enumerate().map(|(c, (a, b))| {
        let init = b
            .params
            .iter()
            .enumerate()
            .map(|(pi, a)| render_new_val(root, a, quasiquote! {#{format_ident!("p{pi}")}}));
        let init =
            once(quote! {#root::wasm_runtime_layer::Value::I32(self.base as i32)}).chain(init);
        let fini = b.rets.iter().enumerate().map(|(ri, r)| {
            quasiquote! {
                #{render_base_val(root, r, quote! {&rets[#ri]})}
            }
        });
        quasiquote! {
            fn #{format_ident!("{a}")}#{render_sig(root,b,&quote! {&self},quote!{
                ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>
            })}{
                let a = self.all[#{c+1}].clone();
                let args = vec![#(#init),*];
                let mut rets = a(ctx,args);
                return Ok((#(#fini),*))
            }
        }
    });
    let injects = i.methods.iter().map(|(a,b)|{
        let init = b.params.iter().enumerate().map(|(pi,a)|quasiquote!{#{render_base_val(root, a, quote! {&args[#pi + 1]})}});
        let fini = b.rets.iter().enumerate().map(|(ri,r)|{
            quasiquote!{
                #{render_new_val(root, r, quasiquote! {r . #{Index{index: ri as u32,span: root.span()}}})}
            }
        });
        quasiquote!{
            let r = a.clone();
            Arc::new(ctx,move|ctx,args|{
                let r = r.#{format_ident!("{a}")}(ctx,#(#init),*)
                Ok(vec![#(#fini),*])
            })
    }});
    quasiquote! {
        pub trait #id<U: 'static,E: #root::wasm_runtime_layer::backend::WasmEngine>{
            #(#methods)*
            unsafe fn finalize(&self, ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>) -> #root::anyhow::Result<()>;
        }
        const _: () = {
            impl<U: 'static,E: #root::wasm_runtime_layer::backend::WasmEngine> #id<U,E> for #root::RWrapped<U,E>{
                #(#impls)*
                unsafe fn finalize(&self, ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>) -> #root::anyhow::Result<()>{
                    self.all[0](ctx,vec![])?;
                    Ok(())
                }
            }
            impl<U: 'static,E: #root::wasm_runtime_layer::backend::WasmEngine> From<Arc<dyn $id<U,E>> for #root::RWrapped<U,E>{
                fn from(a: Arc<dyn $id<U,E>>) -> Self{
                    Self{
                        rid: Arc::new(#root::pit_core::Interface::parse_interface(#{i.to_string()}).ok().unwrap()),
                        all: vec![#{
                            let all = once(quasiquote!{
                                let r = a.clone();
                                unsafe{
                                    Arc::new(move|ctx,args|{r.finalize(ctx);Ok(vec![])})
                                }
                            }).chain(injects);

                            quote!{
                                #(#all),*
                            }
                        }]
                    }
                }
            }
            #{match opts.guest.as_ref(){
                None=>quote!{},
                Some(g) => proxy(root,i,opts,g), 
            }}
        }
    }
}
pub fn proxy(root: &TokenStream, i: &Interface, opts: &Opts, g: &pit_rust_guest::Opts) -> TokenStream{
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
                            c = quote!{Box::new(#root::W{
                                r: #root::RWrapped<U,E>::from(Arc::new(#c)),
                                store: self.store.clone()
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
                                #root::RWrapped<U,E>::from(Compat(::std::cell::UnsafeCell::new(Some(#c)))) 
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
                        #root::RWrapped<U,E>::from(Compat(::std::cell::UnsafeCell::new(Some(#c)))) 
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
            c = quote!{Box::new(#root::W{
                r: #root::RWrapped<U,E>::from(Arc::new(#c)),
                store: Arc::new(#root::StoreCell{
                    wrapped: ::std::cell::UnsafeCell::new(#root::wasm_runtime_layer::Store::new(ctx.engine(),ctx.data().clone()))
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
    quasiquote!{
        struct Compat<T>(::std::cell::UnsafeCell<Option<T>>);
        impl<U: 'static + Clone,E: #root::wasm_runtime_layer::backend::WasmEngine> #pid for #root::W<#root::RWrapped<U,E>,U,E>{
            #(#impl_guest)*
        }
        impl<U: 'static + Clone,E: #root::wasm_runtime_layer::backend::WasmEngine, X: #pid> #id<U,E> for Compat<X>{
            #(#impl_host)*
            unsafe fn finalize(&self, ctx: #root::wasm_runtime_layer::StoreContextMut<'_,U,E>) -> #root::anyhow::Result<()>{
                let x = unsafe{
                    &mut *(self.0.get())
                }.take();
                let Some(x) = x else{
                    return Err(#root::anyhow::anyhow!("double finalized"))
                }
                Ok(())
            }
        }
        impl From<Box<dyn #pid>> for Arc<dyn #id>{
            fn from(p: Box<dyn #pid>) -> Self{
                return Arc::new(Compat(::std::cell::UnsafeCell::new(Some(#res::from(p)))))
            }
        }
    }
}
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
        .map(|(a, b)| quasiquote!(#{format_ident!("p{a}")} : #b));
    let params = once(self_).cloned().chain(once(s2)).chain(params);
    let rets = s.rets.iter().map(|a| render_ty(root, a));
    quote! {
        (#(#params),*) -> #root::anyhow::Result<(#(#rets),*)>
    }
}
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
        _ => todo!()
    }
}
pub fn render_blit_sig(root: &TokenStream, s: &Sig) -> TokenStream {
    quasiquote! {
        #root::wasm_runtime_layer::FuncType::new([#{
            let p = s.params.iter().map(|p|render_blit(root, p));
            let p = once(quote! {#root::wasm_runtime_layer::ValueType::ExternRef}).chain(p);
            quote! {
                #(#p),*
            }
        }].into_iter(),[#{            let p = s.rets.iter().map(|p|render_blit(root, p));
            quote! {
                #(#p),*
            }}].into_iter())
    }
}
pub fn render_ty(root: &TokenStream, p: &Arg) -> TokenStream {
    match p {
        Arg::I32 => quote! {
            u32
        },
        Arg::I64 => quote! {
            u64
        },
        Arg::F32 => quote! {
            f32
        },
        Arg::F64 => quote! {
            f64
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
                let a = quasiquote! {
                    ::std::sync::Arc<#root::Wrapped<U,E>>
                };
                if *nullable {
                    quote! {Option<#a>}
                } else {
                    a
                }
            }
        },
        _ => todo!()
    }
}
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
                quasiquote!{
                    let t = match t.downcast::<'_,'_,                ::std::sync::Arc<#root::Wrapped<U,E>>,U,E>(ctx){
                            Ok(t) => Arc::new(t.clone()),
                            Err(_) =>                     #root::anyhow::bail!("invalid param")
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
        },
        _ => todo!()
    };
    quote! {
        {
            #v;t
        }
    }
}
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
                quasiquote! {
                    {
                        let t = #t;
                        #{match ty{
                            ResTy::None => quote! {                    #root::wasm_runtime_layer::ExternRef::new(ctx,t)},
                            _ => quote! {
                                match t.to_any().downcast_ref::<::std::sync::Arc<#root::Wrapped<U,E>>>(){
                                    None =>                     #root::wasm_runtime_layer::ExternRef::new(ctx,t),
                                    Some(t) => #root::wasm_runtime_layer::ExternRef::new(ctx,t.clone()),
                                }
                            }
                        }}
                    }
                }
            };
            if !*nullable {
                quasiquote! {
                    #root::wasm_runtime_layer::Value::ExternRef(Some(#{match ty{
                        ResTy::None => t,
                        _ => tq(t)
                    }}))
                }
            } else {
                quasiquote! {
                    #root::wasm_runtime_layer::Value::ExternRef(#{match ty{
                        ResTy::None => t,
                        _ => quasiquote! {#t.map(|t|#{tq(quote! {t})})}
                    }})
                }
            }
        },
        _ => todo!()
    }
}
