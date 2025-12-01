//! # PIT Rust Generator
//!
//! Standalone executable for generating Rust code from PIT interface files.
//!
//! This is a command-line wrapper around [`pit_rust_guest`] that reads a PIT
//! interface file, generates Rust code, and writes it to a file or stdout.
//!
//! ## Usage
//!
//! ```bash
//! # Generate to file
//! pit-rust-generator input.pit output.rs
//!
//! # Generate to stdout
//! pit-rust-generator input.pit
//! ```
//!
//! ## Environment Variables
//!
//! - `PIT_SALT` - Additional salt to include in unique ID generation

use anyhow::{anyhow, Context};
use pit_rust_guest::Opts;
use quote::quote;
use syn::parse_quote;
fn main() -> anyhow::Result<()> {
    let mut a = std::env::args();
    a.next();
    let src = a.next().context("in getting the src")?;
    let src = std::fs::read_to_string(src)?;
    let (_, src) =
        pit_core::parse_interface(&src).map_err(|e| anyhow!("invalid interface: {e}"))?;
    let mut x = Opts {
        root: quote! {},
        salt: std::env::var("PIT_SALT")
            .unwrap_or(format!(""))
            .into_bytes(),
        tpit: true,
    };
    let v = pit_rust_guest::render(&x, &src);
    let v = prettyplease::unparse(&parse_quote!(#v));
    let Some(dst) = a.next() else {
        println!("{v}");
        return Ok(());
    };
    std::fs::write(dst, v.to_string())?;
    Ok(())
}
