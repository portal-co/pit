pub trait R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5 {
    fn read8(&mut self, p0: u32) -> (u32);
    fn size(&mut self) -> (u32);
    fn write8(&mut self, p0: u32, p1: u32) -> ();
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
        51u8,
        50u8,
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
        51u8,
        50u8,
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
        51u8,
        50u8,
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
    impl R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5
    for ::externref::Resource<
        Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>,
    > {
        fn read8(&mut self, p0: u32) -> (u32) {
            #[::externref::externref(crate = ":: externref")]
            #[link(
                wasm_import_module = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5"
            )]
            extern "C" {
                #[link_name = "read8"]
                fn go(
                    this: &mut ::externref::Resource<
                        Box<
                            dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
                        >,
                    >,
                    p0: u32,
                ) -> (u32);
            }
            return unsafe { go(self, p0) };
        }
        fn size(&mut self) -> (u32) {
            #[::externref::externref(crate = ":: externref")]
            #[link(
                wasm_import_module = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5"
            )]
            extern "C" {
                #[link_name = "size"]
                fn go(
                    this: &mut ::externref::Resource<
                        Box<
                            dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
                        >,
                    >,
                ) -> (u32);
            }
            return unsafe { go(self) };
        }
        fn write8(&mut self, p0: u32, p1: u32) -> () {
            #[::externref::externref(crate = ":: externref")]
            #[link(
                wasm_import_module = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5"
            )]
            extern "C" {
                #[link_name = "write8"]
                fn go(
                    this: &mut ::externref::Resource<
                        Box<
                            dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
                        >,
                    >,
                    p0: u32,
                    p1: u32,
                ) -> ();
            }
            return unsafe { go(self, p0, p1) };
        }
    }
    #[::externref::externref(crate = ":: externref")]
    #[export_name = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5/~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e.drop"]
    extern "C" fn _drop(
        a: *mut Box<
            dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
        >,
    ) {
        unsafe { Box::from_raw(a) };
    }
    #[::externref::externref(crate = ":: externref")]
    #[export_name = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5/~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e/read8"]
    extern "C" fn read8(
        id: *mut Box<
            dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
        >,
        p0: u32,
    ) -> (u32) {
        return unsafe { &mut *id }.read8(p0);
    }
    #[::externref::externref(crate = ":: externref")]
    #[export_name = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5/~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e/size"]
    extern "C" fn size(
        id: *mut Box<
            dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
        >,
    ) -> (u32) {
        return unsafe { &mut *id }.size();
    }
    #[::externref::externref(crate = ":: externref")]
    #[export_name = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5/~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e/write8"]
    extern "C" fn write8(
        id: *mut Box<
            dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
        >,
        p0: u32,
        p1: u32,
    ) -> () {
        return unsafe { &mut *id }.write8(p0, p1);
    }
    impl From<Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>>
    for ::externref::Resource<
        Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>,
    > {
        fn from(
            a: Box<dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5>,
        ) -> Self {
            #[::externref::externref(crate = ":: externref")]
            #[link(
                wasm_import_module = "pit/867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5"
            )]
            extern "C" {
                #[link_name = "~8afbd04c549e07db517e034114e4b8ff8c76ce748f2e2d6e29fcaf48051eaf3e"]
                fn _push(
                    a: *mut Box<
                        dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
                    >,
                ) -> ::externref::Resource<
                    Box<
                        dyn R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5,
                    >,
                >;
            }
            return unsafe { _push(Box::into_raw(Box::new(a))) };
        }
    }
};
