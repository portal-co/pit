use std::{
    collections::{BTreeMap, BTreeSet},
    iter::once,
    mem::{replace, take},
};

use anyhow::Context;
use pit_core::{Arg, ResTy};
use waffle::{
    util::new_sig, Block, BlockTarget, Export, ExportKind, Func, FuncDecl, FunctionBody, Import,
    ImportKind, Module, Operator, SignatureData, Table, TableData, Type, Value, WithNullable,
};
use waffle_ast::{add_op, results_ref_2, Builder, Expr};

use crate::get_interfaces;

pub use waffle_ast::tutils::*;

// use waffle::{util::new_sig, Module};

pub fn to_waffle_type(t: &pit_core::Arg, tpit: bool) -> waffle::Type {
    match t {
        pit_core::Arg::I32 => waffle::Type::I32,
        pit_core::Arg::I64 => waffle::Type::I64,
        pit_core::Arg::F32 => waffle::Type::F32,
        pit_core::Arg::F64 => waffle::Type::F64,
        pit_core::Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => {
            if tpit {
                waffle::Type::I32
            } else {
                waffle::Type::Heap(WithNullable {
                    nullable: true,
                    value: waffle::HeapType::ExternRef,
                })
            }
        },
        _ => todo!()
    }
}
pub fn to_waffle_sig(m: &mut Module, t: &pit_core::Sig, tpit: bool) -> waffle::Signature {
    return new_sig(
        m,
        waffle::SignatureData::Func {
            params: once(if tpit {
                Type::I32
            } else {
                waffle::Type::Heap(WithNullable {
                    nullable: true,
                    value: waffle::HeapType::ExternRef,
                })
            })
            .chain(t.params.iter().map(|a| to_waffle_type(a, tpit)))
            .collect(),
            returns: t.rets.iter().map(|a| to_waffle_type(a, tpit)).collect(),
        },
    );
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
            let f = m
                .funcs
                .push(waffle::FuncDecl::Import(s, format!("{module}.{name}")));
            m.imports.push(Import {
                module,
                name,
                kind: waffle::ImportKind::Func(f),
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
            returns: vec![waffle::Type::Heap(WithNullable{nullable: true, value: waffle::HeapType::ExternRef})],
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
        let sig = new_sig(
            m,
            SignatureData::Func {
                params: tys.iter().map(|a| to_waffle_type(a, false)).collect(),
                returns: meth.rets.iter().map(|a| to_waffle_type(a, false)).collect(),
            },
        );
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
        f.set_terminator(f.entry, waffle::Terminator::Return { values: r });
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
                params: vec![waffle::Type::Heap(WithNullable{nullable: true, value: waffle::HeapType::ExternRef})],
                returns: vec![],
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
        let sig = new_sig(
            m,
            SignatureData::Func {
                params: tys.iter().map(|a| to_waffle_type(a, false)).collect(),
                returns: vec![],
            },
        );
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
                f.set_terminator(f.entry, waffle::Terminator::Return { values: vec![] });
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
