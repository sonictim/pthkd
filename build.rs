fn main() {
    // Tell Cargo where to find the Swift library
    println!("cargo:rustc-link-search=native=target/release");
    println!("cargo:rustc-link-search=native=swift/.build/x86_64-apple-macosx/release");

    // Set rpath so the executable can find the dylib at runtime
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");

    // Force rebuild if Swift library changes
    println!("cargo:rerun-if-changed=target/release/libPTHKDui.dylib");
    println!("cargo:rerun-if-changed=swift/.build/x86_64-apple-macosx/release/libPTHKDui.dylib");
}
