use std::{
    collections::{BTreeMap, BTreeSet},
    iter::once,
    mem::take,
};

use anyhow::Context;
use waffle::{
    entity::EntityRef, util::new_sig, ExportKind, Func, FuncDecl, FunctionBody, HeapType, Import, ImportKind, Module, Operator, SignatureData, StorageType, TableData, Type, ValueDef, WithNullable
};
use waffle_ast::{
    add_op,
    fcopy::{obf_mod, DontObf, Obfuscate},
    Builder, Expr,
};

use crate::{canon::canon, util::talloc};
pub fn patch_ty(t: &mut Type) {
    if let Type::Heap(WithNullable { value: HeapType::ExternRef, nullable }) = t.clone() {
        *t = Type::I32
    }
}
pub struct LowerTables {}
impl Obfuscate for LowerTables {
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
                match module.tables[table_index].ty.clone() {
                    Type::I32 | Type::I64 | Type::F32 | Type::F64 => {
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
                        let id = format!(
                            "pit-patch-rt/@{}/{}",
                            module.tables[table_index].ty.to_string(),
                            o.to_string().split_once("<").unwrap().0
                        );
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
                    }
                    _ => DontObf {}.obf(o, f, b, args, types, module),
                }
            }
            _ => DontObf {}.obf(o, f, b, args, types, module),
        }
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
pub fn canon_all(m: &mut Module, cfg: &Cfg, root: &str) -> anyhow::Result<()>{
    let i = crate::get_interfaces(m)?;
    let interfaces = i
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    for j in interfaces.iter() {
        canon(m, &j.rid_str(), root)?;
    }
    if !cfg.unexportable_i32_tables {
        obf_mod(m, &mut LowerTables {})?;
    }
    Ok(())
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
        match s{
            SignatureData::Func { params, returns } => {
                for p in params.iter_mut().chain(returns.iter_mut()) {
                    patch_ty(p)
                }
            },
            SignatureData::Struct { fields } => {
                for f in fields.iter_mut(){
                    if let StorageType::Val(v) = &mut f.value{
                        patch_ty(v);
                    }
                }
            },
            SignatureData::Array { ty } =>{
                if let StorageType::Val(v) = &mut ty.value{
                    patch_ty(v);
                }
            },
            SignatureData::None => {

            },
            _ => todo!(),
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
                        if matches!(ty,Type::Heap(WithNullable { value: HeapType::ExternRef, nullable })) {
                            *o = Operator::I32Const { value: 0 }
                        }
                    }
                    Operator::RefIsNull => {
                        if matches!(b.type_pool[*tys][0] ,Type::Heap(WithNullable { value: HeapType::ExternRef, nullable })) {
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
    // if cfg.unexportable_i32_tables {
    for t in m.tables.values_mut() {
        if matches!(t.ty,Type::Heap(WithNullable { value: HeapType::ExternRef, nullable })) {
            t.ty = Type::I32;
        }
    }
    // } else {
    //     obf_mod(m, &mut X {})?;
    //     for t in m.tables.values_mut() {
    //         if t.ty == Type::ExternRef {
    //             t.ty = Type::FuncRef;
    //         }
    //     }
    // }

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
                    let tys = b.blocks[k].params.iter().map(|a| a.0).collect::<Vec<_>>();

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
                    values = vec![v];
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
        if i.module == "pit" {
            let n = &i.name;
            let fs = interfaces
                .iter()
                .filter_map(|i| {
                    let mut ex = m.exports.iter();
                    let ex = loop {
                        let Some(x) = ex.next() else {
                            return None;
                        };
                        if x.name != format!("pit/{}/{root}.{n}", i.rid_str()) {
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
                ty: Type::Heap(WithNullable{
                    nullable: true,
                    value: waffle::HeapType::FuncRef
                }),
                initial: fs.len() as u64,
                max: Some(fs.len() as u64),
                func_elements: Some(fs.clone()),
                table64: false,
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
        if i.module == "system" && i.name == "stub" {
            if let ImportKind::Func(f) = i.kind {
                let fsi = m.funcs[f].sig();
                let fname = m.funcs[f].name().to_owned();
                let mut b = FunctionBody::new(&m, fsi);
                let k = b.entry;
                let s = b.add_op(k, Operator::I32Const { value: 1 }, &[], &[Type::I32]);
                b.set_terminator(k, waffle::Terminator::Return { values: vec![s] });
                m.funcs[f] = FuncDecl::Body(fsi, fname, b);
                continue;
            }
        }
        m.imports.push(i);
    }
    if !cfg.unexportable_i32_tables {
        obf_mod(m, &mut LowerTables {})?;
    }
    Ok(())
}
