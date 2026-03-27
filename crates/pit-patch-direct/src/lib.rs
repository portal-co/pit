pub mod canon;
pub mod codegen;
pub mod lower;
pub mod module;
pub mod tpit;
pub mod tutils;
pub mod util;

pub use module::DirectModule;
use pit_core::Interface;

/// Parse the `.pit-types` custom section from raw wasm bytes.
pub fn get_interfaces(bytes: &[u8]) -> anyhow::Result<Vec<Interface>> {
    let m = DirectModule::from_wasm_bytes(bytes)?;
    m.get_interfaces()
}
