//! # 32-bit Buffer Module
//!
//! Provides the 32-bit addressable buffer interface and implementations.
//!
//! This module contains:
//! - The `R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5` trait (32-bit buffer)
//! - Implementations for common Rust types (`Vec<u8>`, `Box<[u8]>`, slices)
//! - Helper functions for working with buffers
//!
//! ## Buffer Interface
//!
//! The buffer interface is defined in `buffer.pit`:
//! ```text
//! {
//!     read8(I32) -> (I32);
//!     write8(I32, I32) -> ();
//!     size() -> (I32)
//! }
//! ```

pub mod ffi;
use crate::buffer64::R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d;
pub use ffi::*;
use std::sync::Arc;
macro_rules! buffer_slice_impl {
    ($t:ty) => {
        impl R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 for $t {
            fn read8(&mut self, p0: u32) -> (u32) {
                return self[p0 as usize].into();
            }
            fn write8(&mut self, p0: u32, p1: u32) -> () {
                self[p0 as usize] = (p1 & 0xff) as u8;
            }
            fn size(&mut self) -> (u32) {
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
        impl R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 for $t {
            fn read8(&mut self, p0: u32) -> (u32) {
                return self[p0 as usize].into();
            }
            fn write8(&mut self, p0: u32, p1: u32) -> () {}
            fn size(&mut self) -> (u32) {
                return self.len().try_into().unwrap();
            }
        }
    };
}
buffer_ro_slice_impl!(Arc<[u8]>);
buffer_ro_slice_impl!(&'static [u8]);

/// Creates an iterator over the bytes in a buffer.
///
/// # Arguments
///
/// * `x` - A mutable reference to a buffer
///
/// # Returns
///
/// An iterator yielding each byte in the buffer.
pub fn slice<'a>(
    x: &'a mut dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
) -> impl Iterator<Item = u8> + 'a {
    return (0..x.size()).map(|a| (x.read8(a) & 0xff) as u8);
}

/// Copies bytes from one buffer to another.
///
/// # Arguments
///
/// * `a` - Destination buffer
/// * `ai` - Starting offset in destination
/// * `b` - Source buffer
/// * `bi` - Starting offset in source
pub fn copy<'a, 'b>(
    a: &'a mut dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
    ai: u32,
    b: &'b mut dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
    bi: u32,
) {
    let l = (a.size() - ai).min(b.size() - bi);
    for i in 0..l {
        a.write8(ai + i, b.read8(bi + i));
    }
}

/// Copies bytes from a Rust slice into a buffer.
///
/// # Arguments
///
/// * `a` - Destination buffer
/// * `ai` - Starting offset in destination
/// * `b` - Source slice
pub fn copy_slice_in<'a, 'b>(
    a: &'a mut dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
    ai: u32,
    b: &'b [u8],
) {
    for (i, b) in b.iter().enumerate() {
        if (i as u32) + ai >= a.size() {
            return;
        }
        a.write8(ai + (i as u32), *b as u32)
    }
}

/// Copies bytes from a buffer into a Rust slice.
///
/// # Arguments
///
/// * `c` - Destination slice
/// * `b` - Source buffer
/// * `bi` - Starting offset in source
pub fn copy_slice_out<'a, 'b>(
    c: &'a mut [u8],
    b: &'b mut dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
    bi: u32,
) {
    for (i, c) in c.iter_mut().enumerate() {
        if (i as u32) + bi >= b.size() {
            return;
        }
        *c = (b.read8(bi + (i as u32)) & 0xff) as u8;
    }
}

/// Adapter that converts between 32-bit and 64-bit buffer interfaces.
///
/// `Snip<T>` wraps a buffer and provides the opposite interface:
/// - If `T` implements the 64-bit interface, `Snip<T>` implements the 32-bit interface
/// - If `T` implements the 32-bit interface, `Snip<T>` implements the 64-bit interface
pub struct Snip<T> {
    /// The wrapped buffer.
    pub wrapped: T,
}
impl<T: crate::buffer64::R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d>
    R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 for Snip<T>
{
    fn read8(&mut self, p0: u32) -> (u32) {
        self.wrapped.read8(p0 as u64)
    }
    fn size(&mut self) -> (u32) {
        self.wrapped.size() as u32
    }
    fn write8(&mut self, p0: u32, p1: u32) -> () {
        self.wrapped.write8(p0 as u64, p1)
    }
}
impl<T: R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>
    R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d for Snip<T>
{
    fn read8(&mut self, p0: u64) -> (u32) {
        self.wrapped.read8(p0 as u32)
    }
    fn size(&mut self) -> (u64) {
        self.wrapped.size().into()
    }
    fn write8(&mut self, p0: u64, p1: u32) -> () {
        self.wrapped.write8(p0 as u32, p1)
    }
}

/// A slice view into a buffer.
///
/// `Slice<T>` wraps a buffer and provides access to a contiguous sub-region.
pub struct Slice<T> {
    /// The wrapped buffer.
    pub wrapped: T,
    /// The starting offset of the slice.
    pub begin: u32,
    /// The size of the slice.
    pub size: u32,
}
impl<T: R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>
    R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 for Slice<T>
{
    fn read8(&mut self, p0: u32) -> (u32) {
        self.wrapped.read8(p0 + self.begin)
    }
    fn size(&mut self) -> (u32) {
        self.size
    }
    fn write8(&mut self, p0: u32, p1: u32) -> () {
        self.wrapped.write8(p0 + self.begin, p1)
    }
}
