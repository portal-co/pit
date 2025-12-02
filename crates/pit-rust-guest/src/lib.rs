//! # PIT Rust Guest Code Generator
//!
//! Generates Rust code for implementing and consuming PIT interfaces in WebAssembly guest modules.
//!
//! This crate provides the [`render`] function which takes a PIT interface definition
//! and generates Rust trait definitions along with the necessary FFI glue code for
//! WebAssembly interoperability.
//!
//! ## Generated Code
//!
//! For each PIT interface, this crate generates:
//! - A trait definition with all interface methods
//! - FFI import declarations for calling methods on external resources
//! - Export functions for implementing the interface
//! - Conversion implementations for boxing trait objects
//! - A static table for managing live objects
//!
//! ## Usage
//!
//! ```ignore
//! use pit_rust_guest::{render, Opts};
//! use quote::quote;
//!
//! let opts = Opts {
//!     root: quote! { ::tpit_rt },
//!     salt: vec![],
//!     tpit: true,
//! };
//!
//! let (_, interface) = pit_core::parse_interface("{ method(I32) -> (I64); }").unwrap();
//! let code = render(&opts, &interface);
//! ```
//!
//! ## Options
//!
//! - `root` - The crate path prefix for runtime types
//! - `salt` - Additional bytes to include in the unique ID hash
//! - `tpit` - Whether to use TPIT (table-based) or externref

use pit_core::{Arg, Interface, ResTy, Sig};
use proc_macro2::TokenStream;
use quasiquote::quasiquote;
use quote::{format_ident, quote};
use sha3::Digest;
use std::io::Write;
use std::iter::once;

/// Configuration options for Rust guest code generation.
pub struct Opts {
    /// The crate path prefix for runtime types (e.g., `::tpit_rt` or `::externref`).
    pub root: TokenStream,
    /// Additional bytes to include in the unique ID hash for disambiguation.
    pub salt: Vec<u8>,
    /// Whether to use TPIT (table-based externref emulation) or native externref.
    pub tpit: bool,
}

/// Renders a PIT interface as Rust code.
///
/// # Arguments
///
/// * `opts` - Code generation options
/// * `i` - The PIT interface to render
///
/// # Returns
///
/// A `TokenStream` containing the generated Rust code, suitable for inclusion
/// in a proc-macro or writing to a file after formatting.
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
/// Renders a method signature as Rust code.
///
/// # Arguments
///
/// * `opts` - Code generation options
/// * `root` - The crate path prefix
/// * `base` - The containing interface (for `this` type resolution)
/// * `s` - The method signature to render
/// * `self_` - The self parameter declaration
/// * `ffi` - Whether to render FFI-compatible types
///
/// # Returns
///
/// A `TokenStream` containing the rendered function signature.
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
/// Renders a PIT argument type as a Rust type.
///
/// # Arguments
///
/// * `opts` - Code generation options
/// * `root` - The crate path prefix
/// * `base` - The containing interface (for `this` type resolution)
/// * `p` - The argument type to render
/// * `ffi` - Whether to render FFI-compatible types
///
/// # Returns
///
/// A `TokenStream` containing the Rust type expression.
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
                    _ => todo!(),
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
                    _ => todo!(),
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
        _ => todo!(), // Arg::Func(_) => todo!()
    }
}
