use crate::util::add_op;
use alloc::vec::Vec;
use alloc::{boxed::Box, vec};
use core::iter::once;
use portal_pc_waffle::op_traits::op_outputs;
use portal_pc_waffle::{
    util::new_sig, Block, BlockTarget, Func, FunctionBody, Module, Operator, Table, Value,
};
use portal_pc_waffle::{SignatureData, Type};
pub trait Builder {
    type Result;
    fn build(
        &mut self,
        mo: &mut Module,
        func: &mut FunctionBody,
        k: Block,
    ) -> anyhow::Result<(Self::Result, Block)>;
}
// use crate::*;
pub enum Expr {
    Leaf(Value),
    Bind(Operator, Vec<Expr>),
    Mount(Box<dyn Builder<Result = portal_pc_waffle::Value>>),
}
impl Builder for Expr {
    type Result = portal_pc_waffle::Value;
    fn build(
        &mut self,
        mo: &mut portal_pc_waffle::Module,
        func: &mut portal_pc_waffle::FunctionBody,
        mut k: portal_pc_waffle::Block,
    ) -> anyhow::Result<(Self::Result, portal_pc_waffle::Block)> {
        match self {
            Expr::Leaf(a) => Ok((*a, k)),
            Expr::Bind(a, c) => {
                let mut r = vec![];
                for d in c.iter_mut() {
                    let (e, f) = d.build(mo, func, k)?;
                    k = f;
                    r.push(e);
                }
                let o = add_op(func, &r, &op_outputs(&mo, None, &a)?, a.clone());
                func.append_to_block(k, o);
                return Ok((o, k));
            }
            Expr::Mount(m) => m.build(mo, func, k),
        }
    }
}
pub fn talloc(m: &mut Module, t: Table, aux: &[Table]) -> anyhow::Result<Func> {
    let e = m.tables[t].ty.clone();
    let atys = aux
        .iter()
        .map(|a| m.tables[*a].ty.clone())
        .collect::<Vec<_>>();
    let sig = new_sig(
        m,
        SignatureData::Func {
            params: vec![e].into_iter().chain(atys.into_iter()).collect(),
            returns: vec![Type::I32],
            shared: true,
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
        portal_pc_waffle::Terminator::Br {
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
        portal_pc_waffle::Terminator::CondBr {
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
        portal_pc_waffle::Terminator::CondBr {
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
            portal_pc_waffle::Terminator::CondBr {
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
            portal_pc_waffle::Terminator::Br {
                target: BlockTarget {
                    block: d,
                    args: vec![],
                },
            },
        );
        o = d;
        f.add_op(o, Operator::TableSet { table_index: *a }, &[idx, *v], &[]);
    }
    f.set_terminator(
        o,
        portal_pc_waffle::Terminator::Return { values: vec![idx] },
    );
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
            portal_pc_waffle::Terminator::CondBr {
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
            portal_pc_waffle::Terminator::Br {
                target: BlockTarget {
                    block: d,
                    args: vec![],
                },
            },
        );
        o = d;
        f.add_op(o, Operator::TableSet { table_index: *a }, &[idx, *v], &[]);
    }
    f.set_terminator(
        o,
        portal_pc_waffle::Terminator::Return { values: vec![idx] },
    );
    return Ok(m.funcs.push(portal_pc_waffle::FuncDecl::Body(
        sig,
        alloc::format!("talloc"),
        f,
    )));
}
pub fn tfree(m: &mut Module, t: Table, aux: &[Table]) -> anyhow::Result<Func> {
    let ety = m.tables[t].ty.clone();
    let atys = aux
        .iter()
        .map(|a| m.tables[*a].ty.clone())
        .collect::<Vec<_>>();
    let sig = new_sig(
        m,
        SignatureData::Func {
            params: vec![Type::I32],
            returns: vec![ety.clone()]
                .into_iter()
                .chain(atys.into_iter())
                .collect(),
            shared: true,
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
    f.set_terminator(o, portal_pc_waffle::Terminator::Return { values: r });
    return Ok(m.funcs.push(portal_pc_waffle::FuncDecl::Body(
        sig,
        alloc::format!("tfree"),
        f,
    )));
}
