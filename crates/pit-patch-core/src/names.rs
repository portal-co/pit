use alloc::string::String;
use alloc::format;

/// Returns `"pit/{rid}"` — the wasm import module name for a PIT interface.
pub fn pit_module_name(rid: &str) -> String {
    format!("pit/{rid}")
}

/// Returns `"tpit/{rid}"` — the wasm import module name for a TPIT interface.
pub fn tpit_module_name(rid: &str) -> String {
    format!("tpit/{rid}")
}

/// Returns `"~{name}"` — the import name for a constructor.
pub fn constructor_import_name(name: &str) -> String {
    format!("~{name}")
}

/// Returns `"pit/{rid}/~{ctor}/{method}"` — the export name for an interface method.
pub fn method_export_name(rid: &str, ctor: &str, method: &str) -> String {
    format!("pit/{rid}/~{ctor}/{method}")
}

/// Returns `"pit/{rid}/~{ctor}.drop"` — the export name for an interface destructor.
pub fn drop_export_name(rid: &str, ctor: &str) -> String {
    format!("pit/{rid}/~{ctor}.drop")
}
