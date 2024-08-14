use std::{collections::BTreeMap, fmt::Display};

use nom::{
    bytes::complete::{tag, take_while_m_n}, character::complete::{alphanumeric1, multispace0}, multi::many0, sequence::delimited, IResult
};

use crate::{merge, parse_attr, Attr};
#[derive(Default,Clone)]
pub struct Info {
    pub interfaces: BTreeMap<[u8; 32], InfoEntry>,
}
impl Info{
    pub fn merge(self, x: Info) -> Info{
        let mut m: BTreeMap<[u8;32],InfoEntry> = BTreeMap::new();
        for (a,b) in self.interfaces.into_iter().chain(x.interfaces.into_iter()){
            let c = m.remove(&a).unwrap_or_default().merge(b);
            m.insert(a, c);
        }
        Info { interfaces: m }
    }
}
#[derive(Default,Clone)]
pub struct InfoEntry {
    pub attrs: Vec<Attr>,
    pub methods: BTreeMap<String,MethEntry>
}
impl InfoEntry{
    pub fn merge(self, x: InfoEntry) -> InfoEntry{
        let mut m: BTreeMap<String, MethEntry> = BTreeMap::new();
        for (a,b) in self.methods.into_iter().chain(x.methods.into_iter()){
            let c = m.remove(&a).unwrap_or_default().merge(b);
            m.insert(a, c);
        }
        InfoEntry { attrs: merge(self.attrs, x.attrs), methods: m }
    }
}
#[derive(Default,Clone)]
pub struct MethEntry{
    pub attrs: Vec<Attr>
}
impl MethEntry{
    pub fn merge(self, x: MethEntry) -> MethEntry{
        MethEntry { attrs: merge(self.attrs, x.attrs) }
    }
}
impl Display for InfoEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for a in self.attrs.iter(){
            write!(f,"root {a}")?;
        }
        for (k,m) in self.methods.iter(){
            for a in m.attrs.iter(){
                write!(f,"method {k} {a}")?;
            }
        }
       Ok(())
    }
}
impl Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, j) in self.interfaces.iter() {
            write!(f, "{}: [{j}]", hex::encode(i))?;
        }
        Ok(())
    }
}
pub fn parse_entry(a: &str) -> IResult<&str, InfoEntry> {
    let (a,_) = multispace0(a)?;
    pub fn go1(a: &str) -> IResult<&str,Attr>{
        let (a,_) = multispace0(a)?;
        let (a,_) = tag("root")(a)?;
        let (a,_) = multispace0(a)?;
        return parse_attr(a);
    }
    pub fn go2(a: &str) -> IResult<&str,(&str,Attr)>{
        let (a,_) = multispace0(a)?;
        let (a,_) = tag("method")(a)?;
        let (a,_) = multispace0(a)?;
        let (a,b) =         alphanumeric1(a)?;
        let (a,_) = multispace0(a)?;
        let (a,c) = parse_attr(a)?;
        Ok((a,(b,c)))
    }
    let (a,mut e) = many0(go1)(a)?;
    e.sort_by_key(|k|k.name.clone());
    let mut n: BTreeMap<String, MethEntry> = BTreeMap::new();
    let (a,l) = many0(go2)(a)?;
    for (k,v) in l{
        n.entry(k.to_owned()).or_insert_with(Default::default).attrs.push(v);
    }
    for v in n.values_mut(){
        v.attrs.sort_by_key(|k|k.name.clone());
    }
    let (a,_) = multispace0(a)?;
    Ok((a, InfoEntry { attrs: e, methods: n }))
}
pub fn parse_info(a: &str) -> IResult<&str, Info> {
    pub fn go(a: &str) -> IResult<&str, ([u8; 32], InfoEntry)> {
        let (a,_) = multispace0(a)?;
        let (a, d) = take_while_m_n(64, 64, |a: char| a.is_digit(16))(a)?;
        let mut b = [0u8; 32];
        hex::decode_to_slice(d, &mut b).unwrap();
        let (a,_) = multispace0(a)?;
        let (a,_) = tag(":")(a)?;
        let (a,_) = multispace0(a)?;
        let (a, c) = delimited(tag("["), parse_entry, tag("]"))(a)?;
        return Ok((a, (b, c)));
    }
    let (a, all) = many0(go)(a)?;
    Ok((
        a,
        Info {
            interfaces: all.into_iter().collect(),
        },
    ))
}
