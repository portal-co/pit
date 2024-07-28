pub mod ffi;
pub use ffi::*;
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
pub fn slice<'a>(
    x: &'a mut dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
) -> impl Iterator<Item = u8> + 'a {
    return (0..x.size()).map(|a| (x.read8(a) & 0xff) as u8);
}
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
pub struct Slice<T>{
    pub wrapped: T,
    pub begin: u64,
    pub size: u64
}
impl<T: R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d> R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d for Slice<T>{
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