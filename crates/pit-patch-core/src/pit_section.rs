use alloc::vec::Vec;
use pit_core::Interface;

/// Parse the contents of a `.pit-types` custom section into interface definitions.
///
/// The section is a sequence of null-byte-separated UTF-8 interface strings.
/// Empty trailing slices (after the last `\0`) are skipped.
pub fn parse_pit_types_section(bytes: &[u8]) -> anyhow::Result<Vec<Interface>> {
    let mut interfaces = Vec::new();
    for chunk in bytes.split(|b| *b == 0) {
        if chunk.is_empty() {
            continue;
        }
        let s = core::str::from_utf8(chunk)?;
        let (_rest, iface) = pit_core::parse_interface(s)
            .map_err(|_e: nom::Err<nom::error::Error<&str>>| anyhow::anyhow!("invalid pit interface"))?;
        interfaces.push(iface);
    }
    Ok(interfaces)
}
