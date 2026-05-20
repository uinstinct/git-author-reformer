// This build.rs exists solely to re-emit the native link flags for libgit2 so that
// integration test binaries can find the C library. Cargo's link-flag propagation for
// integration tests requires the package's own build.rs to re-emit native deps when
// those deps come from transitive build scripts.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
}
