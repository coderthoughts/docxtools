#!/bin/bash +x
cargo clean

# The remap-path-prefix is to avoid full paths to dependency source files in case any
# code in there panics.
RUSTFLAGS="--remap-path-prefix $HOME/.cargo/registry/=./" cargo build --release
echo Note this script is only for MacOSX. For Linux please use ./release_build_linux.sh
