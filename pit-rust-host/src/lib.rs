use std::iter::once;

use pit_core::{Arg, Interface, Sig};
use proc_macro2::TokenStream;
use quasiquote::quasiquote;
use quote::{format_ident, quote, ToTokens};
use syn::{spanned::Spanned, Ident, Index};
pub fn render(root: &TokenStream, i: &Interface) -> TokenStream {
    let id = format_ident!("R{}", i.rid_str());
    let internal = format_ident!("{id}_utils");
    let methods = i.methods.iter().map(|(a, b)| {
        quasiquote! {
            fn #{format_ident!("{a}")}#{render_sig(root,b,&quote! {&self})}
        }
    });
    let impls = i.methods.iter().map(|(a, b)| {
        let init = b.params.iter().enumerate().map(|(pi,a)|render_new_val(root, a, quasiquote!{#{format_ident!("p{pi}")}}));
        let init = once(quote!{#root::wasm_runtime_layer::Value::I32(self.base as i32)}).chain(init);
        let fini = b.rets.iter().enumerate().map(|(ri,r)|{
            quasiquote!{
                #{render_base_val(root, r, quote! {&rets[#ri]})}
            }
        });
        quasiquote! {
            fn #{format_ident!("{a}")}#{render_sig(root,b,&quote! {&self})}{
                let Some(#root::wasm_runtime_layer::Export::Func(f)) = self.instance.get_export(unsafe{
                    &*::std::cell::UnsafeCell::raw_get(self.ctx)
                },#{format_ident!("pit/{}/{}",i.rid_str(),a)}) else{
                    panic!("invalid func")
                };
                let args = vec![#(#init),*];
                let mut rets = vec![#root::wasm_runtime_layer::Value::I32(0);#{b.rets.len()}];
                f.call(unsafe{
                    &mut *::std::cell::UnsafeCell::raw_get(self.ctx)
                },&args,&mut rets)?;
                return Ok((#(#fini),*))
            }
        }
    });
    let injects = i.methods.iter().map(|(a,b)|{
        let init = b.params.iter().enumerate().map(|(pi,a)|quasiquote!{let #{format_ident!("p{pi}")} = #{render_base_val(root, a, quote! {&args[#pi + 1]})}});
        let fini = b.rets.iter().enumerate().map(|(ri,r)|{
            quasiquote!{
                rets[#ri] = #{render_new_val(root, r, quasiquote! {r . #{Index{index: ri as u32,span: root.span()}}})}
            }
        });
        quasiquote!{
            let f = #root::wasm_runtime_layer::Func::new(ctx,#{render_blit_sig(root, b)},|ctx,args,rets|{
                #(#init),*
                let #root::wasm_runtime_layer::Value::ExternRef(t) = &args[0] else{
                    #root::anyhow::bail!("invalid param")
                }
                let r = match t.downcast::<'_,'_,                ::std::sync::Arc<dyn #id<U,E>>,U,E>(ctx){
                    Ok(t) => {
                        t.#{format_ident!("{a}")}(#{
                            let p = b.params.iter().enumerate().map(|(a,_)|format_ident!("p{a}"));
                            quote! {
                                #(#p .clone()),*
                            }
                        })?
                    },
                    Err(_) => match t.downcast::<'_,'_, ::std::sync::Arc<#root::Wrapped<U,E>>,U,E>(ctx)?{
                        Ok(t) => {
                            t.#{format_ident!("{a}")}(#{
                                let p = b.params.iter().enumerate().map(|(a,_)|format_ident!("p{a}"));
                                quote! {
                                    #(#p .clone()),*
                                }
                            })?
                        },
                        Err(_) => #root::anyhow::bail!("invalid externref")
                    }
                };
                #(#fini),*
                Ok(())
            });
            i.define(#{format!("pit/{}",i.rid_str())},#a,f);
    }}).chain(once( quasiquote!{
        let f = #root::wasm_runtime_layer::Func::new(ctx,#root::wasm_runtime_layer::FuncType::new([#root::wasm_runtime_layer::ValueType::I32],[#root::wasm_runtime_layer::ExternType]),|ctx,args,rets|{
            let #root::wasm_runtime_layer::Value::I32(i) = &args[0] else{
                unreachable!()
            };
            let n = ctx.data().as_ref().clone();
            let v = #root::Wrapped::new(*i as u32,#{i.rid_str()},ctx.as_context_mut(),n);
            // let v: ::std::sync::Arc<dyn #id<U,E>> = ::std::sync::Arc::new(v);
            rets[0] = #root::wasm_runtime_layer::Value::ExternRef(#root::wasm_runtime_layer::ExternRef::new(ctx,v));
            Ok(())
        });
        i.define(#{format!("pit/{}",i.rid_str())},"~push",f);
    }));
    quasiquote! {
        pub trait #id<U: 'static,E: #root::wasm_runtime_layer::backend::WasmEngine>{
            fn to_any(&self) -> &dyn ::std::any::Any;
            #(#methods)*
        }
        mod #internal{
            use super::#id;
            impl<U: 'static,E: #root::wasm_runtime_layer::backend::WasmEngine> #id<U,E> for #root::RWrapped<U,E>{
                fn to_any(&self) -> &dyn ::std::any::Any{
                    return self;
                }
                #(#impls)*
            }

            fn inject<U: 'static + AsRef<#root::wasm_runtime_layer::Instance>,E: #root::wasm_runtime_layer::backend::WasmEngine>(ctx: impl #root::wasm_runtime_layer::AsContextMut<UserState = U,Engine = E>, i: &mut #root::wasm_runtime_layer::Imports){
            #(#injects)*
            }
        }
    }
}
pub fn render_sig(root: &TokenStream, s: &Sig, self_: &TokenStream) -> TokenStream {
    let params = s
        .params
        .iter()
        .map(|a| render_ty(root, a))
        .enumerate()
        .map(|(a, b)| quasiquote!(#{format_ident!("p{a}")} : #b));
    let params = once(self_).cloned().chain(params);
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
        Arg::Resource { ty, nullable } => match ty {
            None => quote! {
                #root::wasm_runtime_layer::ExternRef
            },
            Some(a) => {
                let a = quasiquote! {
                    ::std::sync::Arc<dyn #{format_ident!("R{}",hex::encode(a))}<U,E>>
                };
                if *nullable {
                    quote! {Option<#a>}
                } else {
                    a
                }
            }
        },
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
        Arg::Resource { ty, nullable } => {
            let mut a = quote! {
                let #root::wasm_runtime_layer::Value::ExternRef(t) = #x else{
                    #root::anyhow::bail!("invalid param")
                }
            };
            if let Some(r) = ty.as_ref() {
                quasiquote!{
                    let t = match t{None => None,Some(t) => Some(match t.downcast::<'_,'_,                ::std::sync::Arc<dyn #{format_ident!("R{}",hex::encode(r))}<U,E>>,U,E>(ctx){
                        Ok(t) => t.clone(),
                        Err(_) => match t.downcast::<'_,'_,                ::std::sync::Arc<#root::Wrapped<U,E>>,U,E>(ctx){
                            Ok(t) => Arc::new(t.clone()),
                            Err(_) =>                     #root::anyhow::bail!("invalid param")
                        }
                    })}
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
        Arg::Resource { ty, nullable } => {
            let tq = |t:TokenStream|quasiquote! {
                {
                    let t = #t;
                    #{match ty{
                        None => quote! {                    #root::wasm_runtime_layer::ExternRef::new(ctx,t)},
                        Some(_) => quote! {
                            match t.to_any().downcast_ref::<::std::sync::Arc<#root::Wrapped<U,E>>>(){
                                None =>                     #root::wasm_runtime_layer::ExternRef::new(ctx,t),
                                Some(t) => #root::wasm_runtime_layer::ExternRef::new(ctx,t.clone()),
                            }
                        }
                    }}
                }
            };
            if !*nullable {
                quasiquote! {
                    #root::wasm_runtime_layer::Value::ExternRef(Some(#{match ty{
                        None => t,
                        Some(_) => tq(t)
                    }}))
                }
            } else {
                quasiquote! {
                    #root::wasm_runtime_layer::Value::ExternRef(#{match ty{
                        None => t,
                        Some(_) => quasiquote! {#t.map(|t|#{tq(quote! {t})})}
                    }})
                }
            }
        }
    }
}
