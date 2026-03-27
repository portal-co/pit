#![no_std]
extern crate alloc;

pub mod jigger;
pub mod names;
pub mod pit_section;
pub mod types;

/// Pluggable module serialization trait.
///
/// Both the waffle backend (`pit-patch`) and the direct backend (`pit-patch-direct`)
/// implement this so that operations like `jigger` can hash module bytes without
/// depending on a specific backend.
pub trait WasmModule {
    fn to_wasm_bytes(&self) -> anyhow::Result<alloc::vec::Vec<u8>>;
}
