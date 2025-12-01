//! # PIT C Code Generator
//!
//! Generates C header files from PIT interface definitions.
//!
//! This crate provides functionality to convert PIT interfaces into C header files
//! that can be used to implement or consume PIT resources from C code.
//!
//! ## Generated Code
//!
//! For each PIT interface, this crate generates:
//! - Struct definitions for method results
//! - Interface macro definitions using interface99
//! - Import/export function declarations with proper WebAssembly attributes
//! - Helper macros for resource management
//!
//! ## Usage
//!
//! ```ignore
//! use pit_core::parse_interface;
//! use pit_c::cify;
//!
//! let interface_str = "{ method(I32) -> (I64); }";
//! let (_, interface) = parse_interface(interface_str).unwrap();
//! let c_header = cify(&interface);
//! ```
//!
//! ## Dependencies
//!
//! The generated C code requires:
//! - `interface99.h` - For interface/trait implementation
//! - `handle.h` - For handle management
//! - `stdint.h` - For fixed-width integer types

use itertools::Itertools;
use pit_core::{Arg, Interface, ResTy};
use std::iter::once;

/// Generates a C header file from a PIT interface definition.
///
/// # Arguments
///
/// * `i` - The PIT interface to convert
///
/// # Returns
///
/// A string containing the complete C header file content, including:
/// - Include guards
/// - Type definitions for method results
/// - Interface trait definitions
/// - Import/export function implementations
///
/// # Example
///
/// ```ignore
/// let c_code = cify(&my_interface);
/// std::fs::write("MyInterface.h", c_code)?;
/// ```
pub fn cify(i: &Interface) -> String {
    let rid = i.rid_str();
    let iface = i
        .methods
        .iter()
        .map(|(a, b)| {
            format!(
                "vfunc(R{rid}_{a}_res,R{rid}_{a},VSelf,{})",
                b.params.iter().map(|a| cty(i, a, &FFIKind::C {})).join(",")
            )
        })
        .join(" ");
    let types = i
        .methods
        .iter()
        .map(|(a, b)| {
            format!(
                r#"
    typedef struct R{rid}_{a}_res{{
    {}
    }} R{rid}_{a}_res;
    typedef struct R{rid}_{a}_fres{{
    {}
    }} R{rid}_{a}_fres;
    "#,
                b.rets
                    .iter()
                    .enumerate()
                    .map(|(idx, a)| {
                        let a = cty(i, a, &FFIKind::C {});
                        format!("{a} v{idx}")
                    })
                    .join(";"),
                b.rets
                    .iter()
                    .enumerate()
                    .map(|(idx, a)| {
                        let a = cty(i, a, &FFIKind::FFI);
                        format!("{a} v{idx}")
                    })
                    .join(";")
            )
        })
        .join(";");
    let impls = i
        .methods
        .iter()
        .map(|(a, b)| {
            format!(
                r#"
                static __attribute__((import_module("pit/{rid}"), import_name("{a}"))) R{rid}_{a}_fres R{rid}_{a}_impl({});
                R{rid}_{a}_res handle_t_R{rid}_{a}({}){{
                VSELF(handle_t);
                    R{rid}_{a}_fres fres = R{rid}_{a}_impl(handle_borrow(self),{});
                    return R{rid}_{a}_res{{
                        {}
                    }};
                }};
                __attribute__((export_name("pit/{rid}/~c_impl/{a}"))) static R{rid}_{a}_fres R{rid}_{a}_export({}){{
                    R{rid}_{a}_res res = VCALL(me,R{rid}_{a},{});
                    return R{rid}_{a}_fres{{
                        {}
                    }};
                }};
        "#,
                b.rets
                    .iter()
                    .enumerate()
                    .map(|(idx, a)| {
                        let a = cty(i, a, &FFIKind::FFI);
                        format!("{a} v{idx}")
                    })
                    .join(","),
                once(format!("__externref_t self")).chain(b.params
                    .iter()
                    .enumerate()
                    .map(|(idx, a)| {
                        let a = cty(i, a, &FFIKind::C {});
                        format!("{a} v{idx}")
                    }))
                    .join(","),
                b.params.iter().enumerate().map(|(idx,a)|{
                    let mut s = format!("v{idx}");
                    if let Arg::Resource { ty, nullable, take, ann } = a{
                        s = format!("handle_new({s})");
                        if !matches!(ty,ResTy::None){
                            let c = cty(i, a, &FFIKind::C {  });
                            s = format!("DYN(handle_t,{c},({{
                                handle_t handle = {s};
                                handle_t* h = malloc(sizeof(handle_t));
                                *h = handle;
                                h
                            }}))")
                        }
                    }
                    return s;
                }).join(","),
                b.rets.iter().enumerate().map(|(idx,a)|{
                    let mut s = format!("fres.v{idx}");
                    if let Arg::Resource { ty, nullable, take, ann } = a{
                        if !matches!(ty,ResTy::None){
                            let c = cty(i, a, &FFIKind::C {  });
                            s = format!("{c}_ref({s})")
                        }
                        s = format!("handle_pop({s})")
                    }
                    return s;
                }).join(","),
                once(format!("R{rid} me")).chain(b.params
                    .iter()
                    .enumerate()
                    .map(|(idx, a)| {
                        let a = cty(i, a, &FFIKind::FFI);
                        format!("{a} v{idx}")
                    }))
                    .join(","),
                b.params.iter().enumerate().map(|(idx,a)|{
                    let mut s = format!("v{idx}");
                    if let Arg::Resource { ty, nullable, take, ann } = a{
                        if !matches!(ty,ResTy::None){
                            let c = cty(i, a, &FFIKind::C {  });
                            s = format!("{c}_ref({s})")
                        }
                        s = format!("handle_{}({s})",if *take{
                            "pop"
                        }else{
                            "borrow"
                        })
                    }
                    return s;
                }).join(","),
                b.rets.iter().enumerate().map(|(idx,a)|{
                    let mut s = format!("res.v{idx}");
                    if let Arg::Resource { ty, nullable, take, ann } = a{
                        s = format!("handle_new({s})");
                        if !matches!(ty,ResTy::None){
                            let c = cty(i, a, &FFIKind::C {  });
                            s = format!("DYN(handle_t,{c},({{
                                handle_t handle = {s};
                                handle_t* h = malloc(sizeof(handle_t));
                                *h = handle;
                                h
                            }}))")
                        }
                    }
                    return s;
                }).join(","),
            )
        })
        .join(";\n");
    format!(
        r#"
    #ifndef R{rid}
    #define R{rid}
    #include <interface99.h>
    #include <handle.h>
    #include <stdint.h>
    #define R{rid}_IFACE {iface}
    #define R{rid}_EXTENDS (Droppable)
    {types}
    interface(R{rid});
    declImplExtern(R{rid},handle_t);
    extern __externref_t R{rid}_ref(R{rid} rid);
    #define R{rid}_of(a) handle_new(R{rid}_ref(a))
    #endif
    #ifdef R{rid}_IMPL
    __attribute__((import_module("pit/{rid}"), import_name("~c_impl"))) extern __externref_t $$of_${rid}({rid} rid);
    __externref_t R{rid}_ref({rid} rid){{
        return $$of_${rid}(rid);
    }};
    __attribute__((export_name("pit/{rid}/~c_impl.drop"))) static void drop({rid} rid){{
        VCALL_SUPER(rid,Droppable,drop);
        free(rid.self);
    }};
    {impls}
    implExtern(R{rid},handle_t);
    #endif
    "#
    )
}
/// FFI kind enumeration for type conversion.
///
/// Determines whether a type should be rendered for the WebAssembly FFI boundary
/// or for high-level C code.
pub enum FFIKind {
    /// Raw WebAssembly FFI type (uses `__externref_t` for resources)
    FFI,
    /// High-level C type (uses concrete resource types)
    C {},
}

/// Converts a PIT argument type to its C representation.
///
/// # Arguments
///
/// * `i` - The containing interface (used for `this` type resolution)
/// * `t` - The PIT argument type to convert
/// * `ffi_kind` - Whether to generate FFI or high-level C types
///
/// # Returns
///
/// A string containing the C type name.
pub fn cty(i: &Interface, t: &Arg, ffi_kind: &FFIKind) -> String {
    match t {
        Arg::I32 => format!("uint32_t"),
        Arg::I64 => format!("uint64_t"),
        Arg::F32 => format!("float"),
        Arg::F64 => format!("double"),
        Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => match ffi_kind {
            FFIKind::FFI => format!("__externref_t"),
            FFIKind::C {} => match ty {
                pit_core::ResTy::None => format!("handle_t"),
                pit_core::ResTy::Of(a) => format!("R{}", hex::encode(a)),
                pit_core::ResTy::This => format!("R{}", i.rid_str()),
                _ => todo!(),
            },
        },
        _ => todo!(),
    }
}
