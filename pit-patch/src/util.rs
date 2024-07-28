use std::collections::BTreeMap;

use waffle::{
    util::new_sig, BlockTarget, Func, FunctionBody, Import, ImportKind, Module, Operator,
    SignatureData, Table, Type,
};
use waffle_ast::{add_op, Builder, Expr};

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
