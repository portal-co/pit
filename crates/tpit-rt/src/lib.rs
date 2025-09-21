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
    pub unsafe fn new(ptr: u32) -> Self {
        Self {
            ptr: NonZeroU32::new(ptr),
            phantom: PhantomData,
        }
    }
    pub fn ptr(&self) -> u32 {
        match self.ptr {
            Some(a) => {
                let a: u32 = a.into();
                a
            }
            None => 0,
        }
    }
    pub unsafe fn cast<U>(self) -> Tpit<U> {
        unsafe { Tpit::<U>::new(self.forget_to_ptr()) }
    }
    pub fn forget_to_ptr(self) -> u32 {
        let x = self.ptr();
        forget(self);
        return x;
    }
    pub unsafe fn summon(a: &mut u32) -> &mut Self {
        unsafe { core::mem::transmute(a) }
    }
}
