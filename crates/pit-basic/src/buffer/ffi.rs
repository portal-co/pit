//! # 32-bit Buffer FFI
//!
//! This module contains the auto-generated FFI bindings for the 32-bit buffer interface.
//!
//! The resource ID `867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5`
//! corresponds to the `buffer.pit` interface definition:
//!
//! ```text
//! {
//!     read8(I32) -> (I32);
//!     write8(I32, I32) -> ();
//!     size() -> (I32)
//! }
//! ```

/// A 32-bit addressable byte buffer interface.
///
/// This trait defines the fundamental operations for reading and writing bytes
/// to a buffer with 32-bit addressing.
///
/// # Methods
///
/// - `read8(offset)` - Read a single byte at the given offset
/// - `write8(offset, value)` - Write a single byte at the given offset
/// - `size()` - Get the total size of the buffer in bytes
pub trait R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 {
    fn read8(&mut self, p0: u32) -> (u32);
    fn size(&mut self) -> (u32);
    fn write8(&mut self, p0: u32, p1: u32) -> ();
}
const _: () = {
    #[link_section = ".pit-types"]
    static SECTION_CONTENT: [u8; 60usize] = [
        123u8, 114u8, 101u8, 97u8, 100u8, 56u8, 40u8, 73u8, 51u8, 50u8, 41u8, 32u8, 45u8, 62u8,
        32u8, 40u8, 73u8, 51u8, 50u8, 41u8, 59u8, 115u8, 105u8, 122u8, 101u8, 40u8, 41u8, 32u8,
        45u8, 62u8, 32u8, 40u8, 73u8, 51u8, 50u8, 41u8, 59u8, 119u8, 114u8, 105u8, 116u8, 101u8,
        56u8, 40u8, 73u8, 51u8, 50u8, 44u8, 73u8, 51u8, 50u8, 41u8, 32u8, 45u8, 62u8, 32u8, 40u8,
        41u8, 125u8, 0u8,
    ];
    fn alloc<T>(m: &mut ::std::collections::BTreeMap<u32, T>, x: T) -> u32 {
        let mut u = 0;
        while m.contains_key(&u) {
            u += 1;
        }
        m.insert(u, x);
        return u;
    }
    #[derive(Default)]
    struct TableCell {
        all: std::cell::UnsafeCell<
            ::std::collections::BTreeMap<
                u32,
                Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>,
            >,
        >,
    }
    unsafe impl Send for TableCell {}
    unsafe impl Sync for TableCell {}
    static TABLE: ::std::sync::LazyLock<TableCell> =
        ::std::sync::LazyLock::new(|| TableCell::default());
    impl R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5
        for ::tpit_rt::Tpit<
            Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>,
        >
    {
        fn read8(&mut self, p0: u32) -> (u32) {
            #[link(
                wasm_import_module = "tpit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5"
            )]
            extern "C" {
                #[link_name = "read8"]
                fn go(this: u32, p0: u32) -> (u32);
            }
            return unsafe { go(self.ptr(), p0) };
        }
        fn size(&mut self) -> (u32) {
            #[link(
                wasm_import_module = "tpit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5"
            )]
            extern "C" {
                #[link_name = "size"]
                fn go(this: u32) -> (u32);
            }
            return unsafe { go(self.ptr()) };
        }
        fn write8(&mut self, p0: u32, p1: u32) -> () {
            #[link(
                wasm_import_module = "tpit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5"
            )]
            extern "C" {
                #[link_name = "write8"]
                fn go(this: u32, p0: u32, p1: u32) -> ();
            }
            return unsafe { go(self.ptr(), p0, p1) };
        }
    }
    #[export_name = "tpit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5/~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e.drop"]
    extern "C" fn _drop(a: u32) {
        unsafe { (&mut *(TABLE.all.get())).remove(&a) };
    }
    #[export_name = "tpit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5/~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e/read8"]
    extern "C" fn read8(id: u32, p0: u32) -> (u32) {
        return unsafe { &mut *(TABLE.all.get()) }
            .get_mut(&id)
            .unwrap()
            .read8(p0);
    }
    #[export_name = "tpit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5/~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e/size"]
    extern "C" fn size(id: u32) -> (u32) {
        return unsafe { &mut *(TABLE.all.get()) }
            .get_mut(&id)
            .unwrap()
            .size();
    }
    #[export_name = "tpit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5/~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e/write8"]
    extern "C" fn write8(id: u32, p0: u32, p1: u32) -> () {
        return unsafe { &mut *(TABLE.all.get()) }
            .get_mut(&id)
            .unwrap()
            .write8(p0, p1);
    }
    impl From<Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>>
        for ::tpit_rt::Tpit<
            Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>,
        >
    {
        fn from(
            a: Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>,
        ) -> Self {
            #[link(
                wasm_import_module = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5"
            )]
            extern "C" {
                #[link_name = "~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e"]
                fn _push(
                    a: u32,
                ) -> ::tpit_rt::Tpit<
                    Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>,
                >;
            }
            return unsafe { _push(alloc(&mut *(TABLE.all.get()), a)) };
        }
    }
};
