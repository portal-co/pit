use derive_more::Display;
use nom::{
    bytes::complete::{is_not, tag, take, take_while_m_n},
    character::complete::{alpha1, alphanumeric1, char, multispace0},
    combinator::opt,
    error::Error,
    multi::{many0, separated_list0},
    sequence::{delimited, tuple},
    IResult,
};
use sha3::{Digest, Sha3_256};
use std::{collections::BTreeMap, fmt::Display};
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct Attr {
    pub name: String,
    pub value: String,
}

pub fn merge(a: Vec<Attr>, b: Vec<Attr>) -> Vec<Attr> {
    let mut m = BTreeMap::new();
    for x in a.into_iter().chain(b) {
        m.insert(x.name, x.value);
    }
    return m
        .into_iter()
        .map(|(a, b)| Attr { name: a, value: b })
        .collect();
}

pub fn parse_balanced(mut a: &str) -> IResult<&str, String> {
    let mut v = vec![];
    let mut i = 0;
    loop {
        let (b, x) = nom::character::complete::anychar(a)?;
        match x {
            '[' => i += 1,
            ']' => {
                if i == 0 {
                    return Ok((a, v.into_iter().collect()));
                }
                i -= 1;
            }
            _ => {}
        }
        a = b;
        v.push(x)
    }
}

pub fn parse_attr(a: &str) -> IResult<&str, Attr> {
    let (a, _) = multispace0(a)?;
    let (a, _) = char('[')(a)?;
    let (a, _) = multispace0(a)?;
    let (a, name) = alphanumeric1(a)?;
    let (a, _) = multispace0(a)?;
    let (a, _) = char('=')(a)?;
    let (a, _) = multispace0(a)?;
    let (a, value) = parse_balanced(a)?;
    let (a, _) = char(']')(a)?;
    let (a, _) = multispace0(a)?;
    return Ok((
        a,
        Attr {
            name: name.to_owned(),
            value,
        },
    ));
}

pub fn parse_attrs(a: &str) -> IResult<&str, Vec<Attr>> {
    let (a, mut b) = many0(parse_attr)(a)?;
    b.sort_by_key(|a| a.name.clone());
    Ok((a, b))
}

