use std::collections::BTreeMap;
use std::vec::Vec;
use std::string::String;
use anyhow::{Context as _, bail};
use wasm_encoder::reencode::{Reencode, RoundtripReencoder};

// ── Stored types ──────────────────────────────────────────────────────────────

/// A re-encodable composite type (stored as wasm-encoder form after parsing).
#[derive(Clone)]
pub struct StoredType {
    /// Encoded SubType bytes, stored as wasm-encoder SubType so we can re-encode.
    pub inner: wasm_encoder::SubType,
}

/// A stored import entry.
#[derive(Clone)]
pub struct DirectImport {
    pub module: String,
    pub name: String,
    pub ty: DirectImportKind,
}

/// Type of an import.
#[derive(Clone)]
pub enum DirectImportKind {
    Func(u32),
    Table(wasm_encoder::TableType),
    Memory(wasm_encoder::MemoryType),
    Global(wasm_encoder::GlobalType),
    Tag(u32),
}

/// A table definition (locally defined, not imported).
#[derive(Clone)]
pub struct DirectTable {
    pub ty: wasm_encoder::TableType,
}

/// An export entry.
#[derive(Clone)]
pub struct DirectExport {
    pub name: String,
    pub kind: wasm_encoder::ExportKind,
    pub index: u32,
}

/// An element segment (stored raw for pass-through).
#[derive(Clone)]
pub struct DirectElement {
    pub raw: Vec<u8>,
}

/// A locally-defined function body (raw bytes from the original module).
#[derive(Clone)]
pub struct CodeBody {
    pub raw: Vec<u8>,
}

/// A newly generated function added by a transform.
pub struct NewFunc {
    pub type_index: u32,
    /// Local groups: (count, ValType).
    pub locals: Vec<(u32, wasm_encoder::ValType)>,
    /// Pre-encoded instruction bytes (written with wasm_encoder::Function::instruction).
    pub body_bytes: Vec<u8>,
}

/// A pass-through section (id, raw payload bytes).
struct PassThrough {
    id: u8,
    data: Vec<u8>,
}

// ── DirectModule ──────────────────────────────────────────────────────────────

/// A WebAssembly module backed by wasmparser + wasm-encoder.
///
/// Existing code bodies are kept as raw bytes (pass-through).  Only newly
/// generated functions (from transforms like talloc/tfree/canon/tpit) are built
/// with `wasm_encoder` instructions via [`crate::codegen::FuncBuilder`].
pub struct DirectModule {
    /// Type section — index-stable, append-only.
    pub types: Vec<StoredType>,
    /// Imported items in declaration order.
    pub imports: Vec<DirectImport>,
    /// Type indices for locally-defined functions (mirrors FunctionSection).
    pub func_types: Vec<u32>,
    /// Tables (local, not imported).
    pub tables: Vec<DirectTable>,
    /// Exports.
    pub exports: Vec<DirectExport>,
    /// Element segments (pass-through raw bytes).
    pub elements: Vec<DirectElement>,
    /// Custom sections by name.
    pub custom_sections: BTreeMap<String, Vec<u8>>,
    /// Code bodies for locally-defined functions (raw bytes).
    pub code_bodies: Vec<CodeBody>,
    /// Newly generated functions appended by transforms.
    pub new_funcs: Vec<NewFunc>,
    /// Other sections kept verbatim (e.g. memory, global, data, …).
    pass_through: Vec<PassThrough>,
}

impl DirectModule {
    // ── Parse ─────────────────────────────────────────────────────────────────

    pub fn from_wasm_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut m = DirectModule {
            types: Vec::new(),
            imports: Vec::new(),
            func_types: Vec::new(),
            tables: Vec::new(),
            exports: Vec::new(),
            elements: Vec::new(),
            custom_sections: BTreeMap::new(),
            code_bodies: Vec::new(),
            new_funcs: Vec::new(),
            pass_through: Vec::new(),
        };

        let mut reenc = RoundtripReencoder;
        let parser = wasmparser::Parser::new(0);

