//! # PIT Externref Processor Setup
//!
//! This crate provides configuration for the externref processor to work with PIT interfaces.
//!
//! When using externref-based PIT (as opposed to TPIT), WebAssembly modules need to have
//! their externref operations processed. This crate configures the processor to recognize
//! PIT's drop function.
//!
//! ## Usage
//!
//! ```ignore
//! use externref::processor::Processor;
//! use pit_rust_externref::setup;
//!
//! let mut processor = Processor::new(&wasm_bytes);
//! setup(&mut processor);
//! let processed = processor.process()?;
//! ```

/// Configures an externref processor for PIT compatibility.
///
/// This function sets up the processor to recognize the PIT drop function
/// at `pit.drop`, which is used to release resources.
///
/// # Arguments
///
/// * `x` - A mutable reference to the externref processor
///
/// # Returns
///
/// Returns the same processor reference for method chaining.
pub fn setup<'a, 'b>(
    x: &'a mut externref::processor::Processor<'b>,
) -> &'a mut externref::processor::Processor<'b> {
    x.set_drop_fn("pit", "drop")
}
