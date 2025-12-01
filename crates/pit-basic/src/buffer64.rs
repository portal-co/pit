//! # 64-bit Buffer Module
//!
//! Provides the 64-bit addressable buffer interface and implementations.
//!
//! This module contains:
//! - The `R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d` trait (64-bit buffer)
//! - Implementations for common Rust types (`Vec<u8>`, `Box<[u8]>`, slices)
//! - Helper functions for working with large buffers
//! - Optional IC stable structures support
//!
//! ## Buffer Interface
//!
//! The buffer interface is defined in `buffer64.pit`:
//! ```text
//! {
//!     read8(I64) -> (I32);
//!     write8(I64, I32) -> ();
//!     size() -> (I64)
//! }
//! ```
//!
//! ## Features
//!
//! - `ic-stable-structures` - Enables integration with Internet Computer stable memory

pub mod ffi;
pub use ffi::*;
use std::sync::Arc;

/// Internet Computer stable structures integration.
#[cfg(feature = "ic-stable-structures")]
pub mod ic;
macro_rules! buffer_slice_impl {
    ($t:ty) => {
        impl R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d for $t {
            fn read8(&mut self, p0: u64) -> (u32) {
                return self[p0 as usize].into();
            }
            fn write8(&mut self, p0: u64, p1: u32) -> () {
                self[p0 as usize] = (p1 & 0xff) as u8;
            }
            fn size(&mut self) -> (u64) {
                return self.len().try_into().unwrap();
            }
        }
    };
}
buffer_slice_impl!(Vec<u8>);
buffer_slice_impl!(Box<[u8]>);
buffer_slice_impl!(&'static mut [u8]);
macro_rules! buffer_ro_slice_impl {
    ($t:ty) => {
        impl R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d for $t {
            fn read8(&mut self, p0: u64) -> (u32) {
                return self[p0 as usize].into();
            }
            fn write8(&mut self, p0: u64, p1: u32) -> () {}
            fn size(&mut self) -> (u64) {
                return self.len().try_into().unwrap();
            }
        }
    };
}
buffer_ro_slice_impl!(Arc<[u8]>);
buffer_ro_slice_impl!(&'static [u8]);

/// Creates an iterator over the bytes in a 64-bit addressable buffer.
///
/// # Arguments
///
/// * `x` - A mutable reference to a buffer
///
/// # Returns
///
/// An iterator yielding each byte in the buffer.
pub fn slice<'a>(
    x: &'a mut dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
) -> impl Iterator<Item = u8> + 'a {
    return (0..x.size()).map(|a| (x.read8(a) & 0xff) as u8);
}

/// Copies bytes from one 64-bit buffer to another.
///
/// # Arguments
///
/// * `a` - Destination buffer
/// * `ai` - Starting offset in destination (64-bit)
/// * `b` - Source buffer
/// * `bi` - Starting offset in source (64-bit)
pub fn copy<'a, 'b>(
    a: &'a mut dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
    ai: u64,
    b: &'b mut dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
    bi: u64,
) {
    let l = (a.size() - ai).min(b.size() - bi);
    for i in 0..l {
        a.write8(ai + i, b.read8(bi + i));
    }
}

/// Copies bytes from a Rust slice into a 64-bit buffer.
///
/// # Arguments
///
/// * `a` - Destination buffer
/// * `ai` - Starting offset in destination (64-bit)
/// * `b` - Source slice
pub fn copy_slice_in<'a, 'b>(
    a: &'a mut dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
    ai: u64,
    b: &'b [u8],
) {
    for (i, b) in b.iter().enumerate() {
        if (i as u64) + ai >= a.size() {
            return;
        }
        a.write8(ai + (i as u64), *b as u32)
    }
}

/// Copies bytes from a 64-bit buffer into a Rust slice.
///
/// # Arguments
///
/// * `c` - Destination slice
/// * `b` - Source buffer
/// * `bi` - Starting offset in source (64-bit)
pub fn copy_slice_out<'a, 'b>(
    c: &'a mut [u8],
    b: &'b mut dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
    bi: u64,
) {
    for (i, c) in c.iter_mut().enumerate() {
        if (i as u64) + bi >= b.size() {
            return;
        }
        *c = (b.read8(bi + (i as u64)) & 0xff) as u8;
    }
}

/// A slice view into a 64-bit addressable buffer.
///
/// `Slice<T>` wraps a buffer and provides access to a contiguous sub-region.
pub struct Slice<T> {
    /// The wrapped buffer.
    pub wrapped: T,
    /// The starting offset of the slice (64-bit).
    pub begin: u64,
    /// The size of the slice (64-bit).
    pub size: u64,
}
impl<T: R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d>
    R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d for Slice<T>
{
    fn read8(&mut self, p0: u64) -> (u32) {
        self.wrapped.read8(p0 + self.begin)
    }
    fn size(&mut self) -> (u64) {
        self.size
    }
    fn write8(&mut self, p0: u64, p1: u32) -> () {
        self.wrapped.write8(p0 + self.begin, p1)
    }
}
