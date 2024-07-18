use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

include!("buffer.rs");
impl<T: Deref<Target = [u8]> + DerefMut>
    R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 for Mutex<T>
{
    fn read8(&self, p0: u32) -> (u32) {
        return self.lock().unwrap()[p0 as usize].into();
    }

    fn write8(&self, p0: u32, p1: u32) -> () {
        self.lock().unwrap()[p0 as usize] = (p1 & 0xff) as u8;
    }

    fn size(&self) -> (u32) {
        return self.lock().unwrap().len().try_into().unwrap();
    }
}
pub fn slice(
    x: &dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
) -> impl Iterator<Item = u8> + '_ {
    return (0..x.size()).map(|a| (x.read8(a) & 0xff) as u8);
}