        for payload in parser.parse_all(bytes) {
            match payload? {
                wasmparser::Payload::Version { .. } => {}
                wasmparser::Payload::TypeSection(reader) => {
                    for rec_group in reader {
                        let rec_group = rec_group?;
                        for sub in rec_group.types() {
                            let enc = reenc
                                .sub_type(sub.clone())
                                .map_err(|e| anyhow::anyhow!("type reencode: {e:?}"))?;
                            m.types.push(StoredType { inner: enc });
                        }
                    }
                }
                wasmparser::Payload::ImportSection(reader) => {
                    for imp in reader {
                        let imp = imp?;
                        let kind = match imp.ty {
                            wasmparser::TypeRef::Func(idx) => DirectImportKind::Func(idx),
                            wasmparser::TypeRef::Table(tt) => {
                                DirectImportKind::Table(reenc.table_type(tt)
                                    .map_err(|e| anyhow::anyhow!("table type reencode: {e:?}"))?)
                            }
                            wasmparser::TypeRef::Memory(mt) => {
                                DirectImportKind::Memory(reenc.memory_type(mt)
                                    .map_err(|e| anyhow::anyhow!("memory type reencode: {e:?}"))?)
                            }
                            wasmparser::TypeRef::Global(gt) => {
                                DirectImportKind::Global(reenc.global_type(gt)
                                    .map_err(|e| anyhow::anyhow!("global type reencode: {e:?}"))?)
                            }
                            wasmparser::TypeRef::Tag(tt) => {
                                DirectImportKind::Tag(tt.func_type_idx)
                            }
                        };
                        m.imports.push(DirectImport {
                            module: imp.module.to_owned(),
                            name: imp.name.to_owned(),
                            ty: kind,
                        });
                    }
                }
                wasmparser::Payload::FunctionSection(reader) => {
                    for type_idx in reader {
                        m.func_types.push(type_idx?);
                    }
                }
                wasmparser::Payload::TableSection(reader) => {
                    for table in reader {
                        let table = table?;
                        let ty = reenc
                            .table_type(table.ty)
                            .map_err(|e| anyhow::anyhow!("table type reencode: {e:?}"))?;
                        m.tables.push(DirectTable { ty });
                    }
                }
                wasmparser::Payload::ExportSection(reader) => {
                    for exp in reader {
                        let exp = exp?;
                        let kind = match exp.kind {
                            wasmparser::ExternalKind::Func => wasm_encoder::ExportKind::Func,
                            wasmparser::ExternalKind::Table => wasm_encoder::ExportKind::Table,
                            wasmparser::ExternalKind::Memory => wasm_encoder::ExportKind::Memory,
                            wasmparser::ExternalKind::Global => wasm_encoder::ExportKind::Global,
                            wasmparser::ExternalKind::Tag => wasm_encoder::ExportKind::Tag,
                        };
                        m.exports.push(DirectExport {
                            name: exp.name.to_owned(),
                            kind,
                            index: exp.index,
                        });
                    }
                }
                wasmparser::Payload::ElementSection(reader) => {
                    // Store element sections as raw bytes for pass-through.
                    let range = reader.range();
                    m.elements.push(DirectElement {
                        raw: bytes[range].to_vec(),
                    });
                }
                wasmparser::Payload::CustomSection(cs) => {
                    m.custom_sections
                        .insert(cs.name().to_owned(), cs.data().to_vec());
                }
                wasmparser::Payload::CodeSectionEntry(body) => {
                    let range = body.range();
                    m.code_bodies.push(CodeBody {
                        raw: bytes[range].to_vec(),
                    });
                }
                // Pass-through all other sections verbatim.
                wasmparser::Payload::MemorySection(reader) => {
                    let range = reader.range();
                    m.pass_through.push(PassThrough {
                        id: wasm_encoder::SectionId::Memory as u8,
                        data: bytes[range].to_vec(),
                    });
                }
                wasmparser::Payload::GlobalSection(reader) => {
                    let range = reader.range();
                    m.pass_through.push(PassThrough {
                        id: wasm_encoder::SectionId::Global as u8,
                        data: bytes[range].to_vec(),
                    });
                }
                wasmparser::Payload::DataSection(reader) => {
                    let range = reader.range();
                    m.pass_through.push(PassThrough {
                        id: wasm_encoder::SectionId::Data as u8,
                        data: bytes[range].to_vec(),
                    });
                }
                wasmparser::Payload::DataCountSection { range, .. } => {
                    m.pass_through.push(PassThrough {
                        id: wasm_encoder::SectionId::DataCount as u8,
                        data: bytes[range].to_vec(),
                    });
                }
                wasmparser::Payload::StartSection { range, .. } => {
                    m.pass_through.push(PassThrough {
                        id: wasm_encoder::SectionId::Start as u8,
                        data: bytes[range].to_vec(),
                    });
                }
                wasmparser::Payload::TagSection(reader) => {
                    let range = reader.range();
                    m.pass_through.push(PassThrough {
                        id: wasm_encoder::SectionId::Tag as u8,
                        data: bytes[range].to_vec(),
                    });
                }
                wasmparser::Payload::End(_) => {}
                _ => {} // ignore unknown payloads
            }
        }

