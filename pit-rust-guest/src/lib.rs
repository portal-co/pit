use std::iter::once;

use pit_core::{Arg, Interface, ResTy, Sig};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use quasiquote::quasiquote;
use sha3::Digest;
use std::io::Write;
pub struct Opts{
    pub root: TokenStream,
    pub salt: Vec<u8>
}
pub fn render(opts: &Opts, i: &Interface) -> TokenStream{
    let root = &opts.root;
    let id = i.rid_str();
    let mut ha = sha3::Sha3_256::default();
    write!(ha,"~{}",id);
    ha.write(&opts.salt);
    let ha = hex::encode(ha.finalize());
    let id2 = format_ident!("R{}",i.rid_str());
    let internal = format_ident!("{id}_utils");
    let methods = i.methods.iter().map(|(a,b)|quasiquote! {
        fn #{format_ident!("{a}")}#{render_sig(root,i,b,&quote! {&self})}
    });
    let impl_dyns = i.methods.iter().map(|(a,b)|quasiquote! {
        fn #{format_ident!("{a}")}#{render_sig(root,i,b,&quote! {&self})}{
            #[#root::externref::externref(crate = #{quote! {#root::externref}.to_string()})]
            #[wasm_import_module = #{format!("pit/{}",i.rid_str())}]
            extern "C"{
                #[wasm_import_name = #a]
                fn go #{render_sig(root, i,b, &quote! {&#root::externref::Resource<Box<dyn #id2>>})}
            };
            return go(self,#{
                let params = b.params.iter().enumerate().map(|(a,_)|format_ident!("p{a}"));
                quote! {
                    #(#params),*
                }
            });
        }
    });
    let chains2 = i.methods.iter().map(|(a,b)|quasiquote! {
        #[#root::externref::externref(crate = #{quote! {#root::externref}.to_string()})]
        #[export_name = #{format!("pit/{id}/~{ha}/{a}")}]
        fn #{format_ident!("{a}")}#{render_sig(root,i,b,&quote! {id: *mut Box<dyn #id>})}{
            return unsafe{&mut *id}.#{format_ident!("{a}")}(#{
                let params = b.params.iter().enumerate().map(|(a,_)|format_ident!("p{a}"));
                quote! {
                    #(#params),*
                }
            });
        }
    });
    quasiquote!{
        pub trait #id2{
            #(#methods)*
        }
        mod #internal{
            const _: () = {
                #[link_section = ".pit-types"]
                static SECTION_CONTENT: [u8; _] = #{
                    let b = i.to_string();
                    let c = b.as_bytes().iter().cloned().chain(once(0u8));
                    quote!{
                        [#(#c),*]
                    }
                };
            };
            use super::#id2;
            impl #id2 for #root::externref::Resource<Box<dyn #id2>>{
                #(#impl_dyns),*
            }
            #[#root::externref::externref(crate = #{quote! {#root::externref}.to_string()})]
            #[export_name = #{format!("pit/{id}/~{ha}.drop")}]
            fn _drop(a: *mut Box<dyn #id>){
                unsafe{
                    Box::from_raw(a)
                }
            }
            #(#chains2);*
            #[#root::externref::externref(crate = #{quote! {#root::externref}.to_string()})]
            #[wasm_import_module = #{format!("pit/{}",i.rid_str())}]
            extern "C"{
                #[wasm_import_name = #{format!("~{ha}")}]
                fn _push(a: *mut Box<dyn #id2>) -> #root::externref::Resource<Box<dyn #id2>>;
            }
            pub fn push(a: Box<dyn #id2>) -> #root::externref::Resource<Box<dyn #id2>>{
                return unsafe{
                    _push(Box::into_raw(Box::new(a)))
                }
            }
        }
    }
}
pub fn render_sig(root: &TokenStream,base: &Interface, s: &Sig, self_: &TokenStream) -> TokenStream{
    let params = s.params.iter().map(|a|render_ty(root,base,a)).enumerate().map(|(a,b)|quasiquote!(#{format_ident!("p{a}")} : #b));
    let params = once(self_).cloned().chain(params);
    let rets = s.rets.iter().map(|a|render_ty(root,base,a));
    quote! {
        (#(#params),*) -> (#(#rets),*)
    }
}
pub fn render_ty(root: &TokenStream,base:&Interface, p: &Arg) -> TokenStream{
    match p{
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
        Arg::Resource{ty,nullable,take} => {
            let ty = match ty{
                ResTy::Of(a) => quasiquote!{
                    #root::externref::Resource<Box<dyn #{format_ident!("R{}",hex::encode(a))}>>
                },
                ResTy::None => quote! {
                    #root::externref::Resource<()>
                },
                ResTy::This => quasiquote! {
                    #root::externref::Resource<Box<dyn #{format_ident!("R{}",base.rid_str())}>>
                }
            };
            let ty = if *nullable{
                quote! {Option<#ty>}
            }else{
                ty
            };
            let ty = if *take{
                ty
            }else{
                quote! {&#ty}
            };
            ty
        },
        // Arg::Func(_) => todo!()
    }
}