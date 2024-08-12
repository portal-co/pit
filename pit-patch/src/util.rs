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

pub fn talloc(m: &mut Module, t: Table) -> anyhow::Result<Func> {
    let e = m.tables[t].ty.clone();
    let sig = new_sig(
        m,
        SignatureData {
            params: vec![e],
            returns: vec![Type::I32],
        },
    );
    let mut f = FunctionBody::new(m, sig);
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
    let (s, o) = e.build(m, &mut f, q)?;
    f.set_terminator(o, waffle::Terminator::Return { values: vec![idx] });
    let mut e = Expr::Bind(
        Operator::TableGrow { table_index: t },
        vec![
            Expr::Bind(Operator::I32Const { value: 1 }, vec![]),
            Expr::Leaf(f.blocks[f.entry].params[0].1),
        ],
    );
    let (s, o) = e.build(m, &mut f, r)?;
    f.set_terminator(o, waffle::Terminator::Return { values: vec![idx] });
    return Ok(m
        .funcs
        .push(waffle::FuncDecl::Body(sig, format!("talloc"), f)));
}
pub fn tfree(m: &mut Module, t: Table) -> anyhow::Result<Func> {
    let ety = m.tables[t].ty.clone();
    let sig = new_sig(
        m,
        SignatureData {
            params: vec![Type::I32],
            returns: vec![ety.clone()],
        },
    );
    let mut f = FunctionBody::new(m, sig);
    let o = f.entry;
    let mut e = Expr::Bind(
        Operator::TableGet { table_index: t },
        vec![Expr::Leaf(f.blocks[f.entry].params[0].1)],
    );
    let (r, o) = e.build(m, &mut f, o)?;
    let mut e = Expr::Bind(
        Operator::TableSet { table_index: t },
        vec![
            Expr::Leaf(f.blocks[f.entry].params[0].1),
            Expr::Bind(Operator::RefNull { ty: ety }, vec![]),
        ],
    );
    let (_, o) = e.build(m, &mut f, o)?;
    return Ok(m
        .funcs
        .push(waffle::FuncDecl::Body(sig, format!("tfree"), f)));
}

// use waffle::{util::new_sig, Module};

