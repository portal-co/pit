use std::{
    collections::{BTreeMap, BTreeSet},
    iter::once,
    mem::take,
};

use anyhow::Context;
use pit_core::{Arg, Interface, Sig};
use util::tfree;
use waffle::{
    util::new_sig, ExportKind, FuncDecl, FunctionBody, ImportKind, Module, SignatureData,
    TableData, Type,
};
use waffle_ast::add_op;

pub mod util;
pub mod canon;
pub mod lower;
pub fn get_interfaces(m: &Module) -> anyhow::Result<Vec<Interface>> {
    let c = m
        .custom_sections
        .get(".pit-types")
        .context("in getting type section")?;
    let mut is = vec![];
    for b in c.split(|a| *a == 0) {
        let s = std::str::from_utf8(b)?;
        let (s, i) = pit_core::parse_interface(s)
            .map_err(|e: nom::Err<nom::error::Error<&str>>| anyhow::anyhow!("invalid pit"))?;
        is.push(i);
    }

    return Ok(is);
}
