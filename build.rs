fn main() {
    // This is needed to be able to test the println! output that some code produces
    println!("cargo:rustc-env=RUST_TEST_NOCAPTURE=1");
}
