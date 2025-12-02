//! # PIT Basic
//!
//! Basic buffer interface implementations for Portal Interface Types.
//!
//! This crate provides implementations of the standard PIT buffer interfaces
//! (`buffer.pit` and `buffer64.pit`) for common Rust types like `Vec<u8>`,
//! `Box<[u8]>`, and slice references.
//!
//! ## Modules
//!
//! - [`buffer`] - 32-bit addressable buffer interface implementations
//! - [`buffer64`] - 64-bit addressable buffer interface implementations
//!
//! ## Buffer Interface
//!
//! The buffer interface defines three methods:
//! - `read8(offset)` - Read a byte at the given offset
//! - `write8(offset, value)` - Write a byte at the given offset
//! - `size()` - Get the buffer size
//!
//! ## Features
//!
//! - `host` - Enable host-side functionality
//! - `host-ic` - Enable Internet Computer stable structures support
//! - `ic-stable-structures` - Enable IC stable structures dependency

use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

/// 32-bit addressable buffer module.
///
/// See [`buffer::R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5`]
/// for the main buffer trait.
pub mod buffer;

/// 64-bit addressable buffer module.
///
/// See [`buffer64::R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d`]
/// for the main buffer trait.
pub mod buffer64;
