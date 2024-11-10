use std::{
    collections::{BTreeMap, BTreeSet},
    iter::once,
    mem::{replace, take},
};

use anyhow::Context;
use pit_core::{Arg, ResTy};
use waffle::{
    util::new_sig, Block, BlockTarget, Export, ExportKind, Func, FuncDecl, FunctionBody, Import,
    ImportKind, Module, Operator, SignatureData, Table, TableData, Type, Value,
};
use waffle_ast::{add_op, results_ref_2, Builder, Expr};

use crate::get_interfaces;

pub fn talloc(m: &mut Module, t: Table, aux: &[Table]) -> anyhow::Result<Func> {
    let e = m.tables[t].ty.clone();
    let atys = aux
        .iter()
        .map(|a| m.tables[*a].ty.clone())
        .collect::<Vec<_>>();
    let sig = new_sig(
        m,
        SignatureData {
            params: vec![e].into_iter().chain(atys.into_iter()).collect(),
            returns: vec![Type::I32],
        },
    );
    let mut f = FunctionBody::new(m, sig);
    let avals = f.blocks[f.entry].params[1..]
        .iter()
        .map(|a| a.1)
        .collect::<Vec<_>>();
    let n = f.add_block();
    let zero = add_op(&mut f, &[], &[Type::I32], Operator::I32Const { value: 0 });
    f.append_to_block(f.entry, zero);
    f.set_terminator(
        f.entry,
        waffle::Terminator::Br {
            target: BlockTarget {
                block: n,
                args: vec![zero],
            },
        },
    );
    let idx = f.add_blockparam(n, Type::I32);
    let mut e = Expr::Bind(
        Operator::RefIsNull,
        vec![Expr::Bind(
            Operator::TableGet { table_index: t },
            vec![Expr::Leaf(idx)],
        )],
    );
    let (r, o) = e.build(m, &mut f, n)?;
    let mut e = Expr::Bind(
        Operator::I32Add,
        vec![
            Expr::Bind(Operator::I32Const { value: 1 }, vec![]),
            Expr::Leaf(idx),
        ],
    );
    let (s, o) = e.build(m, &mut f, o)?;
    let p = f.add_block();
    f.set_terminator(
        o,
        waffle::Terminator::CondBr {
            cond: r,
            if_true: BlockTarget {
                block: p,
                args: vec![],
            },
            if_false: BlockTarget {
                block: n,
                args: vec![s],
            },
        },
    );
    let q = f.add_block();
    let r = f.add_block();
    let mut e = Expr::Bind(
        Operator::I32Eq,
        vec![
            Expr::Bind(Operator::TableSize { table_index: t }, vec![]),
            Expr::Leaf(idx),
        ],
    );
    let (s, o) = e.build(m, &mut f, p)?;
    f.set_terminator(
        o,
        waffle::Terminator::CondBr {
            cond: s,
            if_true: BlockTarget {
                block: r,
                args: vec![],
            },
            if_false: BlockTarget {
                block: q,
                args: vec![],
            },
        },
    );
    let mut e = Expr::Bind(
        Operator::TableSet { table_index: t },
        vec![Expr::Leaf(idx), Expr::Leaf(f.blocks[f.entry].params[0].1)],
    );
    let (s, mut o) = e.build(m, &mut f, q)?;
    for (a, v) in aux.iter().zip(avals.iter()) {
        let b = f.add_block();
        let d = f.add_block();
        let mut e = Expr::Bind(
            Operator::I32GeU,
            vec![
                Expr::Leaf(idx),
                Expr::Bind(Operator::TableSize { table_index: *a }, vec![]),
            ],
        );
        let c;
        (c, o) = e.build(m, &mut f, o)?;
        f.set_terminator(
            o,
            waffle::Terminator::CondBr {
                cond: c,
                if_true: BlockTarget {
                    block: b,
                    args: vec![],
                },
                if_false: BlockTarget {
                    block: d,
                    args: vec![],
                },
            },
        );
        let mut e = Expr::Bind(
            Operator::TableGrow { table_index: *a },
            vec![Expr::Bind(
                Operator::I32Sub,
                vec![
                    Expr::Leaf(idx),
                    Expr::Bind(Operator::TableSize { table_index: *a }, vec![]),
                ],
            )],
        );
        let (_, b) = e.build(m, &mut f, b)?;
        f.set_terminator(
            b,
            waffle::Terminator::Br {
                target: BlockTarget {
                    block: d,
                    args: vec![],
                },
            },
        );
        o = d;
        f.add_op(o, Operator::TableSet { table_index: *a }, &[idx, *v], &[]);
    }
    f.set_terminator(o, waffle::Terminator::Return { values: vec![idx] });
    let mut e = Expr::Bind(
        Operator::TableGrow { table_index: t },
        vec![
            Expr::Bind(Operator::I32Const { value: 1 }, vec![]),
            Expr::Leaf(f.blocks[f.entry].params[0].1),
        ],
    );
    let (s, mut o) = e.build(m, &mut f, r)?;
    for (a, v) in aux.iter().zip(avals.iter()) {
        let b = f.add_block();
        let d = f.add_block();
        let mut e = Expr::Bind(
            Operator::I32GeU,
            vec![
                Expr::Leaf(idx),
                Expr::Bind(Operator::TableSize { table_index: *a }, vec![]),
            ],
        );
        let c;
        (c, o) = e.build(m, &mut f, o)?;
        f.set_terminator(
            o,
            waffle::Terminator::CondBr {
                cond: c,
                if_true: BlockTarget {
                    block: b,
                    args: vec![],
                },
                if_false: BlockTarget {
                    block: d,
                    args: vec![],
                },
            },
        );
        let mut e = Expr::Bind(
            Operator::TableGrow { table_index: *a },
            vec![Expr::Bind(
                Operator::I32Sub,
                vec![
                    Expr::Leaf(idx),
                    Expr::Bind(Operator::TableSize { table_index: *a }, vec![]),
                ],
            )],
        );
        let (_, b) = e.build(m, &mut f, b)?;
        f.set_terminator(
            b,
            waffle::Terminator::Br {
                target: BlockTarget {
                    block: d,
                    args: vec![],
                },
            },
        );
        o = d;
        f.add_op(o, Operator::TableSet { table_index: *a }, &[idx, *v], &[]);
    }
    f.set_terminator(o, waffle::Terminator::Return { values: vec![idx] });
    return Ok(m
        .funcs
        .push(waffle::FuncDecl::Body(sig, format!("talloc"), f)));
}
pub fn tfree(m: &mut Module, t: Table, aux: &[Table]) -> anyhow::Result<Func> {
    let ety = m.tables[t].ty.clone();
    let atys = aux
        .iter()
        .map(|a| m.tables[*a].ty.clone())
        .collect::<Vec<_>>();
    let sig = new_sig(
        m,
        SignatureData {
            params: vec![Type::I32],
            returns: vec![ety.clone()]
                .into_iter()
                .chain(atys.into_iter())
                .collect(),
        },
    );
    let mut f = FunctionBody::new(m, sig);
    let o = f.entry;
    let e = f.blocks[f.entry].params[0].1;
    let r = once(t)
        .chain(aux.iter().cloned())
        .zip(f.rets.clone().into_iter())
        .map(|(t, e2)| f.add_op(o, Operator::TableGet { table_index: t }, &[e], &[e2]))
        .collect();
    let mut e = Expr::Bind(
        Operator::TableSet { table_index: t },
        vec![
            Expr::Leaf(f.blocks[f.entry].params[0].1),
            Expr::Bind(Operator::RefNull { ty: ety }, vec![]),
        ],
    );
    let (_, o) = e.build(m, &mut f, o)?;
    f.set_terminator(o, waffle::Terminator::Return { values: r });
    return Ok(m
        .funcs
        .push(waffle::FuncDecl::Body(sig, format!("tfree"), f)));
}

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
                waffle::Type::ExternRef
            }
        }
    }
}
pub fn to_waffle_sig(m: &mut Module, t: &pit_core::Sig, tpit: bool) -> waffle::Signature {
    return new_sig(
        m,
        waffle::SignatureData {
            params: once(if tpit { Type::I32 } else { Type::ExternRef })
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
        SignatureData {
            params: tys.iter().map(|a| to_waffle_type(a, false)).collect(),
            returns: vec![Type::ExternRef],
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
            SignatureData {
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
            SignatureData {
                params: vec![Type::ExternRef],
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
            SignatureData {
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
