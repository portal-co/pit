set -e
cd $(dirname $0)
cargo run -p pit-rust-generator ../buffer.pit src/buffer.rs