impl Display for Attr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}={}]", self.name, self.value)
    }
}
#[non_exhaustive]
#[derive(Display, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum ResTy {
    #[display(fmt = "")]
    None,
    #[display(fmt = "{}", "hex::encode(_0)")]
    Of([u8; 32]),
    #[display(fmt = "this")]
    This,
}
pub fn parse_resty(a: &str) -> IResult<&str, ResTy> {
    if let Some(a) = a.strip_prefix("this") {
        // let (a, k) = opt(tag("n"))(a)?;
        return Ok((a, ResTy::This));
    }
    let (a, d) = opt(take_while_m_n(64, 64, |a: char| a.is_digit(16)))(a)?;
    return Ok((
        a,
        match d {
            Some(d) => {
                let mut b = [0u8; 32];
                hex::decode_to_slice(d, &mut b).unwrap();
                ResTy::Of(b)
            }
            None => ResTy::None,
        },
    ));
}
#[non_exhaustive]
#[derive(Display, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub enum Arg {
    I32,
    I64,
    F32,
    F64,
    #[display(
        fmt = "{}R{}{}{}",
        "ann.iter().map(|a|a.to_string()).collect::<Vec<_>>().join(\"\")",
        "ty",
        "if *nullable{\"n\"}else{\"\"}",
        "if *take{\"\"}else{\"&\"}"
    )]
    Resource {
        ty: ResTy,
        nullable: bool,
        take: bool,
        ann: Vec<Attr>,
    },
    // #[display(fmt = "{}", _0)]
    // Func(Sig),
}
pub fn parse_arg(a: &str) -> IResult<&str, Arg> {
    let (a, ann) = parse_attrs(a)?;
    let (a, _) = multispace0(a)?;
    // let (c,b) = take(1usize)(a)?;
    match a.strip_prefix("R") {
        Some(b) => {
            // if let Some(a) = b.strip_prefix("this"){
            //     let (a, k) = opt(tag("n"))(a)?;
            //     return Ok((
            //         a,
            //         Arg::Resource {
            //             ty: ResTy::This,
            //             nullable: k.is_some(),
            //         },
            //     ));
            // }
            let (a, d) = parse_resty(b)?;
            let (a, k) = opt(tag("n"))(a)?;
            let (a, take) = opt(tag("&"))(a)?;
            return Ok((
                a,
                Arg::Resource {
                    ty: d,
                    nullable: k.is_some(),
                    take: take.is_none(),
                    ann,
                },
            ));
        }
        // "(" => {
        //     let (a, x) = parse_sig(a)?;
        //     return Ok((a, Arg::Func(x)));
        // }
        None => {
            let (a, c) = take(3usize)(a)?;
            match c {
                "I32" => return Ok((a, Arg::I32)),
                "I64" => return Ok((a, Arg::I64)),
                "F32" => return Ok((a, Arg::F32)),
                "F64" => return Ok((a, Arg::F64)),
                _ => return Err(nom::Err::Error(Error::new(a, nom::error::ErrorKind::Tag))),
            }
        }
    }
    todo!()
}
#[derive(Display, Clone, Eq, PartialEq, PartialOrd, Ord)]
#[display(
    fmt = "{}({}) -> ({})",
    "ann.iter().map(|a|a.to_string()).collect::<Vec<_>>().join(\"\")",
    "params.iter().map(|a|a.to_string()).collect::<Vec<_>>().join(\",\")",
    "rets.iter().map(|a|a.to_string()).collect::<Vec<_>>().join(\",\")"
)]
pub struct Sig {
    pub ann: Vec<Attr>,
    pub params: Vec<Arg>,
    pub rets: Vec<Arg>,
}
pub fn parse_sig(a: &str) -> IResult<&str, Sig> {
    let (a, b) = parse_attrs(a)?;
    let (a, _) = multispace0(a)?;
    let mut d = delimited(char('('), separated_list0(char(','), parse_arg), char(')'));
    let (a, params) = d(a)?;
    let (a, _) = multispace0(a)?;
    let (a, _) = tag("->")(a)?;
    let (a, _) = multispace0(a)?;
    let (a, rets) = d(a)?;
    return Ok((
        a,
        Sig {
            params,
            rets,
            ann: b,
        },
    ));
}
#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct Interface {
    pub methods: BTreeMap<String, Sig>,
    pub ann: Vec<Attr>,
}
impl Display for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for a in self.ann.iter() {
            write!(f, "{a}")?;
        }
        write!(f, "{}", "{")?;
        let mut x = self.methods.iter().collect::<Vec<_>>();
        x.sort_by_key(|a| a.0);
        for (i, (a, b)) in x.into_iter().enumerate() {
            if i != 0 {
                write!(f, ";")?;
            }
            write!(f, "{}{}", a, b)?;
        }
        return write!(f, "{}", "}");
    }
}
pub fn parse_interface(a: &str) -> IResult<&str, Interface> {
    pub fn go(a: &str) -> IResult<&str, Interface> {
        let (a, s) = separated_list0(char(';'), tuple((multispace0, alphanumeric1, parse_sig)))(a)?;
        let (a, _) = multispace0(a)?;
        return Ok((
            a,
            Interface {
                methods: s.into_iter().map(|(_, a, b)| (a.to_owned(), b)).collect(),
                ann: vec![],
            },
        ));
    }
    let (a, _) = multispace0(a)?;
    let (a, b) = parse_attrs(a)?;
    let (a, mut c) = delimited(char('{'), go, char('}'))(a)?;
    c.ann = b;
    return Ok((a, c));
}
impl Interface {
    pub fn rid(&self) -> [u8; 32] {
        use std::io::Write;
        let mut s = Sha3_256::default();
        write!(s, "{}", self);
        return s.finalize().try_into().unwrap();
    }
    pub fn rid_str(&self) -> String {
        return hex::encode(self.rid());
    }
}
pub mod info;
pub fn retuple(a: Vec<Arg>) -> Interface {
    Interface {
        methods: a.into_iter().enumerate().map(|(a, b)| {
            (
                format!("v{a}"),
                Sig {
                    ann: vec![],
                    params: vec![],
                    rets: vec![b],
                },
            )
        }).collect(),
        ann: vec![],
    }
}
