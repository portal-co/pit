//! # PIT Patch
//!
//! WebAssembly module patching and transformation for Portal Interface Types.
//!
//! This crate provides utilities for transforming WebAssembly modules to work with
//! PIT interfaces, including:
//!
//! - Converting between TPIT (tablified) and externref-based PIT
//! - Lowering externref types to i32 table indices
//! - Canonicalizing interface implementations
//! - Instantiating PIT modules
//!
//! ## Modules
//!
//! - [`canon`] - Interface canonicalization and jiggering
//! - [`lower`] - Externref lowering to table-based representation
//! - [`tpit`] - TPIT wrapper generation
//! - [`tutils`] - Table allocation/deallocation utilities
//! - [`util`] - General utilities for WebAssembly manipulation
//!
//! ## no_std
//!
//! This crate is `no_std` compatible, using `alloc` for dynamic allocation.

#![no_std]
extern crate alloc;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    vec::Vec,
};
use anyhow::Context;
use pit_core::{Arg, Interface, Sig};
// use util::tfree;
use crate::util::add_op;
use portal_pc_waffle::{
    util::new_sig, ExportKind, FuncDecl, FunctionBody, ImportKind, Module, SignatureData,
    TableData, Type,
};

/// Interface canonicalization and ID jiggering.
///
/// This module provides functions to:
/// - Canonicalize interface implementations across multiple unique IDs
/// - Generate unique IDs (jigger) based on module content
pub mod canon;

/// Externref lowering to table-based representation.
///
/// This module provides the [`instantiate`](lower::instantiate) function which
/// transforms a PIT module to use i32 table indices instead of externref values.
pub mod lower;

/// TPIT (Tablified PIT) wrapper generation.
///
/// This module handles the conversion between TPIT and regular PIT by:
/// - Wrapping externref values in table entries
/// - Shimming method calls to convert between representations
pub mod tpit;

/// Table allocation and deallocation utilities.
///
/// Provides [`talloc`](tutils::talloc) and [`tfree`](tutils::tfree) functions
/// for managing table entries.
pub mod tutils;

/// General WebAssembly manipulation utilities.
///
/// Includes helper functions for type conversion and function generation.
pub mod util;

/// Extracts PIT interface definitions from a WebAssembly module.
///
/// Reads the `.pit-types` custom section and parses all interface definitions.
///
/// # Arguments
///
/// * `m` - The WebAssembly module to extract interfaces from
///
/// # Returns
///
/// A vector of all PIT interfaces defined in the module.
///
/// # Errors
///
/// Returns an error if the `.pit-types` section is missing or contains
/// invalid interface definitions.
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
