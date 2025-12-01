fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Clear log file on recompile (fresh logs for new builds)
    let _ = std::fs::remove_file("macrod.log");

    tonic_prost_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["proto/PTSL.proto"], // Files to compile
            &["proto/"],           // Include directories
        )?;
    Ok(())
}
