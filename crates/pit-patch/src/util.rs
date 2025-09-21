use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::iter::once;
use core::mem::{replace, take};

use anyhow::Context;
use pit_core::{Arg, ResTy};
use portal_pc_waffle::{
    util::new_sig, Block, BlockTarget, Export, ExportKind, Func, FuncDecl, FunctionBody, Import,
    ImportKind, Module, Operator, SignatureData, Table, TableData, Type, Value, WithNullable,
};
pub fn add_op(f: &mut FunctionBody, args: &[Value], types: &[Type], op: Operator) -> Value{
    let args = f.arg_pool.from_iter(args.iter().cloned());
    let types = f.type_pool.from_iter(types.iter().cloned());
    f.values.push(portal_pc_waffle::ValueDef::Operator(op, args, types))
}
// use waffle_ast::{add_op, results_ref_2, Builder, Expr};

use crate::get_interfaces;

// pub use waffle_ast::tutils::*;

// use portal_pc_waffle::{util::new_sig, Module};

pub fn to_waffle_type(t: &pit_core::Arg, tpit: bool) -> portal_pc_waffle::Type {
    match t {
        pit_core::Arg::I32 => portal_pc_waffle::Type::I32,
        pit_core::Arg::I64 => portal_pc_waffle::Type::I64,
        pit_core::Arg::F32 => portal_pc_waffle::Type::F32,
        pit_core::Arg::F64 => portal_pc_waffle::Type::F64,
        pit_core::Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => {
            if tpit {
                portal_pc_waffle::Type::I32
            } else {
                portal_pc_waffle::Type::Heap(WithNullable {
                    nullable: true,
                    value: portal_pc_waffle::HeapType::ExternRef,
                })
            }
        }
        _ => todo!(),
    }
}
pub fn to_waffle_type_in(
    t: &pit_core::Arg,
    tpit: bool,
    module: &mut Module,
) -> portal_pc_waffle::Type {
    match t {
        t => to_waffle_type(t, tpit),
    }
}
pub fn to_waffle_sig(m: &mut Module, t: &pit_core::Sig, tpit: bool) -> portal_pc_waffle::Signature {
    let s = portal_pc_waffle::SignatureData::Func {
        params: once(if tpit {
            Type::I32
        } else {
            portal_pc_waffle::Type::Heap(WithNullable {
                nullable: true,
                value: portal_pc_waffle::HeapType::ExternRef,
            })
        })
        .chain(t.params.iter().map(|a| to_waffle_type_in(a, tpit, m)))
        .collect(),
        returns: t
            .rets
            .iter()
            .map(|a| to_waffle_type_in(a, tpit, m))
            .collect(),
        shared: true,
    };
    return new_sig(m, s);
}
pub fn waffle_funcs(m: &mut Module, i: &pit_core::Interface, tpit: bool) -> BTreeMap<String, Func> {
    return i
        .methods
        .iter()
        .map(|(a, b)| {
            let module = format!("{}pit/{}", i.rid_str(), if tpit { "t" } else { "" });
            let name = a.clone();
            if let Some(f) = m.imports.iter().find_map(|i| {
                if i.module == module && i.name == name {
                    match &i.kind {
                        ImportKind::Func(f) => Some(*f),
                        _ => None,
                    }
                } else {
                    None
                }
            }) {
                return (a.clone(), f);
            };
            let s = to_waffle_sig(m, b, tpit);
            let f = m.funcs.push(portal_pc_waffle::FuncDecl::Import(
                s,
                format!("{module}.{name}"),
            ));
            m.imports.push(Import {
                module,
                name,
                kind: portal_pc_waffle::ImportKind::Func(f),
            });
            (a.clone(), f)
        })
        .collect();
}
pub fn canon(m: &mut Module, i: &pit_core::Interface, destruct: Option<Func>, name: &str) -> Func {
    let tys = i
        .methods
        .iter()
        .flat_map(|a| a.1.rets.iter())
        .cloned()
        .collect::<Vec<_>>();
    let sig = new_sig(
        m,
        SignatureData::Func {
            params: tys.iter().map(|a| to_waffle_type(a, false)).collect(),
            returns: vec![portal_pc_waffle::Type::Heap(WithNullable {
                nullable: true,
                value: portal_pc_waffle::HeapType::ExternRef,
            })],
            shared: true,
        },
    );
    let f = m
        .funcs
        .push(FuncDecl::Import(sig, format!("pit-canon/{}", i.rid_str())));
    m.imports.push(Import {
        module: format!("pit/{}", i.rid_str()),
        name: format!("~{name}"),
        kind: ImportKind::Func(f),
    });
    let mut j = 0;
    for (s, meth) in i.methods.iter() {
        let sig = SignatureData::Func {
            params: tys.iter().map(|a| to_waffle_type_in(a, false, m)).collect(),
            returns: meth
                .rets
                .iter()
                .map(|a| to_waffle_type_in(a, false, m))
                .collect(),
                shared: true,
        };
        let sig = new_sig(m, sig);
        let mut f = FunctionBody::new(&m, sig);
        let args = f.blocks[f.entry]
            .params
            .iter()
            .map(|a| a.1)
            .collect::<Vec<_>>();
        let r = meth
            .rets
            .iter()
            .map(|_| {
                let a = args[j];
                j += 1;
                a
            })
            .collect();
        f.set_terminator(f.entry, portal_pc_waffle::Terminator::Return { values: r });
        let f = m.funcs.push(FuncDecl::Body(
            sig,
            format!("pit-canon/{}/{s}", i.rid_str()),
            f,
        ));
        m.exports.push(Export {
            name: format!("pit/{}/~{name}/{s}", i.rid_str()),
            kind: ExportKind::Func(f),
        });
    }
    {
        let sig = new_sig(
            m,
            SignatureData::Func {
                params: vec![portal_pc_waffle::Type::Heap(WithNullable {
                    nullable: true,
                    value: portal_pc_waffle::HeapType::ExternRef,
                })],
                returns: vec![],
                shared: true,
            },
        );
        let dropper = m.funcs.push(FuncDecl::Import(
            sig,
            format!("pit-canon/{}--idrop", i.rid_str()),
        ));
        m.imports.push(Import {
            module: format!("pit"),
            name: format!("drop"),
            kind: ImportKind::Func(dropper),
        });
        let sig = SignatureData::Func {
            params: tys.iter().map(|a| to_waffle_type_in(a, false, m)).collect(),
            returns: vec![],
            shared: true,
        };
        let sig = new_sig(m, sig);
        let f = match destruct {
            None => {
                let mut f = FunctionBody::new(&m, sig);
                let args = f.blocks[f.entry]
                    .params
                    .iter()
                    .map(|a| a.1)
                    .collect::<Vec<_>>();
                for (v, a) in args.iter().zip(tys.iter()) {
                    if let Arg::Resource {
                        ty,
                        nullable,
                        take,
                        ann,
                    } = a
                    {
                        f.add_op(
                            f.entry,
                            Operator::Call {
                                function_index: dropper,
                            },
                            &[*v],
                            &[],
                        );
                    }
                }
                f.set_terminator(
                    f.entry,
                    portal_pc_waffle::Terminator::Return { values: vec![] },
                );
                m.funcs.push(FuncDecl::Body(
                    sig,
                    format!("pit-canon/{}.drop", i.rid_str()),
                    f,
                ))
            }
            Some(a) => a,
        };
        m.exports.push(Export {
            name: format!("pit/{}/~{name}.drop", i.rid_str()),
            kind: ExportKind::Func(f),
        });
    };
    return f;
}