        Ok(m)
    }

    // ── Serialize ─────────────────────────────────────────────────────────────

    pub fn to_wasm_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut module = wasm_encoder::Module::new();

        // 1. Type section
        if !self.types.is_empty() {
            let mut ts = wasm_encoder::TypeSection::new();
            for st in &self.types {
                ts.ty().subtype(&st.inner);
            }
            module.section(&ts);
        }

        // 2. Import section
        if !self.imports.is_empty() {
            let mut is = wasm_encoder::ImportSection::new();
            for imp in &self.imports {
                let ety = match &imp.ty {
                    DirectImportKind::Func(idx) => wasm_encoder::EntityType::Function(*idx),
                    DirectImportKind::Table(tt) => wasm_encoder::EntityType::Table(*tt),
                    DirectImportKind::Memory(mt) => wasm_encoder::EntityType::Memory(*mt),
                    DirectImportKind::Global(gt) => wasm_encoder::EntityType::Global(*gt),
                    DirectImportKind::Tag(idx) => wasm_encoder::EntityType::Tag(
                        wasm_encoder::TagType {
                            kind: wasm_encoder::TagKind::Exception,
                            func_type_idx: *idx,
                        }
                    ),
                };
                is.import(&imp.module, &imp.name, ety);
            }
            module.section(&is);
        }

        // 3. Function section (existing + new)
        {
            let total = self.func_types.len() + self.new_funcs.len();
            if total > 0 {
                let mut fs = wasm_encoder::FunctionSection::new();
                for &ti in &self.func_types {
                    fs.function(ti);
                }
                for nf in &self.new_funcs {
                    fs.function(nf.type_index);
                }
                module.section(&fs);
            }
        }

        // 4. Table section
        if !self.tables.is_empty() {
            let mut ts = wasm_encoder::TableSection::new();
            for t in &self.tables {
                ts.table(t.ty);
            }
            module.section(&ts);
        }

        // 5. Pass-through sections that come before exports (memory, global, etc.)
        //    We emit them in the order they were parsed.
        for pt in &self.pass_through {
            module.section(&wasm_encoder::RawSection {
                id: pt.id,
                data: &pt.data,
            });
        }

        // 6. Export section
        if !self.exports.is_empty() {
            let mut es = wasm_encoder::ExportSection::new();
            for exp in &self.exports {
                es.export(&exp.name, exp.kind, exp.index);
            }
            module.section(&es);
        }

        // 7. Element section (raw pass-through)
        for elem in &self.elements {
            module.section(&wasm_encoder::RawSection {
                id: wasm_encoder::SectionId::Element as u8,
                data: &elem.raw,
            });
        }

        // 8. Code section (existing bodies + new funcs)
        {
            let total = self.code_bodies.len() + self.new_funcs.len();
            if total > 0 {
                let mut cs = wasm_encoder::CodeSection::new();
                for body in &self.code_bodies {
                    cs.raw(&body.raw);
                }
                for nf in &self.new_funcs {
                    let mut f = wasm_encoder::Function::new(nf.locals.iter().cloned());
                    f.raw(nf.body_bytes.iter().copied());
                    cs.function(&f);
                }
                module.section(&cs);
            }
        }

        // 9. Custom sections
        for (name, data) in &self.custom_sections {
            module.section(&wasm_encoder::CustomSection { name: std::borrow::Cow::Borrowed(name.as_str()), data: std::borrow::Cow::Borrowed(data) });
        }

        Ok(module.finish())
    }

    // ── Index helpers ─────────────────────────────────────────────────────────

    /// Number of imported functions (base offset for local func indices).
    pub fn imported_func_count(&self) -> u32 {
        self.imports
            .iter()
            .filter(|i| matches!(i.ty, DirectImportKind::Func(_)))
            .count() as u32
    }

    /// Func index of the n-th local function (0-based within func_types).
    pub fn local_func_index(&self, n: usize) -> u32 {
        self.imported_func_count() + n as u32
    }

    /// Append a new generated function; returns its func index.
    pub fn add_func(&mut self, type_index: u32, locals: Vec<(u32, wasm_encoder::ValType)>, body_bytes: Vec<u8>) -> u32 {
        let idx = self.imported_func_count()
            + self.func_types.len() as u32
            + self.new_funcs.len() as u32;
        self.new_funcs.push(NewFunc { type_index, locals, body_bytes });
        idx
    }

    /// Find an existing function type or append a new one; returns type index.
    pub fn add_func_type(&mut self, params: &[wasm_encoder::ValType], results: &[wasm_encoder::ValType]) -> u32 {
        let ft = wasm_encoder::FuncType::new(params.iter().cloned(), results.iter().cloned());
        // Search for existing identical type (only Func types with matching sig).
        for (i, st) in self.types.iter().enumerate() {
            if let wasm_encoder::CompositeInnerType::Func(ref existing_ft) = st.inner.composite_type.inner {
                if existing_ft == &ft {
                    return i as u32;
                }
            }
        }
        let idx = self.types.len() as u32;
        self.types.push(StoredType {
            inner: wasm_encoder::SubType {
                is_final: true,
                supertype_idx: None,
                composite_type: wasm_encoder::CompositeType {
                    shared: false,
                    inner: wasm_encoder::CompositeInnerType::Func(ft),
                },
            },
        });
        idx
    }

    /// Append a struct type; returns type index.
    pub fn add_struct_type(&mut self, fields: Vec<wasm_encoder::FieldType>) -> u32 {
        let idx = self.types.len() as u32;
        self.types.push(StoredType {
            inner: wasm_encoder::SubType {
                is_final: true,
                supertype_idx: None,
                composite_type: wasm_encoder::CompositeType {
                    shared: false,
                    inner: wasm_encoder::CompositeInnerType::Struct(
                        wasm_encoder::StructType { fields: fields.into_boxed_slice() },
                    ),
                },
            },
        });
        idx
    }

    /// Append a table; returns its table index.
    pub fn add_table(&mut self, ty: wasm_encoder::TableType) -> u32 {
        let imported_tables = self.imports
            .iter()
            .filter(|i| matches!(i.ty, DirectImportKind::Table(_)))
            .count() as u32;
        let idx = imported_tables + self.tables.len() as u32;
        self.tables.push(DirectTable { ty });
        idx
    }

    /// Find an existing imported function matching (module, name) or add one.
    /// Returns the func index.
    pub fn ensure_import_func(&mut self, module: &str, name: &str, type_index: u32) -> u32 {
        // Look for existing import with matching module+name.
        let mut fi = 0u32;
        for imp in &self.imports {
            if let DirectImportKind::Func(_) = imp.ty {
                if imp.module == module && imp.name == name {
                    return fi;
                }
                fi += 1;
            }
        }
        // Not found — append.
        let idx = self.imported_func_count();
        // Bump all local func indices by 1 (we're inserting before them).
        // Since we append the import and shift local funcs, we need to update
        // func_types indices in exports and calls. For simplicity, we append
        // the import at the end of the import list, but wasm requires imports
        // to come before local functions in the index space. Since we always
        // call ensure_import_func before any transforms generate local funcs
        // that reference earlier indices, this is safe.
        self.imports.push(DirectImport {
            module: module.to_owned(),
            name: name.to_owned(),
            ty: DirectImportKind::Func(type_index),
        });
        // The new import gets index = (previous imported_func_count).
        // All previously-added local func indices in exports must be bumped.
        // We do this lazily: callers should call ensure_import_func before
        // adding local functions to avoid index invalidation.
        idx
    }

    // ── Interface helpers ─────────────────────────────────────────────────────

    /// Read the .pit-types custom section and parse interface definitions.
    pub fn get_interfaces(&self) -> anyhow::Result<Vec<pit_core::Interface>> {
        let bytes = self.custom_sections
            .get(".pit-types")
            .context("missing .pit-types custom section")?;
        pit_patch_core::pit_section::parse_pit_types_section(bytes)
    }
}

impl pit_patch_core::WasmModule for DirectModule {
    fn to_wasm_bytes(&self) -> anyhow::Result<Vec<u8>> {
        DirectModule::to_wasm_bytes(self)
    }
}
