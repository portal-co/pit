# PIT - Portal Interface Types for WebAssembly

PIT (Portal Interface Types) is a WebAssembly Interface Definition Language (IDL) and toolkit for defining and implementing cross-module interfaces. It provides a structured format for describing interfaces, methods, arguments, resources, and attributes suitable for WebAssembly module interoperability.

For the core PIT IDL specification and library, see [pit-core](https://github.com/portal-co/pit-core).

## Overview

This repository contains the implementation tooling for PIT, including:

- **Code generators** for Rust, C, Scala/TeaVM
- **WebAssembly module patching** utilities
- **Runtime libraries** for both guest and host environments
- **CLI tools** for working with PIT interfaces

## Crates

### Core Tools

| Crate | Description |
|-------|-------------|
| [`pit-cli`](crates/pit-cli) | Command-line interface for PIT operations |
| [`pit-patch`](crates/pit-patch) | WebAssembly module patching and transformation |

### Rust Integration

| Crate | Description |
|-------|-------------|
| [`pit-rust-guest`](crates/pit-rust-guest) | Rust code generation for guest modules |
| [`pit-rust-host`](crates/pit-rust-host) | Rust code generation for host environments |
| [`pit-rust-host-core`](crates/pit-rust-host-core) | Core host code generation utilities |
| [`pit-rust-host-lib`](crates/pit-rust-host-lib) | Runtime library for hosting PIT modules |
| [`pit-rust-externref`](crates/pit-rust-externref) | Externref processor configuration |
| [`pit-rust-generator`](crates/pit-rust-generator) | Standalone Rust code generator |

### Runtime Libraries

| Crate | Description |
|-------|-------------|
| [`tpit-rt`](crates/tpit-rt) | Tablified PIT runtime (table-based externref emulation) |
| [`pit-basic`](crates/pit-basic) | Basic buffer interface implementations |

### Other Language Targets

| Crate | Description |
|-------|-------------|
| [`pit-c`](crates/pit-c) | C header file generation |
| [`pit-teavm`](crates/pit-teavm) | Scala code generation for TeaVM |
| [`pit-wit-bridge`](crates/pit-wit-bridge) | Bridge between PIT and WIT (WebAssembly Interface Types) |

## ABI Versions

### ABI v1

Uses unique IDs for resource identification:

**Exports:**
```
pit/<resource id>/~<unique id>/<method>
```

Drop methods are implemented as:
```
pit/<resource id>/~<unique id>.drop
```

**Imports:**
```
pit/<resource id>.~<method>
```

Drop methods are called via `pit.drop`

### ABI v2 (WIP)

All ABI v2 import modules and exports start with `pitx`. This version is still work in progress.

## TPIT (Tablified PIT)

TPIT is an intermediate binary format used in the compilation pipeline for ABI v1. It uses WebAssembly tables to emulate `externref` on platforms that don't support it natively, allowing code to be written using table-based handles that are later converted to proper PIT ABI v1 externref calls.

### Compilation Flow

The typical workflow for compiling Rust or LLVM-based code to PIT ABI v1:

```
┌─────────────────┐
│  Rust / LLVM    │
│  Source Code    │
└────────┬────────┘
         │ compile with TPIT bindings
         ▼
┌─────────────────┐
│   TPIT Module   │  (uses tpit/* imports, table-based handles)
│   (.wasm)       │
└────────┬────────┘
         │ pit untpit
         ▼
┌─────────────────┐
│   PIT ABI v1    │  (uses pit/* imports, externref-based)
│   Module        │
└─────────────────┘
```

### Example: Rust to PIT ABI v1

1. **Generate TPIT bindings** from a PIT interface:
   ```bash
   pit rust-guest interface.pit bindings.rs
   ```

2. **Compile Rust code** that uses the generated bindings:
   ```bash
   cargo build --target wasm32-unknown-unknown --release
   ```

3. **Convert TPIT to PIT ABI v1**:
   ```bash
   pit untpit target/wasm32-unknown-unknown/release/my_module.wasm output.wasm
   ```

The resulting `output.wasm` uses PIT ABI v1 with proper `externref` handling.

## Common Interfaces

The `common/` directory contains standard PIT interface definitions:

| File | Description |
|------|-------------|
| `buffer.pit` | 32-bit addressable byte buffer interface |
| `buffer64.pit` | 64-bit addressable byte buffer interface |
| `reader.pit` | Buffer reader interface |
| `writer.pit` | Buffer writer interface |

## Usage

### CLI Commands

```bash
# Generate Rust guest bindings
pit rust-guest <input.pit> <output.rs>

# Generate Rust guest bindings, preserving existing doc comments
pit rust-guest <input.pit> --preserve-docs <output.rs>

# Generate C header
pit gen-c <input.pit> <output.h>

# Generate TeaVM/Scala bindings
pit teavm <input.pit> <output.scala>

# Generate complete package with all bindings
pit package <input.pit> <output-dir>

# Lower PIT module (remove externref)
pit lower <input.wasm> <output.wasm>

# Unwrap TPIT to PIT
pit untpit <input.wasm> <output.wasm>

# Compute interface hash
pit hash <input.pit>

# Embed interface types in module
pit embed <input.wasm> <output.wasm>

# Jigger unique IDs
pit jigger <input.wasm> <output.wasm>
```

## PIT Format Overview

PIT interfaces are defined using a structured text format:

```pit
{
    method_name(param_types) -> (return_types);
    another_method(I32, I64) -> (F64);
}
```

### Argument Types

- `I32`, `I64` - 32/64-bit integers
- `F32`, `F64` - 32/64-bit floats
- `R<resource_id>` - Resource reference
- `R<resource_id>n` - Nullable resource
- `R<resource_id>&` - Borrowed resource reference

### Attributes

Annotations can be added with `[key=value]` syntax:

```pit
[version=1]{
    [async]get_data(I32) -> (R<buffer>);
}
```

For the complete format specification, see [SPEC.md in pit-core](https://github.com/portal-co/pit-core/blob/main/SPEC.md).

## Building

```bash
cargo build --release
```

## License

CC0-1.0 (Public Domain)

## Goals
- [ ] Add project goals

## Progress
- [ ] Initial setup

---
*AI assisted*
