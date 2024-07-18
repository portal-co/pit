use std::{collections::BTreeMap, iter::once, mem::take};

use anyhow::Context;
use sha3::{Digest, Sha3_256};
use waffle::{
    entity::EntityRef, util::new_sig, BlockTarget, Export, ExportKind, Func, FuncDecl,
    FunctionBody, Import, ImportKind, Module, Operator, SignatureData, Type,
};
use waffle_ast::{Builder, Expr};

pub fn canon(m: &mut Module, rid: &str, target: &str) -> anyhow::Result<()> {
    let mut xs = vec![];
    for i in m.imports.iter() {
        if i.module == format!("pit/{rid}") {
            if let Some(a) = i.name.strip_prefix("~") {
                xs.push(a.to_owned())
            }
        }
    }
    let s = new_sig(
        m,
        SignatureData {
            params: vec![Type::I32],
            returns: vec![Type::ExternRef],
        },
    );
    let f2 = m
        .funcs
        .push(waffle::FuncDecl::Import(s, format!("pit/{rid}.~{target}")));
    for i in take(&mut m.imports) {
        if i.module == format!("pit/{rid}") {
            if let Some(a) = i.name.strip_prefix("~") {
                if let Ok(x) = xs.binary_search(&a.to_owned()) {
                    if let ImportKind::Func(f) = i.kind {
                        let fs = m.funcs[f].sig();
                        let fname = m.funcs[f].name().to_owned();
                        let mut b = FunctionBody::new(&m, fs);
                        let k = b.entry;
                        let mut e = Expr::Bind(
                            Operator::I32Add,
                            vec![
                                Expr::Bind(Operator::I32Const { value: x as u32 }, vec![]),
                                Expr::Bind(
                                    Operator::I32Mul,
                                    vec![
                                        Expr::Bind(
                                            Operator::I32Const {
                                                value: xs.len() as u32,
                                            },
                                            vec![],
                                        ),
                                        Expr::Leaf(b.blocks[k].params[0].1),
                                    ],
                                ),
                            ],
                        );
                        let (a, k) = e.build(m, &mut b, k)?;
                        let args = once(a)
                            .chain(b.blocks[b.entry].params[1..].iter().map(|a| a.1))
                            .collect();
                        b.set_terminator(k, waffle::Terminator::ReturnCall { func: f2, args });
                        m.funcs[f] = FuncDecl::Body(fs, fname, b);
                        continue;
                    }
                }
            }
        }
        m.imports.push(i)
    }
    m.imports.push(Import {
        module: format!("pit/{rid}"),
        name: format!("~{target}"),
        kind: ImportKind::Func(f2),
    });
    let mut b = BTreeMap::new();
    for x in take(&mut m.exports) {
        for x2 in xs.iter() {
            if let Some(a) = x.name.strip_prefix(&format!("pit/{rid}/~{x2}")) {
                let mut b = b.entry(a.to_owned()).or_insert_with(|| BTreeMap::new());
                let mut e = b.entry(x2.clone()).or_insert_with(|| Func::invalid());
                if let ExportKind::Func(f) = x.kind {
                    *e = f;
                    continue;
                }
            }
        }
        m.exports.push(x)
    }
    for (method, inner) in b.into_iter() {
        let sig = *inner.iter().next().context("in getting an instance")?.1;
        let funcs: Vec<Func> = xs
            .iter()
            .filter_map(|f| inner.get(f))
            .cloned()
            .collect::<Vec<_>>();
        let sig = m.funcs[sig].sig();
        let mut b = FunctionBody::new(&m, sig);
        let k = b.entry;
        let mut e = Expr::Bind(
            Operator::I32DivU,
            vec![
                Expr::Leaf(b.blocks[k].params[0].1),
                Expr::Bind(
                    Operator::I32Const {
                        value: funcs.len() as u32,
                    },
                    vec![],
                ),
            ],
        );
        let (a, k) = e.build(m, &mut b, k)?;
        let mut e = Expr::Bind(
            Operator::I32RemU,
            vec![
                Expr::Leaf(b.blocks[k].params[0].1),
                Expr::Bind(
                    Operator::I32Const {
                        value: funcs.len() as u32,
                    },
                    vec![],
                ),
            ],
        );
        let (c, k) = e.build(m, &mut b, k)?;
        let args = once(a)
            .chain(b.blocks[b.entry].params[1..].iter().map(|a| a.1))
            .collect::<Vec<_>>();
        let blocks = funcs
            .iter()
            .map(|f| {
                let k = b.add_block();
                b.set_terminator(
                    k,
                    waffle::Terminator::ReturnCall {
                        func: *f,
                        args: args.clone(),
                    },
                );
                BlockTarget {
                    block: k,
                    args: vec![],
                }
            })
            .collect::<Vec<_>>();
        b.set_terminator(
            k,
            waffle::Terminator::Select {
                value: c,
                targets: blocks,
                default: BlockTarget {
                    block: b.entry,
                    args,
                },
            },
        );
        let f = m
            .funcs
            .push(FuncDecl::Body(sig, format!("pit/{rid}/~{target}{method}"), b));
        m.exports.push(Export {
            name: format!("pit/{rid}/~{target}{method}"),
            kind: ExportKind::Func(f),
        });
    }
    Ok(())
}
pub fn jigger(m: &mut Module) -> anyhow::Result<()>{
    let mut s = Sha3_256::default();
    s.update(&m.to_wasm_bytes()?);
    let s = s.finalize();
    for i in m.imports.iter_mut(){
        if !i.module.starts_with("pit/"){
            continue;
        }
        if let Some(a) = i.name.strip_prefix("~"){
            let a = format!("{a}-{s:?}");
            let mut s = Sha3_256::default();
            s.update(a.as_bytes());
            let s = s.finalize();
            let s = hex::encode(s);
            i.name = format!("~{s}");
        }
    }
    for x in m.exports.iter_mut(){
        if let Some(a) = x.name.strip_prefix("pit/"){
            if let Some((b,a)) = a.split_once("/~"){
                if let Some((a,c)) = a.split_once("/"){
                    let a = format!("{a}-{s:?}");
                    let mut s = Sha3_256::default();
                    s.update(a.as_bytes());
                    let s = s.finalize();
                    let s = hex::encode(s);
                    x.name = format!("pit/{b}/~{s}/{c}");
                }
            }
        }
    }
    Ok(())
}