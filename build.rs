fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Clear log file on recompile (fresh logs for new builds)
    let _ = std::fs::remove_file("macrod.log");

    // Link to libdispatch (Grand Central Dispatch) on macOS
    // Note: dispatch is part of System.framework umbrella framework
    #[cfg(target_os = "macos")]
    {
        // No need to explicitly link - dispatch symbols are in System which is already linked
        // Just leaving this comment for documentation
    }

    tonic_prost_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &["proto/PTSL.proto"], // Files to compile
            &["proto/"],           // Include directories
        )?;
    Ok(())
}
