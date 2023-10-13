#!/bin/bash +x
cargo clean

# The remap-path-prefix is to avoid full paths to dependency source files in case any
# code in there panics.
# The crt-static is to statically link dependencies which makes the linux executable 
# more portable.
RUSTFLAGS="-C target-feature=+crt-static --remap-path-prefix $HOME/.cargo/registry/=./" cargo build --release --target x86_64-unknown-linux-gnu
echo The release can be found in ./target/x86_64-unknown-linux-gnu/release/docxtools
