use ic_stable_structures::Memory;

pub struct MemBuf<T> {
    pub wrapped: T,
}
impl<T: Memory> super::R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d
    for MemBuf<T>
{
    fn read8(&mut self, p0: u64) -> (u32) {
        let mut a = [0u8; 1];
        self.wrapped.read(p0, &mut a);
        return a[0] as u32;
    }

    fn size(&mut self) -> (u64) {
        return self.wrapped.size() * 65536;
    }

    fn write8(&mut self, p0: u64, p1: u32) -> () {
        self.wrapped.write(p0, &[(p1 & 0xff) as u8])
    }
}
impl<T: Memory> crate::buffer::R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5
    for MemBuf<T>
{
    fn read8(&mut self, p0: u32) -> (u32) {
        let mut a = [0u8; 1];
        self.wrapped.read(p0.into(), &mut a);
        return a[0] as u32;
    }

    fn size(&mut self) -> (u32) {
        return ((self.wrapped.size() * 65536) & 0xffffffff) as u32;
    }

    fn write8(&mut self, p0: u32, p1: u32) -> () {
        self.wrapped.write(p0.into(), &[(p1 & 0xff) as u8])
    }
}
