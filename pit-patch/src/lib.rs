#![no_std]
extern crate alloc;
use alloc::{
    collections::{BTreeMap, BTreeSet}, vec::Vec,

};

use anyhow::Context;
use pit_core::{Arg, Interface, Sig};
// use util::tfree;
use portal_pc_waffle::{
    util::new_sig, ExportKind, FuncDecl, FunctionBody, ImportKind, Module, SignatureData,
    TableData, Type,
};
use crate::util::add_op;

pub mod util;
pub mod canon;
pub mod lower;
// pub mod tpit;
pub fn get_interfaces(m: &Module) -> anyhow::Result<Vec<Interface>> {
    let c = m
        .custom_sections
        .get(".pit-types")
        .context("in getting type section")?;
    let mut is = alloc::vec![];
    for b in c.split(|a| *a == 0) {
        let s = core::str::from_utf8(b)?;
        let (s, i) = pit_core::parse_interface(s)
            .map_err(|e: nom::Err<nom::error::Error<&str>>| anyhow::anyhow!("invalid pit"))?;
        is.push(i);
    }

    return Ok(is);
}
