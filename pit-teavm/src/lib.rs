use std::{collections::BTreeSet, iter::once};

use pit_core::{Arg, Interface, ResTy, Sig};

pub fn emit(i: &Interface, pkg: &str) -> String {
    // let generics: u32 = i
    //     .ann
    //     .iter()
    //     .find_map(|a| {
    //         if a.name == "generics" {
    //             Some(a.value.clone())
    //         } else {
    //             None
    //         }
    //     })
    //     .map(|a| a.parse().unwrap())
    //     .unwrap_or_default();
    // let mut genericsStr = (0..generics)
    //     .map(|a| format!("T{a}: Handler"))
    //     .collect::<Vec<_>>()
    //     .join(",");
    // let mut genericsRStr = (0..generics)
    //     .map(|a| format!("T{a}"))
    //     .collect::<Vec<_>>()
    //     .join(",");
    // if genericsStr != "" {
    //     genericsStr = format!("[{}]", genericsStr);
    // }
    // if genericsRStr != "" {
    //     genericsRStr = format!("[{}]", genericsRStr);
    // }
    // let qs = if generics == 0 {
    //     "".to_owned()
    // } else {
    //     format!("[{}]", ",?".repeat(generics as usize).split_off(1))
    // };
    format!(
        r#"
        package {pkg};
        import scala.collection.mutable.{{Map,HashMap}};
        import org.teavm.interop.{{Import,Export}};
        import pc.portal.pit.guest.Handler;
        trait R{}{{
            {}
            def finalize: Unit
        }}
        object R{}{{
            given handler = new Handler[R{}]{{
                type Impl = Impl
                def createImpl(a: R{}): Impl = new Impl(a);
                def handleOf(a: Impl): Int = a.handle;
                def fromHandle(a: Int): Impl = new Impl(a);
                def finalize(a: R{}): Unit = a.finalize;
            }};
            object Impl{{
                val all: Map[Int,R{}] = HashMap.empty;
                def create(r: R{}): Int = {{
                    var x = 0
                    while all.contains(x){{
                        x = x + 1   
                    }}
                    all.addOne((x,r))
                    return x
                }}
                {}
            }}
            class Impl(val handle: Int) extends R{}{{
                import Impl.{{all,create}};
                def finalize: Unit = {{
                    @Import(name="drop",module="tpit") native def drop(handle: Int);
                    drop(handle);
                }}
                {}
                def this(res: R{}) = {{
                    @Import(name="~{pkg}",module="tpit/{}") native def go(a: Int): Int
                    this(go(create(res)))
                }}
            }};
        }}"#,
        i.rid_str(),
        i.methods
            .iter()
            .map(|(a, b)| format!(
                "def {a}{}",
                emit_sig(b, &i.rid_str(), "", &FFIStatus::HighLevel {})
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
                                ResTy::None => {}
                                _ => {
                                    let rid = match ty {
                                        ResTy::None => todo!(),
                                        ResTy::Of(a) => *a,
                                        ResTy::This => i.rid(),
                                    };
                                    let rid = hex::encode(rid);
                                    let d = emit_ty(
                                        b,
                                        &i.rid_str(),
                                        &FFIStatus::HighLevel {  },
                                    );
                                    c = format!("summon[Handler[Option[R{rid}]]].fromHandle({c})");
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
                                ResTy::None => {}
                                _ => {
                                    let rid = match ty {
                                        ResTy::None => todo!(),
                                        ResTy::Of(a) => *a,
                                        ResTy::This => i.rid(),
                                    };
                                    let rid = hex::encode(rid);
                                    c = format!("summon[Handler[Option[R{rid}]]].handleOf({c}")
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
                    emit_sig(b, &i.rid_str(), "handle:Int,", &FFIStatus::FFI)
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
                                ResTy::None => {}
                                _ => {
                                    let rid = match ty {
                                        ResTy::None => todo!(),
                                        ResTy::Of(a) => *a,
                                        ResTy::This => i.rid(),
                                    };
                                    let rid = hex::encode(rid);
                                    let d = emit_ty(
                                        b,
                                        &i.rid_str(),
                                        &FFIStatus::HighLevel {  },
                                    );
                                    c = format!("summon[Handler[Option[R{rid}]]].fromHandle({c})");
                                }
                            }
                        }
                        c
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    r#"def {a}{} = {{
                        @Import(name = "{}",module="{}") native def go{};
                        return go(handle,{}) match{{
                            case ({rns}) => ({rvs})
                        }};
                    }}"#,
                    emit_sig(
                        b,
                        &i.rid_str(),
                        "",
                        &FFIStatus::HighLevel {  }
                    ),
                    a,
                    format!("tpit/{}", i.rid_str()),
                    emit_sig(b, &i.rid_str(), "handle: Int,", &FFIStatus::FFI),
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
                                    ResTy::None => {}
                                    _ => {
                                        let rid = match ty {
                                            ResTy::None => todo!(),
                                            ResTy::Of(a) => *a,
                                            ResTy::This => i.rid(),
                                        };
                                        let rid = hex::encode(rid);
                                        c = format!("summon[Handler[Option[R{rid}]]].handleOf({c})")
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
    HighLevel {  },
}
pub fn emit_ty(a: &Arg, rid: &str, ffi: &FFIStatus) -> String {
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
            FFIStatus::HighLevel {  } => {
                // let generics2: Option<u32> = ann
                //     .iter()
                //     .find_map(|a| {
                //         if a.name == "generics" {
                //             Some(a.value.clone())
                //         } else {
                //             None
                //         }
                //     })
                //     .map(|a| a.parse().unwrap());
                match ty {
                    ResTy::Of(x) => {
                        let x = hex::encode(x);
                        format!("Option[R{x}.Impl]")
                    }
                    ResTy::This => format!("Option[R{rid}.Impl]"),
                    ResTy::None => format!("Int"),
                }
            }
        },
    }
}
pub fn emit_sig(a: &Sig, rid: &str, prepend: &str, ffi: &FFIStatus) -> String {
    format!(
        "({prepend}{}): ({})",
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
