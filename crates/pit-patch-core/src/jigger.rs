use alloc::string::String;
use sha3::{Digest, Sha3_256};

/// A simplified import entry for jigger renaming.
pub struct ImportEntry {
    pub module: String,
    pub name: String,
}

/// A simplified export entry for jigger renaming.
pub struct ExportEntry {
    pub name: String,
}

/// Compute the jigger hash: SHA3-256 of `module_bytes || seed`.
pub fn jigger_hash(module_bytes: &[u8], seed: &[u8]) -> [u8; 32] {
    let mut h = Sha3_256::default();
    h.update(module_bytes);
    h.update(seed);
    h.finalize().into()
}

/// Apply jigger renames to a flat list of import/export entries.
///
/// - Imports in `pit/*` modules with names starting `~` get the name replaced
///   with `~{sha3(original_without_tilde + "-" + hash_hex)}`
/// - Exports of the form `pit/{rid}/~{ctor}/{method}` get the ctor portion
///   replaced with `{sha3(ctor + "-" + hash_hex)}`
pub fn apply_jigger(imports: &mut [ImportEntry], exports: &mut [ExportEntry], hash: &[u8; 32]) {
    let hash_hex = hex::encode(hash);

    for i in imports.iter_mut() {
        if !i.module.starts_with("pit/") {
            continue;
        }
        if let Some(base) = i.name.strip_prefix("~") {
            let combined = alloc::format!("{base}-{hash_hex}");
            let mut h = Sha3_256::default();
            h.update(combined.as_bytes());
            let digest = hex::encode(h.finalize());
            i.name = alloc::format!("~{digest}");
        }
    }

    for x in exports.iter_mut() {
        if let Some(rest) = x.name.strip_prefix("pit/") {
            if let Some((rid, after_tilde)) = rest.split_once("/~") {
                if let Some((ctor, method)) = after_tilde.split_once('/') {
                    let combined = alloc::format!("{ctor}-{hash_hex}");
                    let mut h = Sha3_256::default();
                    h.update(combined.as_bytes());
                    let digest = hex::encode(h.finalize());
                    x.name = alloc::format!("pit/{rid}/~{digest}/{method}");
                }
            }
        }
    }
}
