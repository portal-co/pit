use std::{collections::BTreeSet, iter::once, mem::take};

use anyhow::Context;
use waffle::{
    entity::EntityRef, util::new_sig, ExportKind, Func, FuncDecl, FunctionBody, Import, ImportKind,
    Module, Operator, SignatureData, TableData, Type, ValueDef,
};
use waffle_ast::{
    add_op,
    fcopy::{obf_mod, DontObf, Obfuscate},
    Builder, Expr,
};

use crate::canon::canon;
pub fn patch_ty(t: &mut Type) {
    if let Type::ExternRef = t.clone() {
        *t = Type::I32
    }
}
pub fn import_fn(m: &mut Module, mo: String, n: String, s: SignatureData) -> Func {
    for i in m.imports.iter() {
        if i.module == mo && i.name == n {
            if let ImportKind::Func(f) = &i.kind {
                return *f;
            }
        }
    }
    let s = new_sig(m, s);
    let f = m
        .funcs
        .push(waffle::FuncDecl::Import(s, format!("{mo}.{n}")));
    m.imports.push(Import {
        module: mo,
        name: n,
        kind: ImportKind::Func(f),
    });
    return f;
}
pub struct Cfg {
    pub unexportable_i32_tables: bool,
}
pub fn instantiate(m: &mut Module, cfg: &Cfg) -> anyhow::Result<()> {
    let root = "pit_patch_internal_instantiate";
    let i = crate::get_interfaces(m)?;
    let interfaces = i
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    for j in interfaces.iter() {
        canon(m, &j.rid_str(), root)?;
    }
    for s in m.signatures.values_mut() {
        for p in s.params.iter_mut().chain(s.returns.iter_mut()) {
            patch_ty(p)
        }
    }
    m.try_take_per_func_body(|m, b| {
        let x = b.type_pool.from_iter(once(Type::I32));
        for p in b
            .blocks
            .values_mut()
            .flat_map(|a| a.params.iter_mut().map(|a| &mut a.0))
            .chain(b.rets.iter_mut())
        {
            patch_ty(p)
        }
        for v in b.values.iter().collect::<Vec<_>>() {
            if let ValueDef::Operator(o, _1, tys) = &mut b.values[v] {
                match o.clone() {
                    Operator::RefNull { ty } => {
                        if ty == Type::ExternRef {
                            *o = Operator::I32Const { value: 0 }
                        }
                    }
                    Operator::RefIsNull => {
                        if b.type_pool[*tys][0] == Type::ExternRef {
                            *o = Operator::I32Eqz
                        }
                    }
                    _ => {}
                }
            }
        }
        for t in b.type_pool.storage.iter_mut() {
            patch_ty(t)
        }
        Ok::<_, anyhow::Error>(())
    })?;
    struct X {}
    impl Obfuscate for X {
        fn obf(
            &mut self,
            o: Operator,
            f: &mut waffle::FunctionBody,
            b: waffle::Block,
            args: &[waffle::Value],
            types: &[Type],
            module: &mut Module,
        ) -> anyhow::Result<(waffle::Value, waffle::Block)> {
            match o {
                Operator::TableGet { table_index }
                | Operator::TableSet { table_index }
                | Operator::TableSize { table_index }
                | Operator::TableGrow { table_index } => {
                    if module.tables[table_index].ty == Type::ExternRef {
                        // let x = b.arg_pool[*_1].to_vec();
                        let w = add_op(
                            f,
                            &[],
                            &[Type::I32],
                            Operator::I32Const {
                                value: table_index.index() as u32,
                            },
                        );
                        f.append_to_block(b, w);
                        let id =
                            format!("pit-patch-rt/{}", o.to_string().split_once("<").unwrap().0);
                        let mut a = module.exports.iter();
                        let a = loop {
                            let Some(b) = a.next() else {
                                anyhow::bail!("pit patch rt not found")
                            };
                            if b.name != id {
                                continue;
                            }
                            let ExportKind::Func(a) = &b.kind else {
                                continue;
                            };
                            break *a;
                        };
                        DontObf {}.obf(
                            Operator::Call { function_index: a },
                            f,
                            b,
                            &once(w).chain(args.iter().cloned()).collect::<Vec<_>>(),
                            types,
                            module,
                        )
                    } else {
                        DontObf {}.obf(o, f, b, args, types, module)
                    }
                }
                _ => DontObf {}.obf(o, f, b, args, types, module),
            }
        }
    }
    if cfg.unexportable_i32_tables {
        for t in m.tables.values_mut() {
            if t.ty == Type::ExternRef {
                t.ty = Type::I32;
            }
        }
    } else {
        obf_mod(m, &mut X {})?;
        for t in m.tables.values_mut() {
            if t.ty == Type::ExternRef {
                t.ty = Type::FuncRef;
            }
        }
    }
    for i in take(&mut m.imports) {
        if let Some(rid) = i.module.strip_prefix("pit/") {
            let ridx = interfaces
                .iter()
                .enumerate()
                .find_map(|x| {
                    if x.1.rid_str() == rid {
                        Some(x.0)
                    } else {
                        None
                    }
                })
                .context("in getting the index")?;
            let rl = interfaces.len();
            if i.name.ends_with(&format!("~{root}")) {
                if let ImportKind::Func(f) = i.kind {
                    let fs = m.funcs[f].sig();
                    let fname = m.funcs[f].name().to_owned();
                    let mut b = FunctionBody::new(&m, fs);
                    let k = b.entry;
                    let mut values: Vec<waffle::Value> =
                        b.blocks[k].params.iter().map(|a| a.1).collect();
                    let mut e = Expr::Bind(
                        Operator::I32Add,
                        vec![
                            Expr::Bind(Operator::I32Const { value: ridx as u32 }, vec![]),
                            Expr::Bind(
                                Operator::I32Mul,
                                vec![
                                    Expr::Bind(Operator::I32Const { value: rl as u32 }, vec![]),
                                    Expr::Leaf(values[0]),
                                ],
                            ),
                        ],
                    );
                    let (v, k) = e.build(m, &mut b, k)?;
                    values[0] = v;
                    b.set_terminator(k, waffle::Terminator::Return { values });
                    m.funcs[f] = FuncDecl::Body(fs, fname, b);
                    continue;
                }
            } else {
                let ek = format!("{}/~{root}{}", i.module, i.name);
                let mut ex = m.exports.iter();
                let ex = loop {
                    let Some(x) = ex.next() else {
                        anyhow::bail!("export not found")
                    };
                    if x.name != ek {
                        continue;
                    };
                    let ExportKind::Func(ef) = &x.kind else {
                        continue;
                    };
                    break *ef;
                };
                if let ImportKind::Func(f) = i.kind {
                    let fs = m.funcs[f].sig();
                    let fname = m.funcs[f].name().to_owned();
                    let mut b = FunctionBody::new(&m, fs);
                    let k = b.entry;
                    let mut args = b.blocks[b.entry]
                        .params
                        .iter()
                        .map(|a| a.1)
                        .collect::<Vec<_>>();
                    let mut e = Expr::Bind(
                        Operator::I32DivU,
                        vec![
                            Expr::Leaf(args[0]),
                            Expr::Bind(Operator::I32Const { value: rl as u32 }, vec![]),
                        ],
                    );
                    let (v, k) = e.build(m, &mut b, k)?;
                    args[0] = v;
                    b.set_terminator(k, waffle::Terminator::ReturnCall { func: ex, args });
                    m.funcs[f] = FuncDecl::Body(fs, fname, b);
                    continue;
                }
            }
        }
        if i.module == "pit" && i.name == "drop" {
            let fs = interfaces
                .iter()
                .filter_map(|i| {
                    let mut ex = m.exports.iter();
                    let ex = loop {
                        let Some(x) = ex.next() else {
                            return None;
                        };
                        if x.name != format!("pit/{}/{root}.drop", i.rid_str()) {
                            continue;
                        };
                        let ExportKind::Func(ef) = &x.kind else {
                            continue;
                        };
                        break *ef;
                    };
                    return Some(ex);
                })
                .collect::<Vec<_>>();
            let t = m.tables.push(TableData {
                ty: Type::FuncRef,
                initial: fs.len() as u32,
                max: Some(fs.len() as u32),
                func_elements: Some(fs.clone()),
            });
            if let ImportKind::Func(f) = i.kind {
                let fsi = m.funcs[f].sig();
                let fname = m.funcs[f].name().to_owned();
                let mut b = FunctionBody::new(&m, fsi);
                let k = b.entry;
                let mut args = b.blocks[b.entry]
                    .params
                    .iter()
                    .map(|a| a.1)
                    .collect::<Vec<_>>();
                let mut e = Expr::Bind(
                    Operator::I32DivU,
                    vec![
                        Expr::Leaf(b.blocks[k].params[0].1),
                        Expr::Bind(
                            Operator::I32Const {
                                value: fs.len() as u32,
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
                                value: fs.len() as u32,
                            },
                            vec![],
                        ),
                    ],
                );
                let (c, k) = e.build(m, &mut b, k)?;
                args[0] = a;
                args.push(c);
                b.set_terminator(
                    k,
                    waffle::Terminator::ReturnCallIndirect {
                        sig: fsi,
                        table: t,
                        args,
                    },
                );
                m.funcs[f] = FuncDecl::Body(fsi, fname, b);
                continue;
            }
        }
        m.imports.push(i);
    }
    Ok(())
}
