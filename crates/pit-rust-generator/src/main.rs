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
//! # Generate to file, preserving existing doc comments
//! pit-rust-generator input.pit --preserve-docs output.rs
//!
//! # Generate to stdout
//! pit-rust-generator input.pit
//! ```
//!
//! ## Options
//!
//! - `--preserve-docs` - Preserve doc comments (//!) at the top of the output file if it already exists
//!
//! ## Environment Variables
//!
//! - `PIT_SALT` - Additional salt to include in unique ID generation

use anyhow::{anyhow, Context};
use pit_rust_guest::Opts;
use quote::quote;
use syn::parse_quote;

/// Extracts doc comments from the top of a Rust source file.
/// Returns the doc comments as a string, or None if the file doesn't exist or has no doc comments.
fn extract_top_doc_comments(path: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut doc_lines = Vec::new();
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//!") {
            doc_lines.push(line.to_string());
        } else if trimmed.is_empty() {
            // Empty lines are okay between doc comments
            if !doc_lines.is_empty() {
                doc_lines.push(line.to_string());
            }
        } else {
            // Stop at first non-doc-comment, non-empty line
            break;
        }
    }
    
    if doc_lines.is_empty() {
        None
    } else {
        // Add a blank line after doc comments
        doc_lines.push(String::new());
        Some(doc_lines.join("\n"))
    }
}

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
    
    let mut preserve_docs = false;
    let dst = loop {
        let Some(arg) = a.next() else {
            break None;
        };
        if arg == "--preserve-docs" {
            preserve_docs = true;
        } else {
            break Some(arg);
        }
    };
    
    let v = pit_rust_guest::render(&x, &src);
    let v = prettyplease::unparse(&parse_quote!(#v));
    
    let Some(dst) = dst else {
        println!("{v}");
        return Ok(());
    };
    
    let output = if preserve_docs {
        if let Some(doc_comments) = extract_top_doc_comments(&dst) {
            format!("{}{}", doc_comments, v)
        } else {
            v.to_string()
        }
    } else {
        v.to_string()
    };
    
    std::fs::write(dst, output)?;
    Ok(())
}
