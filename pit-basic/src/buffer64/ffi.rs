pub trait R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d {
    fn read8(&mut self, p0: u64) -> (u32);
    fn size(&mut self) -> (u64);
    fn write8(&mut self, p0: u64, p1: u32) -> ();
}
const _: () = {
    #[link_section = ".pit-types"]
    static SECTION_CONTENT: [u8; 60usize] = [
        123u8,
        114u8,
        101u8,
        97u8,
        100u8,
        56u8,
        40u8,
        73u8,
        54u8,
        52u8,
        41u8,
        32u8,
        45u8,
        62u8,
        32u8,
        40u8,
        73u8,
        51u8,
        50u8,
        41u8,
        59u8,
        115u8,
        105u8,
        122u8,
        101u8,
        40u8,
        41u8,
        32u8,
        45u8,
        62u8,
        32u8,
        40u8,
        73u8,
        54u8,
        52u8,
        41u8,
        59u8,
        119u8,
        114u8,
        105u8,
        116u8,
        101u8,
        56u8,
        40u8,
        73u8,
        54u8,
        52u8,
        44u8,
        73u8,
        51u8,
        50u8,
        41u8,
        32u8,
        45u8,
        62u8,
        32u8,
        40u8,
        41u8,
        125u8,
        0u8,
    ];
    impl R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d
    for ::externref::Resource<
        Box<dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d>,
    > {
        fn read8(&mut self, p0: u64) -> (u32) {
            #[::externref::externref(crate = ":: externref")]
            #[link(
                wasm_import_module = "pit/68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d"
            )]
            extern "C" {
                #[link_name = "read8"]
                fn go(
                    this: &mut ::externref::Resource<
                        Box<
                            dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
                        >,
                    >,
                    p0: u64,
                ) -> (u32);
            }
            return unsafe { go(self, p0) };
        }
        fn size(&mut self) -> (u64) {
            #[::externref::externref(crate = ":: externref")]
            #[link(
                wasm_import_module = "pit/68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d"
            )]
            extern "C" {
                #[link_name = "size"]
                fn go(
                    this: &mut ::externref::Resource<
                        Box<
                            dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
                        >,
                    >,
                ) -> (u64);
            }
            return unsafe { go(self) };
        }
        fn write8(&mut self, p0: u64, p1: u32) -> () {
            #[::externref::externref(crate = ":: externref")]
            #[link(
                wasm_import_module = "pit/68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d"
            )]
            extern "C" {
                #[link_name = "write8"]
                fn go(
                    this: &mut ::externref::Resource<
                        Box<
                            dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
                        >,
                    >,
                    p0: u64,
                    p1: u32,
                ) -> ();
            }
            return unsafe { go(self, p0, p1) };
        }
    }
    #[::externref::externref(crate = ":: externref")]
    #[export_name = "pit/68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d/~b3435bc747738a8874da05bf54d1e6b7c57bbab9ef27b0d40a5db3c3c8b6e5b9.drop"]
    extern "C" fn _drop(
        a: *mut Box<
            dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
        >,
    ) {
        unsafe { Box::from_raw(a) };
    }
    #[::externref::externref(crate = ":: externref")]
    #[export_name = "pit/68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d/~b3435bc747738a8874da05bf54d1e6b7c57bbab9ef27b0d40a5db3c3c8b6e5b9/read8"]
    extern "C" fn read8(
        id: *mut Box<
            dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
        >,
        p0: u64,
    ) -> (u32) {
        return unsafe { &mut *id }.read8(p0);
    }
    #[::externref::externref(crate = ":: externref")]
    #[export_name = "pit/68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d/~b3435bc747738a8874da05bf54d1e6b7c57bbab9ef27b0d40a5db3c3c8b6e5b9/size"]
    extern "C" fn size(
        id: *mut Box<
            dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
        >,
    ) -> (u64) {
        return unsafe { &mut *id }.size();
    }
    #[::externref::externref(crate = ":: externref")]
    #[export_name = "pit/68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d/~b3435bc747738a8874da05bf54d1e6b7c57bbab9ef27b0d40a5db3c3c8b6e5b9/write8"]
    extern "C" fn write8(
        id: *mut Box<
            dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
        >,
        p0: u64,
        p1: u32,
    ) -> () {
        return unsafe { &mut *id }.write8(p0, p1);
    }
    impl From<Box<dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d>>
    for ::externref::Resource<
        Box<dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d>,
    > {
        fn from(
            a: Box<dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d>,
        ) -> Self {
            #[::externref::externref(crate = ":: externref")]
            #[link(
                wasm_import_module = "pit/68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d"
            )]
            extern "C" {
                #[link_name = "~b3435bc747738a8874da05bf54d1e6b7c57bbab9ef27b0d40a5db3c3c8b6e5b9"]
                fn _push(
                    a: *mut Box<
                        dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
                    >,
                ) -> ::externref::Resource<
                    Box<
                        dyn R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d,
                    >,
                >;
            }
            return unsafe { _push(Box::into_raw(Box::new(a))) };
        }
    }
};
