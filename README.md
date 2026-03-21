# PIT - Portal Interface Types for WebAssembly

PIT (Portal Interface Types) is a WebAssembly interface definition language (IDL) and toolchain for defining and implementing cross-module interfaces. It provides a structured format for describing interfaces, methods, and resource types, and a set of tools to generate language bindings and transform WebAssembly modules to conform to the PIT ABI.

The core parsing library and IDL specification live in a separate crate: [pit-core](https://github.com/portal-co/pit-core).

**Current status:** Alpha (version `0.5.0-alpha.1`). The project is under active development. ABI v1 is functional. ABI v2 (`pitx`) is a work in progress with no stable implementation yet.

## What it does

PIT lets you define an interface as a set of typed methods operating on resources. Given an interface definition, the toolchain can:

- Generate Rust guest bindings (code a `.wasm` module compiles against)
- Generate Rust host bindings (code to instantiate and call into a PIT module)
- Generate C header files
- Generate Scala/TeaVM bindings for JVM-to-WebAssembly targets
- Transform WebAssembly binary modules: lower `externref` to i32 table indices, convert TPIT intermediate format to PIT ABI v1, canonicalize and jigger unique resource IDs
- Embed interface type metadata into a module's custom sections

## Interface format

PIT interfaces are plain text files:

```pit
{
    method_name(param_types) -> (return_types);
    another_method(I32, I64) -> (F64);
}
```

### Argument types

- `I32`, `I64` - 32/64-bit integers
- `F32`, `F64` - 32/64-bit floats
- `R<resource_id>` - owned resource reference
- `R<resource_id>n` - nullable resource reference
- `R<resource_id>&` - borrowed resource reference

Resource IDs are SHA3-based hex hashes derived from the interface content.

### Attributes

```pit
[version=1]{
    [async]get_data(I32) -> (R<buffer>);
}
```

For the full format specification, see [SPEC.md in pit-core](https://github.com/portal-co/pit-core/blob/main/SPEC.md).

## ABI versions

### ABI v1

Resources are identified by a content-derived unique ID. The ABI uses `externref` for resource handles.

**Module exports** follow the pattern:
```
pit/<resource id>/~<unique id>/<method>
pit/<resource id>/~<unique id>.drop
```

**Module imports** follow the pattern:
```
pit/<resource id>.~<method>
pit.drop
```

### ABI v2 (work in progress)

All ABI v2 import modules and exports start with `pitx`. No stable definition yet.

## TPIT (Tablified PIT)

Many WebAssembly toolchains (including LLVM/Rust targeting `wasm32-unknown-unknown`) do not natively support `externref`. TPIT is an intermediate binary format that replaces `externref` handles with integer indices into a WebAssembly table. The `pit untpit` command converts a TPIT module into a proper PIT ABI v1 module.

### Compilation flow for Rust guests

```
Rust source code
     │ compile with TPIT bindings (tpit-rt)
     ▼
TPIT module (.wasm)       ← uses tpit/* imports, i32 table handles
     │ pit untpit
     ▼
PIT ABI v1 module (.wasm) ← uses pit/* imports, externref handles
```

1. Generate TPIT Rust bindings from a PIT interface:
   ```bash
   pit rust-guest interface.pit bindings.rs
   ```

2. Compile the Rust crate (with `tpit-rt` as a dependency):
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

3. Convert the TPIT output to PIT ABI v1:
   ```bash
   pit untpit target/wasm32-unknown-unknown/release/my_module.wasm output.wasm
   ```

## Common interfaces (`common/`)

Standard PIT interface definitions included in the repo:

| File | Resource ID | Description |
|------|-------------|-------------|
| `buffer.pit` | `867207405f...` | 32-bit addressable byte buffer (`read8`, `write8`, `size`) |
| `buffer64.pit` | `68da167712...` | 64-bit addressable byte buffer |
| `reader.pit` | — | Produces `buffer` or `buffer64` resources |
| `writer.pit` | — | Consumes `buffer` or `buffer64` resources |

## Crates

### Core tooling

| Crate | Description |
|-------|-------------|
| `pit-cli` | Command-line interface for all PIT operations |
| `pit-patch` | WebAssembly module transformation (TPIT unwrap, externref lowering, canonicalization, embedding interface metadata) |

### Rust code generation

| Crate | Description |
|-------|-------------|
| `pit-rust-guest` | Generates Rust code for guest modules (uses `tpit-rt` or `externref`) |
| `pit-rust-host` | Generates Rust code for host environments |
| `pit-rust-host-core` | Shared utilities for host code generation |
| `pit-rust-host-lib` | Runtime support for hosting PIT modules (uses `wasm_runtime_layer`) |
| `pit-rust-externref` | Configures the `externref` crate's processor for PIT's drop function |
| `pit-rust-generator` | Standalone binary wrapping `pit-rust-guest` generation |

### Runtime libraries

| Crate | Description |
|-------|-------------|
| `tpit-rt` | TPIT runtime: the `Tpit<D>` type wrapping an i32 table handle with RAII drop via `tpit.drop` import |
| `pit-basic` | Implementations of the standard buffer interfaces for `Vec<u8>`, `Box<[u8]>`, slices |

### Other language targets

| Crate | Description |
|-------|-------------|
| `pit-c` | Generates C headers using `interface99` and `handle.h` conventions |
| `pit-teavm` | Generates Scala code for TeaVM (JVM-to-WebAssembly) consumption of PIT interfaces |
| `pit-wit-bridge` | Bridge between PIT and WIT (WebAssembly Interface Types); currently nearly empty |

## CLI reference

```bash
# Generate Rust guest bindings (uses TPIT by default)
pit rust-guest <input.pit> <output.rs>
pit rust-guest <input.pit> --extern-externref <output.rs>   # use externref directly
pit rust-guest <input.pit> --preserve-docs <output.rs>      # keep existing doc comments
pit rust-guest <input.pit> --salt <bytes> <output.rs>

# Generate other language bindings
pit gen-c <input.pit> <output.h>
pit teavm [-pkg <package>] <input.pit> <output.scala>

# Generate a complete multi-language package directory
# (Rust crate, Scala file, C header, Cargo.toml, BUILD.bazel, CMakeLists.txt)
pit package <input.pit> <output-dir>

# WebAssembly module transformations
pit untpit <input.wasm> <output.wasm>      # convert TPIT to PIT ABI v1
pit lower <input.wasm> <output.wasm>        # lower externref to i32 table indices
pit jigger <input.wasm> <output.wasm>       # regenerate unique IDs based on content
pit embed [-<interface.pit>...] <input.wasm> <output.wasm>  # embed interface metadata

# Utilities
pit hash <input.pit> [<input2.pit> ...]    # print resource ID (SHA3 hash) of each interface
```

## Dependencies

The workspace depends on:
- `portal-pc-waffle` — fork of the [waffle](https://github.com/bytecodealliance/waffle) WebAssembly IR library, used for module analysis and transformation
- `pit-core` — the PIT parser and core data structures

## Building

```bash
cargo build --release
```

Note: `portal-pc-waffle` is pulled from a git repository. The workspace `Cargo.toml` has a known typo in that URL (`httpsd://`) which must be corrected before the build will work.

## License

CC0-1.0 (Public Domain)
