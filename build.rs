fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate protobuf code
    tonic_prost_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["proto/PTSL.proto"], // Files to compile
            &["proto/"],           // Include directories
        )?;

    // Determine architecture-specific paths
    let target = std::env::var("TARGET").unwrap_or_else(|_| String::from("unknown"));

    let (swift_arch, rust_target_dir) = if target.contains("aarch64") {
        ("arm64-apple-macosx", "target/aarch64-apple-darwin/release")
    } else if target.contains("x86_64") {
        ("x86_64-apple-macosx", "target/x86_64-apple-darwin/release")
    } else {
        ("x86_64-apple-macosx", "target/release")
    };

    // Tell Cargo where to find the Swift library for this architecture
    println!("cargo:rustc-link-search=native={}", rust_target_dir);
    println!("cargo:rustc-link-search=native=swift/.build/{}/release", swift_arch);

    // Set rpath so the executable can find the dylib at runtime
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");

    // Force rebuild if Swift library changes
    println!("cargo:rerun-if-changed={}/libPTHKDui.dylib", rust_target_dir);
    println!("cargo:rerun-if-changed=swift/.build/{}/release/libPTHKDui.dylib", swift_arch);

    Ok(())
}
