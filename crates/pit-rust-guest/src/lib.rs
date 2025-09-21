use std::iter::once;

use pit_core::{Arg, Interface, ResTy, Sig};
use proc_macro2::TokenStream;
use quasiquote::quasiquote;
use quote::{format_ident, quote};
use sha3::Digest;
use std::io::Write;
pub struct Opts {
    pub root: TokenStream,
    pub salt: Vec<u8>,
    pub tpit: bool,
}
pub fn render(opts: &Opts, i: &Interface) -> TokenStream {
    let root = &opts.root;
    let id = i.rid_str();
    let mut ha = sha3::Sha3_256::default();
    write!(ha, "~{}", id);
    ha.write(&opts.salt);
    let ha = hex::encode(ha.finalize());
    let id2 = format_ident!("R{}", i.rid_str());
    let methods = i.methods.iter().map(|(a, b)| {
        quasiquote! {
            fn #{format_ident!("{a}")}#{render_sig(opts,root,i,b,&quote! {&mut self},false)};
        }
    });
    let xref = if opts.tpit {
        quote! {}
    } else {
        quasiquote!(
            #[#root::externref::externref(crate = #{quote! {#root::externref}.to_string()})]
        )
    };
    let res = if opts.tpit {
        quote! {
            #root::tpit_rt::Tpit
        }
    } else {
        quote! {
            #root::externref::Resource
        }
    };
    let rx = if opts.tpit {
        quote! {
            u32
        }
    } else {
        quote! {
            &mut #res<Box<dyn #id2>>
        }
    };
    let t = if opts.tpit { "t" } else { "" };
    let impl_dyns = i.methods.iter().map(|(a, b)| {
        quasiquote! {
            fn #{format_ident!("{a}")}#{render_sig(opts,root,i,b,&quote! {&mut self},false)}{
                #xref
                #[link(wasm_import_module = #{format!("{t}pit/{}",i.rid_str())})]
                extern "C"{
                    #[link_name = #a]
                    fn go #{render_sig(opts,root, i,b, &quote! {this: #rx},true)};
                }
                return unsafe{go(#{
                    if opts.tpit{
                        quote!{self.ptr()}
                    }else{
                        quote!{self}
                    }
                },#{
                    let params = b.params.iter().enumerate().map(|(a,b)|{
                        let mut c = format_ident!("p{a}");
                        let mut c = quote!{
                            #c
                        };
                        if let Arg::Resource { ty, nullable, take, ann } = b{
                            if opts.tpit{
                                if !*take{
                                    c = quote!{
                                        #root::tpit_rt::Tpit::summon(&mut #c)
                                    }
                                }
                            }
                        }
                        c
                });
                    quote! {
                        #(#params),*
                    }
                })};
            }
        }
    });
    let chains2 = i.methods.iter().map(|(a,b)|quasiquote! {
       #xref
        #[export_name = #{format!("{t}pit/{id}/~{ha}/{a}")}]
        extern "C" fn #{format_ident!("{a}")}#{render_sig(opts,root,i,b,&quote! {id: u32},true)}{
            return unsafe{&mut *(TABLE.all.get())}.get_mut(&id).unwrap().#{format_ident!("{a}")}(#{
                let params = b.params.iter().enumerate().map(|(a,b)|{
                    let mut c = format_ident!("p{a}");
                    let mut c = quote!{
                        #c
                    };
                    if let Arg::Resource { ty, nullable, take, ann } = b{
                        if opts.tpit{
                            if !*take{
                                c = quote!{
                                    #c.ptr()
                                }
                            }
                        }
                    }
                    c
                });
                quote! {
                    #(#params),*
                }
            });
        }
    });
    let sb = i.to_string();
    let sc = sb
        .as_bytes()
        .iter()
        .cloned()
        .chain(once(0u8))
        .collect::<Vec<_>>();
    quasiquote! {
        pub trait #id2{
            #(#methods)*
        }
            const _: () = {
                #[link_section = ".pit-types"]
                static SECTION_CONTENT: [u8; #{sc.len()}] = #{
                    quote!{
                        [#(#sc),*]
                    }
                };
                fn alloc<T>(m: &mut ::std::collections::BTreeMap<u32,T>, x: T) -> u32{
                    let mut u = 0;
                    while m.contains_key(&u){
                        u += 1;
                    };
                    m.insert(u,x);
                    return u;
                }
                #[derive(Default)]
                struct TableCell{
                    all: std::cell::UnsafeCell<::std::collections::BTreeMap<u32,Box<dyn #id2>>>
                };
                unsafe impl Send for TableCell{}
                unsafe impl Sync for TableCell{}
                static TABLE: ::std::sync::LazyLock<TableCell> = ::std::sync::LazyLock::new(||TableCell::default());
                impl #id2 for #res<Box<dyn #id2>>{
                    #(#impl_dyns)*
                }
                #xref
                #[export_name = #{format!("{t}pit/{id}/~{ha}.drop")}]
                extern "C" fn _drop(a: u32){
                    unsafe{
                        (&mut *(TABLE.all.get())).remove(&a)
                    };
                }
                #(#chains2)*
                impl From<Box<dyn #id2>> for #res<Box<dyn #id2>>{
                    fn from(a: Box<dyn #id2>) -> Self{
                        #xref
                        #[link(wasm_import_module = #{format!("pit/{}",i.rid_str())})]
                        extern "C"{
                            #[link_name = #{format!("~{ha}")}]
                            fn _push(a: u32) -> #res<Box<dyn #id2>>;
                        }
                        return unsafe{
                            _push(alloc(&mut *(TABLE.all.get()),a))
                        }
                    }
                }
            };
            // mod #internal{
            //     use super::#id2;
            //     pub fn push(a: Box<dyn #id2>) -> #root::externref::Resource<Box<dyn #id2>>{
            //         return unsafe{
            //             _push(Box::into_raw(Box::new(a)))
            //         }
            //     }
            // }
    }
}
pub fn render_sig(
    opts: &Opts,
    root: &TokenStream,
    base: &Interface,
    s: &Sig,
    self_: &TokenStream,
    ffi: bool,
) -> TokenStream {
    let params = s
        .params
        .iter()
        .map(|a| render_ty(opts, root, base, a, ffi))
        .enumerate()
        .map(|(a, b)| quasiquote!(#{format_ident!("p{a}")} : #b));
    let params = once(self_).cloned().chain(params);
    let rets = s.rets.iter().map(|a| render_ty(opts, root, base, a, ffi));
    quote! {
        (#(#params),*) -> (#(#rets),*)
    }
}
pub fn render_ty(
    opts: &Opts,
    root: &TokenStream,
    base: &Interface,
    p: &Arg,
    ffi: bool,
) -> TokenStream {
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
        } => {
            if !opts.tpit {
                let ty = match ty {
                    ResTy::Of(a) => quasiquote! {
                        #root::externref::Resource<Box<dyn #{format_ident!("R{}",hex::encode(a))}>>
                    },
                    ResTy::None => quote! {
                        #root::externref::Resource<()>
                    },
                    ResTy::This => quasiquote! {
                        #root::externref::Resource<Box<dyn #{format_ident!("R{}",base.rid_str())}>>
                    },
                    _ => todo!()
                };
                let ty = if *nullable {
                    quote! {Option<#ty>}
                } else {
                    ty
                };
                let ty = if *take {
                    ty
                } else {
                    quote! {&mut #ty}
                };
                ty
            } else {
                let ty = match ty {
                    ResTy::Of(a) => quasiquote! {
                        #root::tpit_rt::Tpit<Box<dyn #{format_ident!("R{}",hex::encode(a))}>>
                    },
                    ResTy::None => quote! {
                        #root::tpit_rt::Tpit<()>
                    },
                    ResTy::This => quasiquote! {
                        #root::tpit_rt::Tpit<Box<dyn #{format_ident!("R{}",base.rid_str())}>>
                    },
                    _ => todo!()
                };
                let mut ty = if *take {
                    ty
                } else {
                    quote! {&mut #ty}
                };
                if *take && ffi {
                    ty = quote! {u32}
                }
                ty
            }
        }
        _ => todo!()
        // Arg::Func(_) => todo!()
    }
}
