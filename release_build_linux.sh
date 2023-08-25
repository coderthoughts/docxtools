#!/bin/bash +x
cargo clean
RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target x86_64-unknown-linux-gnu
echo The release can be found in ./target/x86_64-unknown-linux-gnu/release/docxtools
