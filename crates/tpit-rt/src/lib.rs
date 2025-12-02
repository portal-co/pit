//! # TPIT Runtime
//!
//! Tablified PIT (TPIT) runtime library for WebAssembly.
//!
//! TPIT is an intermediate binary format used in the compilation pipeline for PIT ABI v1.
//! It provides a table-based emulation layer for `externref` on WebAssembly platforms
//! that don't natively support the reference types proposal. Instead of using actual
//! `externref` values, TPIT uses integer handles that index into a WebAssembly table.
//!
//! ## Compilation Flow
//!
//! TPIT modules are typically produced by compiling Rust or LLVM code with TPIT bindings,
//! then converted to PIT ABI v1 using `pit untpit`:
//!
//! ```text
//! Rust/LLVM → TPIT Module → (pit untpit) → PIT ABI v1 Module
//! ```
//!
//! ## Overview
//!
//! The [`Tpit`] type wraps an integer handle that represents a resource reference.
//! When the handle is dropped, it calls the TPIT drop function to release the
//! underlying resource.
//!
//! ## Usage
//!
//! This crate is typically used as a dependency for generated PIT guest code.
//! The generated code will use `Tpit<T>` instead of `externref::Resource<T>`.
//!
//! ## Safety
//!
//! The [`Tpit::new`] and [`Tpit::summon`] functions are unsafe because they create
//! handles from raw integers without verifying that the handle is valid.
//!
//! ## no_std
//!
//! This crate is `no_std` compatible and can be used in WebAssembly environments
//! without the standard library.

#![no_std]
use core::{
    marker::PhantomData,
    mem::forget,
    num::NonZeroU32,
    sync::{
        atomic::{AtomicUsize, Ordering},
        // Arc,
    },
};

/// A tablified PIT resource handle.
///
/// `Tpit<D>` wraps an integer handle that refers to a resource in a WebAssembly table.
/// The type parameter `D` is a phantom type used to distinguish different resource types
/// at compile time.
///
/// When dropped, this type automatically calls the TPIT drop function to release the
/// underlying resource.
///
/// # Example
///
/// ```ignore
/// // Typically created by generated code
/// let resource: Tpit<MyResource> = unsafe { Tpit::new(handle) };
///
/// // Get the raw pointer value
/// let ptr = resource.ptr();
///
/// // Consume without dropping (transfers ownership)
/// let raw = resource.forget_to_ptr();
/// ```
#[repr(transparent)]
pub struct Tpit<D> {
    ptr: Option<NonZeroU32>,
    phantom: PhantomData<D>,
}
impl<D> Drop for Tpit<D> {
    fn drop(&mut self) {
        #[link(wasm_import_module = "tpit")]
        extern "C" {
            fn drop(a: u32);
            fn void(a: u32);
        }
        if let Some(p) = self.ptr {
            unsafe {
                drop(p.into());
            }
        }
    }
}
impl<D> Tpit<D> {
    /// Creates a new `Tpit` handle from a raw integer pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is a valid handle obtained from the TPIT
    /// runtime. Using an invalid handle may lead to undefined behavior.
    ///
    /// A `ptr` value of 0 is treated as a null reference.
    pub unsafe fn new(ptr: u32) -> Self {
        Self {
            ptr: NonZeroU32::new(ptr),
            phantom: PhantomData,
        }
    }

    /// Returns the raw integer handle value.
    ///
    /// Returns 0 if this is a null reference.
    pub fn ptr(&self) -> u32 {
        match self.ptr {
            Some(a) => {
                let a: u32 = a.into();
                a
            }
            None => 0,
        }
    }

    /// Casts this handle to a different resource type.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the underlying resource is actually of type `U`.
    pub unsafe fn cast<U>(self) -> Tpit<U> {
        unsafe { Tpit::<U>::new(self.forget_to_ptr()) }
    }

    /// Consumes the handle and returns the raw pointer value without calling drop.
    ///
    /// This is useful for transferring ownership of the resource to another handle
    /// or to foreign code.
    pub fn forget_to_ptr(self) -> u32 {
        let x = self.ptr();
        forget(self);
        return x;
    }

    /// Creates a mutable reference to a `Tpit` from a mutable reference to a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the memory layout is correct and that the
    /// lifetime of the returned reference does not exceed the input reference.
    pub unsafe fn summon(a: &mut u32) -> &mut Self {
        unsafe { core::mem::transmute(a) }
    }
}