pub fn to_waffle_type(t: &pit_core::Arg) -> waffle::Type {
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
        } => waffle::Type::ExternRef,
    }
}
pub fn to_waffle_sig(m: &mut Module, t: &pit_core::Sig) -> waffle::Signature {
    return new_sig(
        m,
        waffle::SignatureData {
            params: t.params.iter().map(to_waffle_type).collect(),
            returns: t.rets.iter().map(to_waffle_type).collect(),
        },
    );
}
pub fn waffle_funcs(m: &mut Module, i: &pit_core::Interface) -> BTreeMap<String, Func> {
    return i
        .methods
        .iter()
        .map(|(a, b)| {
            let module = format!("pit/{}", i.rid_str());
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
            let s = to_waffle_sig(m, b);
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
pub fn shim(
    retref: bool,
    f: &mut FunctionBody,
    mut k: Block,
    r: &Arg,
    mut v: Value,
    talloc: Func,
    tfree: Func,
    table: Table,
) -> anyhow::Result<(Value, Block)> {
    let Arg::Resource {
        ty,
        nullable,
        take,
        ann,
    } = r
    else {
        return Ok((v, k));
    };
    let end = f.add_block();
    let ep = f.add_blockparam(end, if retref { Type::ExternRef } else { Type::I32 });
    // if *nullable {
    let s = if retref {
        let a = add_op(f, &[], &[Type::I32], Operator::I32Const { value: 0 });
        f.append_to_block(k, a);
        let a = add_op(f, &[a, v], &[Type::I32], Operator::I32Eq);
        f.append_to_block(k, a);
        a
    } else {
        let a = add_op(f, &[v], &[Type::I32], Operator::RefIsNull);
        f.append_to_block(k, a);
        a
    };
    let n = f.add_block();
    if retref {
        let a = add_op(
            f,
            &[],
            &[Type::ExternRef],
            Operator::RefNull {
                ty: Type::ExternRef,
            },
        );
        f.append_to_block(n, a);
        f.set_terminator(
            n,
            waffle::Terminator::Br {
                target: BlockTarget {
                    block: end,
                    args: vec![a],
                },
            },
        )
    } else {
        let a = add_op(f, &[], &[Type::I32], Operator::I32Const { value: 0 });
        f.append_to_block(n, a);
        f.set_terminator(
            n,
            waffle::Terminator::Br {
                target: BlockTarget {
                    block: end,
                    args: vec![a],
                },
            },
        )
    }
    let kk = f.add_block();
    f.set_terminator(
        k,
        waffle::Terminator::CondBr {
            cond: s,
            if_true: BlockTarget {
                block: n,
                args: vec![],
            },
            if_false: BlockTarget {
                block: kk,
                args: vec![],
            },
        },
    );
    k = kk;
    if retref {
        let a = add_op(f, &[], &[Type::I32], Operator::I32Const { value: 1 });
        f.append_to_block(k, a);
        let a = add_op(f, &[v, a], &[Type::I32], Operator::I32Sub);
        f.append_to_block(k, a);
        v = a;
    }
    // }

    match (take, retref) {
        (true, true) => {
            v = add_op(
                f,
                &[v],
                &[Type::ExternRef],
                Operator::Call {
                    function_index: tfree,
                },
            );
            f.append_to_block(k, v);
        }
        (_, false) => {
            v = add_op(
                f,
                &[v],
                &[Type::I32],
                Operator::Call {
                    function_index: talloc,
                },
            );
            f.append_to_block(k, v);
        }
        (false, true) => {
            v = add_op(
                f,
                &[v],
                &[Type::ExternRef],
                Operator::TableGet { table_index: table },
            );
            f.append_to_block(k, v);
        }
    }
    if !retref {
        let a = add_op(f, &[], &[Type::I32], Operator::I32Const { value: 1 });
        f.append_to_block(k, a);
        let a = add_op(f, &[v, a], &[Type::I32], Operator::I32Add);
        f.append_to_block(k, a);
        v = a;
    }
    f.set_terminator(
        k,
        waffle::Terminator::Br {
            target: BlockTarget {
                block: end,
                args: vec![v],
            },
        },
    );
    Ok((ep, end))
}
pub fn wrap(m: &mut Module) -> anyhow::Result<()> {
    let t = m.tables.push(TableData {
        ty: Type::ExternRef,
        initial: 0,
        max: None,
        func_elements: None,
        table64: false,
    });
    let talloc = talloc(m, t)?;
    let tfree = tfree(m, t)?;
    let is = get_interfaces(m)?.into_iter().collect::<BTreeSet<_>>();
    for mut import in take(&mut m.imports) {
        if import.module == "tpit" && import.name == "void" {
            if let ImportKind::Func(f) = &mut import.kind {
                let s = m.funcs[*f].sig();
                let o = *f;
                let mut b = FunctionBody::new(&m, s);
                let e = b.entry;
                let arg = b.blocks[b.entry].params[0].1;
                let arg = add_op(
                    &mut b,
                    &[arg],
                    &[Type::ExternRef],
                    Operator::Call {
                        function_index: tfree,
                    },
                );
                b.append_to_block(e, arg);
                b.set_terminator(e, waffle::Terminator::Return { values: vec![] });
                m.funcs[o] = FuncDecl::Body(s, format!("_pit"), b);
            }
            continue;
        }
        if import.module == "tpit" && import.name == "clone" {
            if let ImportKind::Func(f) = &mut import.kind {
                let s = m.funcs[*f].sig();
                let o = *f;
                let mut b = FunctionBody::new(&m, s);
                let e = b.entry;
                let arg = b.blocks[b.entry].params[0].1;
                let arg = add_op(
                    &mut b,
                    &[arg],
                    &[Type::ExternRef],
                    Operator::TableGet { table_index: t },
                );
                b.append_to_block(e, arg);
                b.set_terminator(
                    e,
                    waffle::Terminator::ReturnCall {
                        func: talloc,
                        args: vec![arg],
                    },
                );
                m.funcs[o] = FuncDecl::Body(s, format!("_pit"), b);
            }
            continue;
        }
        if import.module == "tpit" && import.name == "drop" {
            import.module = format!("pit");
            let p = new_sig(
                m,
                SignatureData {
                    params: vec![Type::I32],
                    returns: vec![],
                },
            );
            let p = m.funcs.push(waffle::FuncDecl::Import(p, format!("_pit")));
            if let ImportKind::Func(f) = &mut import.kind {
                let s = m.funcs[*f].sig();
                let o = replace(f, p);
                let mut b = FunctionBody::new(&m, s);
                let e = b.entry;
                let arg = b.blocks[b.entry].params[0].1;
                let arg = add_op(
                    &mut b,
                    &[arg],
                    &[Type::ExternRef],
                    Operator::Call {
                        function_index: tfree,
                    },
                );
                b.append_to_block(e, arg);
                b.set_terminator(
                    e,
                    waffle::Terminator::ReturnCall {
                        func: p,
                        args: vec![arg],
                    },
                );
                m.funcs[o] = FuncDecl::Body(s, format!("_pit"), b);
            }
        }
        m.imports.push(import);
    }
    for i in is {
        let f = waffle_funcs(m, &i);
        for mut import in take(&mut m.imports) {
            if import.module == format!("tpit/{}", i.rid_str()) {
                match import.name.strip_prefix("~") {
                    Some(a) => {
                        let p = new_sig(
                            m,
                            SignatureData {
                                params: vec![Type::I32],
                                returns: vec![Type::ExternRef],
                            },
                        );
                        let p = m.funcs.push(waffle::FuncDecl::Import(p, format!("_pit")));
                        if let ImportKind::Func(f) = &mut import.kind {
                            let s = m.funcs[*f].sig();
                            let o = replace(f, p);
                            let mut b = FunctionBody::new(&m, s);
                            let e = b.entry;
                            let arg = b.blocks[b.entry].params[0].1;
                            let arg = add_op(
                                &mut b,
                                &[arg],
                                &[Type::ExternRef],
                                Operator::Call { function_index: p },
                            );
                            b.append_to_block(e, arg);
                            b.set_terminator(
                                e,
                                waffle::Terminator::ReturnCall {
                                    func: talloc,
                                    args: vec![arg],
                                },
                            );
                            m.funcs[o] = FuncDecl::Body(s, format!("_pit"), b);
                        }
                    }
                    None => {
                        let x = i
                            .methods
                            .get(&import.name)
                            .context("in getting the method")?;
                        let p = to_waffle_sig(m, x);
                        let p = m.signatures[p].clone();
                        let p = new_sig(
                            m,
                            SignatureData {
                                params: vec![Type::I32]
                                    .into_iter()
                                    .chain(p.params.into_iter().map(|a| {
                                        if a == Type::ExternRef {
                                            Type::I32
                                        } else {
                                            a
                                        }
                                    }))
                                    .collect(),
                                returns: p
                                    .returns
                                    .into_iter()
                                    .map(|a| if a == Type::ExternRef { Type::I32 } else { a })
                                    .collect(),
                            },
                        );
                        let p = m.funcs.push(waffle::FuncDecl::Import(p, format!("_pit")));
                        if let ImportKind::Func(f) = &mut import.kind {
                            let s = m.funcs[*f].sig();
                            let o = replace(f, p);
                            let mut b = FunctionBody::new(&m, s);
                            let mut k = b.entry;
                            let args = b.blocks[b.entry]
                                .params
                                .iter()
                                .map(|a| a.1)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .zip(
                                    once(Arg::Resource {
                                        ty: ResTy::This,
                                        nullable: false,
                                        take: false,
                                        ann: vec![],
                                    })
                                    .chain(x.params.iter().cloned()),
                                );
                            let mut v2 = vec![];
                            for (v, r) in args {
                                let a;
                                (a, k) = shim(false, &mut b, k, &r, v, talloc, tfree, t)?;
                                v2.push(a);
                            }
                            let rets = add_op(
                                &mut b,
                                &v2,
                                &m.signatures[m.funcs[p].sig()].returns,
                                Operator::Call { function_index: p },
                            );
                            b.append_to_block(k, rets);
                            let rets = results_ref_2(&mut b, rets);
                            let mut r2 = vec![];
                            for (v, r) in rets.iter().cloned().zip(x.rets.iter()) {
                                let a;
                                (a, k) = shim(true, &mut b, k, r, v, talloc, tfree, t)?;
                                r2.push(a);
                            }
                            b.set_terminator(k, waffle::Terminator::Return { values: r2 });
                            m.funcs[o] = FuncDecl::Body(s, format!("_pit"), b);
                        }
                    }
                }
                import.module = format!("pit/{}", i.rid_str());
            }
            m.imports.push(import)
        }
        for mut export in take(&mut m.exports) {
            if let Some(a) = export.name.strip_prefix(&format!("tpit/{}/~", i.rid_str())) {
                let a = a.to_owned();
                export.name = format!("pit/{}/~{a}", i.rid_str());
                match a.split_once("/") {
                    None => {
                        let b = a.strip_suffix(".drop").context("in egtting the drop")?;
                    }
                    Some((a, b)) => {
                        let x = i.methods.get(b).context("in getting the method")?;
                        let p = to_waffle_sig(m, x);
                        let p = m.signatures[p].clone();
                        let p = new_sig(
                            m,
                            SignatureData {
                                params: vec![Type::I32]
                                    .into_iter()
                                    .chain(p.params.into_iter())
                                    .collect(),
                                returns: p.returns.into_iter().collect(),
                            },
                        );
                        // let p = m.funcs.push(waffle::FuncDecl::Import(p, format!("_pit")));
                        if let ExportKind::Func(f) = &mut export.kind {
                            let s = m.funcs[*f].sig();
                            let p = *f;
                            let mut b = FunctionBody::new(&m, s);
                            let mut k = b.entry;
                            let args = b.blocks[b.entry]
                                .params
                                .iter()
                                .map(|a| a.1)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .zip(once(Arg::I32).chain(x.params.iter().cloned()));
                            let mut v2 = vec![];
                            for (v, r) in args {
                                let a;
                                (a, k) = shim(true, &mut b, k, &r, v, talloc, tfree, t)?;
                                v2.push(a);
                            }
                            let rets = add_op(
                                &mut b,
                                &v2,
                                &m.signatures[m.funcs[p].sig()].returns,
                                Operator::Call { function_index: p },
                            );
                            b.append_to_block(k, rets);
                            let rets = results_ref_2(&mut b, rets);
                            let mut r2 = vec![];
                            for (v, r) in rets.iter().cloned().zip(x.rets.iter()) {
                                let a;
                                (a, k) = shim(false, &mut b, k, r, v, talloc, tfree, t)?;
                                r2.push(a);
                            }
                            b.set_terminator(k, waffle::Terminator::Return { values: r2 });
                            *f = m.funcs.push(FuncDecl::Body(s, format!("_pit"), b));
                        }
                    }
                }
            }
            m.exports.push(export);
        }
    }
    m.exports.push(Export {
        name: format!("tpit_alloc"),
        kind: ExportKind::Func(talloc),
    });
    m.exports.push(Export {
        name: format!("tpit_free"),
        kind: ExportKind::Func(tfree),
    });
    m.exports.push(Export {
        name: format!("tpit_table"),
        kind: ExportKind::Table(t),
    });
    Ok(())
}
