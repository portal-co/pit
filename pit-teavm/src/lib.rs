use std::{
    collections::{BTreeMap, BTreeSet},
    iter::once,
};

use itertools::Itertools;
use pit_core::{Arg, Interface, ResTy, Sig};

pub fn emit(i: &Interface, pkg: &str, binders: &Binders) -> String {
    let generics: u32 = i
        .ann
        .iter()
        .find_map(|a| {
            if a.name == "generics" {
                Some(a.value.clone())
            } else {
                None
            }
        })
        .map(|a| a.parse().unwrap())
        .unwrap_or_default();
    let mut genericsStr = (0..generics)
        .map(|a| format!("T{a}: Handler"))
        .collect::<Vec<_>>()
        .join(",");
    let mut genericsRStr = (0..generics)
        .map(|a| format!("T{a}"))
        .collect::<Vec<_>>()
        .join(",");
    if genericsStr != "" {
        genericsStr = format!("[{}]", genericsStr);
    }
    if genericsRStr != "" {
        genericsRStr = format!("[{}]", genericsRStr);
    }
    let qs = if generics == 0 {
        "".to_owned()
    } else {
        format!("[{}]", ",?".repeat(generics as usize).split_off(1))
    };
    let bs = binders
        .iter()
        .map(|((name, pkg, exp), (a, cfg))| {
            let generics2: Option<u32> = a
                .ann
                .iter()
                .find_map(|a| {
                    if a.name == "generics" {
                        Some(a.value.clone())
                    } else {
                        None
                    }
                })
                .map(|a| a.parse().unwrap());
            let generics2 = generics2.unwrap_or_default();
            let mut generics2Str = (0..generics2)
                .map(|a| a + generics)
                .map(|a| format!("T{a}: Handler"))
                .collect::<Vec<_>>()
                .join(",");
            let mut generics2RStr = (0..generics2)
                .map(|a| a + generics)
                .map(|a| format!("T{a}"))
                .collect::<Vec<_>>()
                .join(",");
            if generics2Str != "" {
                generics2Str = format!("[{}]", generics2Str);
            }
            if generics2RStr != "" {
                generics2RStr = format!("[{}]", generics2RStr);
            }
            let mut n = cfg
                .split(",")
                .map(|i2| {
                    let (is2, instance_renders) = do_instances(
                        &i2.split(",").collect::<Vec<_>>(),
                        i.rid_str().as_str(),
                        generics,
                    );
                    is2[0].clone()
                })
                .chain(once(format!("{}{genericsRStr}", i.rid_str())))
                .join(",");
            n = format!("[{n}]");
            let rns = a
            .rets
            .iter()
            .enumerate()
            .map(|(a, _)| format!("r{a}"))
            .collect::<Vec<_>>()
            .join(",");
        let rvs = a
            .rets
            .iter()
            .enumerate()
            .map(|(a, b)| {
                let mut c = format!("r{a}");
                if let Arg::Resource {
                    ty,
                    nullable,
                    take,
                    ann,
                } = b
                {
                    match ty {
                        // ResTy::None  if !ann.iter().any(|a|a.name == "instance") => {}
                        _ => {
                            // let rid = match ty {
                            //     ResTy::None => todo!(),
                            //     ResTy::Of(a) => *a,
                            //     ResTy::This => i.rid(),
                            // };
                            // let rid = hex::encode(rid);
                            let d = emit_ty(
                                b,
                                i.rid_str().as_str(),
                                &FFIStatus::HighLevel { generics },
                            );
                            c = format!("summon[Handler[{d}]].fromHandle({c})");
                        }
                    }
                }
                c
            })
            .collect::<Vec<_>>()
            .join(",");
        let s = format!(
            r#"
    given {name}{genericsStr}: {pkg}.{name}{n} = new {pkg}.{name}{n}{{
        def {name}{} = {{
            @Import(name=".{name}@{}",module="tpit") @native def go {};
            return go(me.handle,{}) match{{
                case ({rns}) => ({rvs})
            }};
        }};
    }};
    "#,
            emit_sig(a, None, "me: Impl,", &FFIStatus::HighLevel { generics: 0 }),
            a.to_string(),
            emit_sig(a, None, "handle: Int,", &FFIStatus::FFI),
            a.params
                .iter()
                .enumerate()
                .map(|(a, b)| {
                    let mut c = format!("_{a}");
                    if let Arg::Resource {
                        ty,
                        nullable,
                        take,
                        ann,
                    } = b
                    {
                        match ty {
                            // ResTy::None if !ann.iter().any(|a|a.name == "instance")=> {}
                            _ => {
                                // let rid = match ty {
                                //     ResTy::None => todo!(),
                                //     ResTy::Of(a) => *a,
                                //     ResTy::This => i.rid(),
                                // };
                                // let rid = hex::encode(rid);
                                let d = emit_ty(
                                    b,
                                    i.rid_str().as_str(),
                                    &FFIStatus::HighLevel { generics },
                                );
                                c = format!("summon[Handler[{d}]].handleOf({c})")
                            }
                        }
                    }
                    c
                })
                .collect::<Vec<_>>()
                .join(",")
        );
            match exp {
                Exposition::Import => {
                    return s;
                }
                Exposition::Expose => {
                    let rns = a
                        .rets
                        .iter()
                        .enumerate()
                        .map(|(a, _)| format!("r{a}"))
                        .collect::<Vec<_>>()
                        .join(",");
                    let pvs = a
                        .params
                        .iter()
                        .enumerate()
                        .map(|(a, b)| {
                            let mut c = format!("_{a}");
                            if let Arg::Resource {
                                ty,
                                nullable,
                                take,
                                ann,
                            } = b
                            {
                                match ty {
                                    // ResTy::None  if !ann.iter().any(|a|a.name == "instance") => {}
                                    _ => {
                                        // let rid = match ty {
                                        //     ResTy::None => todo!(),
                                        //     ResTy::Of(a) => *a,
                                        //     ResTy::This => i.rid(),
                                        // };
                                        // let rid = hex::encode(rid);
                                        let d = emit_ty(
                                            b,
                                            i.rid_str().as_str(),
                                            &FFIStatus::HighLevel { generics },
                                        );
                                        c = format!("summon[Handler[{d}]].fromHandle({c})");
                                    }
                                }
                            }
                            c
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    let rvs = a
                        .rets
                        .iter()
                        .enumerate()
                        .map(|(a, b)| {
                            let mut c = format!("r{a}");
                            if let Arg::Resource {
                                ty,
                                nullable,
                                take,
                                ann,
                            } = b
                            {
                                match ty {
                                    // ResTy::None  if !ann.iter().any(|a|a.name == "instance") => {}
                                    _ => {
                                        // let rid = match ty {
                                        //     ResTy::None => todo!(),
                                        //     ResTy::Of(a) => *a,
                                        //     ResTy::This => i.rid(),
                                        // };
                                        // let rid = hex::encode(rid);
                                        let d = emit_ty(
                                            b,
                                            i.rid_str().as_str(),
                                            &FFIStatus::HighLevel { generics },
                                        );
                                        c = format!("summon[Handler[{d}]].handleOf({c}")
                                    }
                                }
                            }
                            c
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(
                        r#"
                    @Export(name = "tpit/{}/.{name}@{}") def go{} = {{
                        val z = all.get(handle);
                        return summon[{pkg}.{name}Impl[{n}]].{name}({pvs}) match {{
                            case ({rns}) => ({rvs})
                        }};
                    }};
                    {s}
                "#,
                        i.rid_str(),
                        a.to_string(),
                        emit_sig(a, None, "handle: Int,", &FFIStatus::FFI),
                    )
                }
            }
        })
        .join(";");
    format!(
        r#"
        package {pkg};
        import scala.collection.mutable.{{Map,HashMap}};
        import org.teavm.interop.{{Import,Export}};
        import pc.portal.pit.guest.{{Handler,drop}};
        trait R{}{genericsStr}{{
            {}
            def finalize: Unit
        }}
        object R{}{{
            given handler{genericsStr} = new Handler[R{}{genericsRStr}]{{
                type Impl = Impl{genericsRStr}
                def createImpl(a: R{}{genericsRStr}): Impl{genericsRStr} = new Impl(a);
                def handleOf(a: Impl{genericsRStr}): Int = a.handle;
                def fromHandle(a: Int): Impl{genericsRStr} = new Impl(a);
                def finalize(a: R{}{genericsRStr}): Unit = a.finalize;
            }};
            {bs}
            object Impl{{
                val all: Map[Int,R{}{qs}] = HashMap.empty;
                def create(r: R{}{qs}): Int = {{
                    var x = 0
                    while all.contains(x){{
                        x = x + 1   
                    }}
                    all.addOne((x,r))
                    return x
                }}
                {}
            }}
            class Impl{genericsStr}(val handle: Int) extends R{}{genericsRStr}{{
                import Impl.{{all,create}};
                def finalize: Unit = {{
                    drop(handle);
                }}
                {}
                def this(res: R{}{genericsRStr}) = {{
                    @Import(name="~{pkg}",module="tpit/{}") @native def go(a: Int): Int
                    this(go(create(res)))
                }}
            }};
        }}"#,
        i.rid_str(),
        i.methods
            .iter()
            .map(|(a, b)| format!(
                "def {a}{}",
                emit_sig(
                    b,
                    i.rid_str().as_str(),
                    "",
                    &FFIStatus::HighLevel { generics }
                )
            ))
            .collect::<Vec<_>>()
            .join("\n"),
        i.rid_str(),
        i.rid_str(),
        i.rid_str(),
        i.rid_str(),
        i.rid_str(),
        i.rid_str(),
        i.methods
            .iter()
            .map(|(a, b)| {
                let rns = b
                    .rets
                    .iter()
                    .enumerate()
                    .map(|(a, _)| format!("r{a}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let pvs = b
                    .params
                    .iter()
                    .enumerate()
                    .map(|(a, b)| {
                        let mut c = format!("_{a}");
                        if let Arg::Resource {
                            ty,
                            nullable,
                            take,
                            ann,
                        } = b
                        {
                            match ty {
                                // ResTy::None  if !ann.iter().any(|a|a.name == "instance") => {}
                                _ => {
                                    // let rid = match ty {
                                    //     ResTy::None => todo!(),
                                    //     ResTy::Of(a) => *a,
                                    //     ResTy::This => i.rid(),
                                    // };
                                    // let rid = hex::encode(rid);
                                    let d = emit_ty(
                                        b,
                                        i.rid_str().as_str(),
                                        &FFIStatus::HighLevel { generics },
                                    );
                                    c = format!("summon[Handler[{d}]].fromHandle({c})");
                                }
                            }
                        }
                        c
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                let rvs = b
                    .rets
                    .iter()
                    .enumerate()
                    .map(|(a, b)| {
                        let mut c = format!("r{a}");
                        if let Arg::Resource {
                            ty,
                            nullable,
                            take,
                            ann,
                        } = b
                        {
                            match ty {
                                // ResTy::None  if !ann.iter().any(|a|a.name == "instance") => {}
                                _ => {
                                    // let rid = match ty {
                                    //     ResTy::None => todo!(),
                                    //     ResTy::Of(a) => *a,
                                    //     ResTy::This => i.rid(),
                                    // };
                                    // let rid = hex::encode(rid);
                                    let d = emit_ty(
                                        b,
                                        i.rid_str().as_str(),
                                        &FFIStatus::HighLevel { generics },
                                    );
                                    c = format!("summon[Handler[{d}]].handleOf({c}")
                                }
                            }
                        }
                        c
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    r#"@Export(name="tpit/{}/~{pkg}/{a}") def {a}{} = {{
                        var z = all(handle);
                        return z.{a}({pvs}) match {{
                            case ({rns}) => ({rvs})
                        }};
                    }}"#,
                    i.rid_str(),
                    emit_sig(b, i.rid_str().as_str(), "handle:Int,", &FFIStatus::FFI)
                )
            })
            .chain(once(format!(
                r#"
            @Export(name="tpit/{}/~{pkg}.drop") def finalize(z: Int) = {{
                val a = all.get(z);
                all.subtractOne(z);
                a match{{
                    None=>(),
                    Some(a) => finalize(a)
                }}
            }}"#,
                i.rid_str()
            )))
            .collect::<Vec<_>>()
            .join("\n"),
        i.rid_str(),
        i.methods
            .iter()
            .map(|(a, b)| {
                let rns = b
                    .rets
                    .iter()
                    .enumerate()
                    .map(|(a, _)| format!("r{a}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let rvs = b
                    .rets
                    .iter()
                    .enumerate()
                    .map(|(a, b)| {
                        let mut c = format!("r{a}");
                        if let Arg::Resource {
                            ty,
                            nullable,
                            take,
                            ann,
                        } = b
                        {
                            match ty {
                                // ResTy::None  if !ann.iter().any(|a|a.name == "instance") => {}
                                _ => {
                                    // let rid = match ty {
                                    //     ResTy::None => todo!(),
                                    //     ResTy::Of(a) => *a,
                                    //     ResTy::This => i.rid(),
                                    // };
                                    // let rid = hex::encode(rid);
                                    let d = emit_ty(
                                        b,
                                        i.rid_str().as_str(),
                                        &FFIStatus::HighLevel { generics },
                                    );
                                    c = format!("summon[Handler[{d}]].fromHandle({c})");
                                }
                            }
                        }
                        c
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    r#"def {a}{} = {{
                        @Import(name = "{}",module="{}") @native def go{};
                        return go(handle,{}) match{{
                            case ({rns}) => ({rvs})
                        }};
                    }}"#,
                    emit_sig(
                        b,
                        i.rid_str().as_str(),
                        "",
                        &FFIStatus::HighLevel { generics }
                    ),
                    a,
                    format!("tpit/{}", i.rid_str()),
                    emit_sig(b, i.rid_str().as_str(), "handle: Int,", &FFIStatus::FFI),
                    b.params
                        .iter()
                        .enumerate()
                        .map(|(a, b)| {
                            let mut c = format!("_{a}");
                            if let Arg::Resource {
                                ty,
                                nullable,
                                take,
                                ann,
                            } = b
                            {
                                match ty {
                                    // ResTy::None if !ann.iter().any(|a|a.name == "instance")=> {}
                                    _ => {
                                        // let rid = match ty {
                                        //     ResTy::None => todo!(),
                                        //     ResTy::Of(a) => *a,
                                        //     ResTy::This => i.rid(),
                                        // };
                                        // let rid = hex::encode(rid);
                                        let d = emit_ty(
                                            b,
                                            i.rid_str().as_str(),
                                            &FFIStatus::HighLevel { generics },
                                        );
                                        c = format!("summon[Handler[{d}]].handleOf({c})")
                                    }
                                }
                            }
                            c
                        })
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        i.rid_str(),
        i.rid_str(),
    )
}
pub enum FFIStatus {
    FFI,
    HighLevel { generics: u32 },
}
pub fn do_instances<'a>(
    instances: &[&str],
    rid: impl Into<Option<&'a str>>,
    generics: u32,
) -> (Vec<String>, String) {
    let rid = rid.into();
    let is2 = instances
        .iter()
        .filter_map(|a| {
            let mut stack = vec![];
            for s in a.split(";") {
                if let Ok(g) = s.parse::<u32>() {
                    stack.push(format!("T{a}"));
                }
                if s.starts_with("R") {
                    let (rid, n) = s.split_once("N")?;
                    let n: u32 = n.parse().ok()?;
                    let mut n = (0..n).filter_map(|a| stack.pop()).join(",");
                    if n != "" {
                        n = format!("[{n}]");
                    };
                    stack.push(format!("R{rid}{n}"));
                }
                if s == "any" {
                    stack.push(format!("Int"));
                }
                if s == "this" {
                    let mut n = (0..generics).filter_map(|a| stack.pop()).join(",");
                    if n != "" {
                        n = format!("[{n}]");
                    };
                    let rid = rid.unwrap();
                    stack.push(format!("R{rid}{n}"));
                }
            }
            stack.pop()
        })
        .collect::<Vec<_>>();
    let mut instance_renders = is2.join(",");
    if instance_renders != "" {
        instance_renders = format!("[{}]", instance_renders);
    };
    return (is2, instance_renders);
}
pub fn emit_ty<'a>(a: &Arg, rid: impl Into<Option<&'a str>>, ffi: &FFIStatus) -> String {
    let rid = rid.into();
    match a {
        Arg::I32 => "Int".to_owned(),
        Arg::I64 => "Long".to_owned(),
        Arg::F32 => "Float".to_owned(),
        Arg::F64 => "Double".to_owned(),
        Arg::Resource {
            ty,
            nullable,
            take,
            ann,
        } => match ffi {
            FFIStatus::FFI => format!("Int"),
            FFIStatus::HighLevel { generics } => {
                let instances: Vec<_> = ann
                    .iter()
                    .find_map(|a| {
                        if a.name == "instance" {
                            Some(a.value.as_str())
                        } else {
                            None
                        }
                    })
                    .into_iter()
                    .flat_map(|a| a.split(","))
                    .collect();
                let (is2, instance_renders) = do_instances(&instances, rid, *generics);
                match ty {
                    ResTy::Of(x) => {
                        let x = hex::encode(x);
                        format!("Option[R{x}.Impl{instance_renders}]")
                    }
                    ResTy::This => {
                        let rid = rid.unwrap();
                        format!("Option[R{rid}.Impl{instance_renders}]")
                    }
                    ResTy::None => {
                        if is2.len() == 1 {
                            format!("Option[summon[Handler[{}]].Impl]", is2[0])
                        } else {
                            format!("Int")
                        }
                    }
                }
            }
        },
    }
}
pub fn emit_sig<'a>(
    a: &Sig,
    rid: impl Into<Option<&'a str>>,
    prepend: &str,
    ffi: &FFIStatus,
) -> String {
    let rid = rid.into();
    let generics2: Option<u32> = a
        .ann
        .iter()
        .find_map(|a| {
            if a.name == "generics" {
                Some(a.value.clone())
            } else {
                None
            }
        })
        .map(|a| a.parse().unwrap());
    let gstr = match ffi {
        FFIStatus::FFI => format!(""),
        FFIStatus::HighLevel { generics } => match generics2 {
            None => format!(""),
            Some(generics2) => {
                let mut generics2Str = (0..generics2)
                    .map(|a| a + generics)
                    .map(|a| format!("T{a}: Handler"))
                    .collect::<Vec<_>>()
                    .join(",");
                let mut generics2RStr = (0..generics2)
                    .map(|a| a + generics)
                    .map(|a| format!("T{a}"))
                    .collect::<Vec<_>>()
                    .join(",");
                if generics2Str != "" {
                    generics2Str = format!("[{}]", generics2Str);
                }
                if generics2RStr != "" {
                    generics2RStr = format!("[{}]", generics2RStr);
                }
                generics2Str
            }
        },
    };
    format!(
        "{gstr}({prepend}{}): ({})",
        a.params
            .iter()
            .enumerate()
            .map(|(i, x)| format!("_{i}: {}", emit_ty(x, rid, ffi)))
            .collect::<Vec<_>>()
            .join(","),
        a.rets
            .iter()
            .map(|x| emit_ty(x, rid, ffi))
            .collect::<Vec<_>>()
            .join(",")
    )
}
pub type Binders = BTreeMap<(String, String, Exposition), (Sig, String)>;
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum Exposition {
    Expose,
    Import,
}
pub fn emit_binder_sig(s: &Sig, name: &str, pkg: &str) -> String {
    let rname = format!(".{name}@{s}");
    let generics2: u32 = s
        .ann
        .iter()
        .find_map(|a| {
            if a.name == "generics" {
                Some(a.value.clone())
            } else {
                None
            }
        })
        .map(|a| a.parse::<u32>().unwrap())
        .unwrap_or(0u32);
    let mut generics2Str = (0..generics2)
        // .map(|a| a + generics)
        .map(|a| format!("T{a}: Handler"))
        .chain(once(format!("Item")))
        .collect::<Vec<_>>()
        .join(",");
    let mut generics2RStr = (0..generics2)
        // .map(|a| a + generics)
        .map(|a| format!("T{a}"))
        .chain(once(format!("Item")))
        .collect::<Vec<_>>()
        .join(",");
    if generics2Str != "" {
        generics2Str = format!("[{}]", generics2Str);
    }
    if generics2RStr != "" {
        generics2RStr = format!("[{}]", generics2RStr);
    }
    return format!(
        r"
        package {pkg};
        import scala.collection.mutable.{{Map,HashMap}};
        import org.teavm.interop.{{Import,Export}};
        import pc.portal.pit.guest.{{Handler,drop}};
        trait {name}{generics2Str}: Handler[Item]{{
            def {name}{}
        }};
        trait {name}Impl{generics2Str}: Handler[Item] with {name}{generics2RStr}{{
            def impl_{name}{}
        }};
    ",
        emit_sig(s, None, "me: Impl,", &FFIStatus::HighLevel { generics: 0 }),
        emit_sig(s, None, "me: Item,", &FFIStatus::HighLevel { generics: 0 })
    );
}
