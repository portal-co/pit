use alloc::borrow::ToOwned;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::iter::once;
use core::mem::{replace, take};
use portal_pc_waffle::util::results_ref_2;
use portal_pc_waffle::{HeapType, WithMutablility};

use anyhow::Context;
use pit_core::Interface;
use portal_pc_waffle::{
    entity::EntityRef, util::new_sig, BlockTarget, Export, ExportKind, Func, FuncDecl,
    FunctionBody, Import, ImportKind, Module, Operator, SignatureData, TableData, Type,
    WithNullable,
};
use sha3::{Digest, Sha3_256};

use crate::tutils::{talloc, tfree};
// use waffle_ast::{results_ref_2, Builder, Expr};

// use crate::util::{talloc, tfree};

pub fn canon(m: &mut Module, rid: &str, target: &str) -> anyhow::Result<()> {
    let mut xs = vec![];
    for i in m.imports.iter() {
        if i.module == format!("pit/{rid}") {
            if let Some(a) = i.name.strip_prefix("~") {
                xs.push(a.to_owned())
            }
        }
    }
    xs.sort();
    let s = new_sig(
        m,
        SignatureData::Func {
            params: vec![Type::I32],
            returns: vec![Type::Heap(WithNullable {
                nullable: true,
                value: portal_pc_waffle::HeapType::ExternRef,
            })],
            shared: true,
        },
    );
    let f2 = m.funcs.push(portal_pc_waffle::FuncDecl::Import(
        s,
        format!("pit/{rid}.~{target}"),
    ));
    let mut tcache: BTreeMap<Vec<Type>, _> = BTreeMap::new();
    let tx = m.tables.push(TableData {
        ty: Type::Heap(WithNullable {
            nullable: true,
            value: portal_pc_waffle::HeapType::ExternRef,
        }),
        initial: 0,
        max: None,
        func_elements: None,
        table64: false,
    });
    // let mut tc2 = BTreeMap::new();
    let mut tc = |m: &mut Module, tys| {
        tcache
            .entry(tys)
            .or_insert_with_key(|tys| {
                let st = m.signatures.push(SignatureData::Struct {
                    fields: tys
                        .iter()
                        .cloned()
                        .map(|a| WithMutablility {
                            mutable: false,
                            value: portal_pc_waffle::StorageType::Val(a),
                        })
                        .collect(),
                    shared: true,
                });
                let stt = m.tables.push(TableData {
                    ty: Type::Heap(WithNullable {
                        value: portal_pc_waffle::HeapType::Sig { sig_index: st },
                        nullable: true,
                    }),
                    initial: 0,
                    max: None,
                    func_elements: None,
                    table64: false,
                });
                (
                    talloc(m, stt, &[]).unwrap(),
                    tfree(m, stt, &[]).unwrap(),
                    st,
                    stt,
                )
            })
            .clone()
    };
    let mut m2 = BTreeMap::new();
    let is = take(&mut m.imports);
    let stub = new_sig(
        m,
        SignatureData::Func {
            params: vec![],
            returns: vec![Type::Heap(WithNullable {
                nullable: true,
                value: portal_pc_waffle::HeapType::ExternRef,
            })],
            shared: true,
        },
    );
    let stub = m.funcs.push(FuncDecl::Import(stub, format!("stub")));
    m.imports.push(Import {
        module: format!("system"),
        name: format!("stub"),
        kind: ImportKind::Func(stub),
    });
    for i in is {
        if i.module == format!("pit/{rid}") {
            if let Some(a) = i.name.strip_prefix("~") {
                if let Ok(x) = xs.binary_search(&a.to_owned()) {
                    if let ImportKind::Func(f) = i.kind {
                        let fs = m.funcs[f].sig();
                        let fname = m.funcs[f].name().to_owned();
                        let mut b = FunctionBody::new(&m, fs);
                        let k = b.entry;
                        let (ta, _, ts, tts) =
                            tc(m, b.blocks[k].params.iter().map(|a| a.0).collect());
                        m2.insert(
                            a.to_owned(),
                            b.blocks[k].params.iter().map(|a| a.0).collect::<Vec<_>>(),
                        );
                        // let mut e = Expr::Bind(
                        //     Operator::I32Add,
                        //     vec![
                        //         Expr::Bind(Operator::I32Const { value: x as u32 }, vec![]),
                        //         Expr::Bind(
                        //             Operator::I32Mul,
                        //             vec![
                        //                 Expr::Bind(
                        //                     Operator::I32Const {
                        //                         value: xs.len() as u32,
                        //                     },
                        //                     vec![],
                        //                 ),
                        //                 if b.blocks[k].params.iter().map(|a| a.0).collect::<Vec<_>>()
                        //                     == vec![Type::I32]
                        //                 {
                        //                     Expr::Leaf(b.blocks[k].params[0].1)
                        //                 } else {
                        //                     Expr::Bind(
                        //                         Operator::Call { function_index: ta },
                        //                         once(Expr::Bind(
                        //                             Operator::Call {
                        //                                 function_index: stub,
                        //                             },
                        //                             vec![],
                        //                         ))
                        //                         .chain(
                        //                             b.blocks[k]
                        //                                 .params
                        //                                 .iter()
                        //                                 .map(|p| Expr::Leaf(p.1)),
                        //                         )
                        //                         .collect(),
                        //                     )
                        //                 },
                        //             ],
                        //         ),
                        //     ],
                        // );
                        // let (a, k) = e.build(m, &mut b, k)?;
                        let a = {
                            let a = b.add_op(
                                k,
                                Operator::I32Const {
                                    value: xs.len() as u32,
                                },
                                &[],
                                &[Type::I32],
                            );
                            let v = if b.blocks[k].params.iter().map(|a| a.0).collect::<Vec<_>>()
                                == vec![Type::I32]
                            {
                                b.blocks[k].params[0].1
                            } else {
                                let a = b.add_op(
                                    k,
                                    Operator::StructNew { sig: ts },
                                    &b.blocks[k].params.iter().map(|a| a.1).collect::<Vec<_>>(),
                                    &[Type::Heap(WithNullable {
                                        value: HeapType::Sig { sig_index: ts },
                                        nullable: true,
                                    })],
                                );
                                b.add_op(k, Operator::Call { function_index: ta }, &[a], &[Type::I32])
                            };
                            let a = b.add_op(k, Operator::I32Mul, &[v, a], &[Type::I32]);
                            let c = b.add_op(
                                k,
                                Operator::I32Const { value: x as u32 },
                                &[],
                                &[Type::I32],
                            );
                            b.add_op(k, Operator::I32Add, &[a, c], &[Type::I32])
                        };
                        let args = once(a)
                            .chain(b.blocks[b.entry].params[1..].iter().map(|a| a.1))
                            .collect();
                        b.set_terminator(
                            k,
                            portal_pc_waffle::Terminator::ReturnCall { func: f2, args },
                        );
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
                let (e, _) = b
                    .entry(x2.clone())
                    .or_insert_with(|| (Func::invalid(), m2.get(a).cloned().unwrap()));
                if let ExportKind::Func(f) = x.kind {
                    *e = f;
                    continue;
                }
            }
        }
        m.exports.push(x)
    }
    for (method, inner) in b.into_iter() {
        let a = inner
            .iter()
            .filter(|a| a.1 .0.is_valid())
            .next()
            .context("in getting an instance")?
            .1;
        let sig = a.0;
        let funcs: Vec<_> = xs
            .iter()
            .map(|f| inner.get(f).cloned().unwrap_or_default())
            .collect::<Vec<_>>();
        let sig = m.funcs[sig].sig();
        let mut sig = m.signatures[sig].clone();
        if let SignatureData::Func {
            params, returns, ..
        } = &mut sig
        {
            *params = once(Type::I32)
                .chain(params[a.1.len()..].iter().cloned())
                .collect::<Vec<_>>();
        }
        let sig = new_sig(m, sig);
        let mut b = FunctionBody::new(&m, sig);
        let k = b.entry;
        let fl = b.add_op(
            k,
            Operator::I32Const {
                value: funcs.len() as u32,
            },
            &[],
            &[Type::I32],
        );
        // let mut e = Expr::Bind(
        //     Operator::I32DivU,
        //     vec![
        //         Expr::Leaf(b.blocks[k].params[0].1),
        //         Expr::Bind(
        //             ,
        //             vec![],
        //         ),
        //     ],
        // );
        // let (a, k) = e.build(m, &mut b, k)?;
        let a = b.add_op(
            k,
            Operator::I32DivU,
            &[b.blocks[k].params[0].1, fl],
            &[Type::I32],
        );
        // let mut e = Expr::Bind(
        //     Operator::I32RemU,
        //     vec![
        //         Expr::Leaf(b.blocks[k].params[0].1),
        //         Expr::Bind(
        //             Operator::I32Const {
        //                 value: funcs.len() as u32,
        //             },
        //             vec![],
        //         ),
        //     ],
        // );
        // let (c, k) = e.build(m, &mut b, k)?;
        let c = b.add_op(
            k,
            Operator::I32RemU,
            &[b.blocks[k].params[0].1, fl],
            &[Type::I32],
        );
        let args = b.blocks[b.entry].params[1..]
            .iter()
            .map(|a| a.1)
            .collect::<Vec<_>>();
        let blocks = funcs
            .iter()
            .map(|(f, t)| {
                let args = args.clone();
                let k = b.add_block();
                if f.is_invalid() {
                    let rets = b
                        .rets
                        .clone()
                        .into_iter()
                        .map(|t| match t.clone() {
                            Type::I32 => b.add_op(k, Operator::I32Const { value: 0 }, &[], &[t]),
                            Type::I64 => b.add_op(k, Operator::I64Const { value: 0 }, &[], &[t]),
                            Type::F32 => b.add_op(k, Operator::F32Const { value: 0 }, &[], &[t]),
                            Type::F64 => b.add_op(k, Operator::F64Const { value: 0 }, &[], &[t]),
                            Type::V128 => todo!(),
                            Type::Heap(_) => {
                                b.add_op(k, Operator::RefNull { ty: t.clone() }, &[], &[t])
                            }
                            _ => todo!(),
                        })
                        .collect();
                    b.set_terminator(k, portal_pc_waffle::Terminator::Return { values: rets });
                } else if *t == vec![Type::I32] {
                    b.set_terminator(
                        k,
                        portal_pc_waffle::Terminator::ReturnCall {
                            func: *f,
                            args: once(a).chain(args.into_iter()).collect(),
                        },
                    );
                } else {
                    let (_, tf, ts, tts) = tc(m, t.clone());
                    let real = if method == ".drop" {
                        let c = b.add_op(k, Operator::Call { function_index: tf }, &[a], &t);
                        t.iter()
                            .cloned()
                            // .zip(ts.into_iter())
                            .enumerate()
                            .map(|(w, u)| {
                                b.add_op(k, Operator::StructGet { sig: ts, idx: w }, &[a], &[u])
                            })
                            .collect::<Vec<_>>()
                    } else {
                        let a = b.add_op(
                            k,
                            Operator::TableGet { table_index: tts },
                            &[a],
                            &[Type::Heap(WithNullable {
                                value: portal_pc_waffle::HeapType::Sig { sig_index: ts },
                                nullable: true,
                            })],
                        );
                        t.iter()
                            .cloned()
                            // .zip(ts.into_iter())
                            .enumerate()
                            .map(|(w, u)| {
                                b.add_op(k, Operator::StructGet { sig: ts, idx: w }, &[a], &[u])
                            })
                            .collect()
                    };
                    b.set_terminator(
                        k,
                        portal_pc_waffle::Terminator::ReturnCall {
                            func: *f,
                            args: real.into_iter().chain(args.into_iter()).collect(),
                        },
                    );
                    // anyhow::bail!("invalid type");
                }
                Ok(BlockTarget {
                    block: k,
                    args: vec![],
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        b.set_terminator(
            k,
            portal_pc_waffle::Terminator::Select {
                value: c,
                targets: blocks,
                default: BlockTarget {
                    block: b.entry,
                    args,
                },
            },
        );
        let f = m.funcs.push(FuncDecl::Body(
            sig,
            format!("pit/{rid}/~{target}{method}"),
            b,
        ));
        m.exports.push(Export {
            name: format!("pit/{rid}/~{target}{method}"),
            kind: ExportKind::Func(f),
        });
    }
    Ok(())
}
pub fn jigger(m: &mut Module, seed: &[u8]) -> anyhow::Result<()> {
    let mut s = Sha3_256::default();
    s.update(&m.to_wasm_bytes()?);
    s.update(seed);
    let s = s.finalize();
    for i in m.imports.iter_mut() {
        if !i.module.starts_with("pit/") {
            continue;
        }
        if let Some(a) = i.name.strip_prefix("~") {
            let a = format!("{a}-{s:?}");
            let mut s = Sha3_256::default();
            s.update(a.as_bytes());
            let s = s.finalize();
            let s = hex::encode(s);
            i.name = format!("~{s}");
        }
    }
    for x in m.exports.iter_mut() {
        if let Some(a) = x.name.strip_prefix("pit/") {
            if let Some((b, a)) = a.split_once("/~") {
                if let Some((a, c)) = a.split_once("/") {
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
