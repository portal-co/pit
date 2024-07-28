set -e
cd $(dirname $0)
cargo run -p pit-rust-generator ../buffer.pit src/buffer/ffi.rs
cargo run -p pit-rust-generator ../buffer64.pit src/buffer64/ffi.rs