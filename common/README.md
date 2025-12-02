# Common PIT Interfaces

This directory contains standard PIT interface definitions that are commonly used across projects.

## Interface Files

### buffer.pit

32-bit addressable byte buffer interface.

```pit
{
    read8(I32) -> (I32);
    write8(I32, I32) -> ();
    size() -> (I32)
}
```

**Resource ID:** `867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5`

**Methods:**
- `read8(offset: I32) -> I32` - Read a byte at the given offset
- `write8(offset: I32, value: I32) -> ()` - Write a byte at the given offset (only low 8 bits used)
- `size() -> I32` - Get the total buffer size in bytes

### buffer64.pit

64-bit addressable byte buffer interface for large buffers.

```pit
{
    read8(I64) -> (I32);
    write8(I64, I32) -> ();
    size() -> (I64)
}
```

**Resource ID:** `68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d`

**Methods:**
- `read8(offset: I64) -> I32` - Read a byte at the given 64-bit offset
- `write8(offset: I64, value: I32) -> ()` - Write a byte at the given 64-bit offset
- `size() -> I64` - Get the total buffer size in bytes (64-bit)

### reader.pit

Buffer reader interface that provides access to both 32-bit and 64-bit buffers.

```pit
{
    read(I32) -> (R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5);
    read64(I64) -> (R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d)
}
```

**Methods:**
- `read(length: I32) -> Buffer` - Read a 32-bit buffer of the given length
- `read64(length: I64) -> Buffer64` - Read a 64-bit buffer of the given length

### writer.pit

Buffer writer interface that accepts both 32-bit and 64-bit buffers.

```pit
{
    write(R867207405fe87fda620c2d7a5485e8e5e274636a898a166fb674448b4391ffc5) -> (I32);
    write64(R68da167712ddf1601aed7908c99972e62a41bdea1e28b241306a6b58d29e532d) -> (I64)
}
```

**Methods:**
- `write(buffer: Buffer) -> I32` - Write a 32-bit buffer, returns bytes written
- `write64(buffer: Buffer64) -> I64` - Write a 64-bit buffer, returns bytes written

## Usage

These interfaces can be used with the PIT CLI to generate bindings:

```bash
# Generate Rust bindings for the buffer interface
pit rust-guest common/buffer.pit buffer_bindings.rs

# Generate C header
pit gen-c common/buffer.pit buffer.h
```

## Implementations

The `pit-basic` crate provides implementations of these buffer interfaces for common Rust types. See `crates/pit-basic/` for details.
