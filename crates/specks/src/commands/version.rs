//! Implementation of the `specks version` command
//!
//! Shows package version and optionally extended build information.
//! Per [D01], this is a subcommand rather than extending --version flag behavior.

/// Run the version command
///
/// # Arguments
/// * `verbose` - Show extended build information (commit, date, rustc)
/// * `json_output` - Output in JSON format
/// * `quiet` - Suppress non-error output
pub fn run_version(verbose: bool, json_output: bool, quiet: bool) -> Result<i32, String> {
    let version = env!("CARGO_PKG_VERSION");
    let commit = env!("SPECKS_COMMIT");
    let build_date = env!("SPECKS_BUILD_DATE");
    let rustc_version = env!("SPECKS_RUSTC_VERSION");

    if quiet {
        return Ok(0);
    }

    if json_output {
        // JSON output always includes all fields regardless of --verbose
        // Full implementation comes in Step 1
        println!(
            r#"{{"status":"ok","schema_version":"1","version":"{}","commit":"{}","build_date":"{}","rustc_version":"{}"}}"#,
            version, commit, build_date, rustc_version
        );
    } else if verbose {
        println!("specks {}", version);
        println!("  commit:     {}", commit);
        println!("  built:      {}", build_date);
        println!("  rustc:      {}", rustc_version);
    } else {
        println!("specks {}", version);
    }

    Ok(0)
}
