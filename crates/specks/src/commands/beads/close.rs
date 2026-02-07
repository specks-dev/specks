//! Implementation of the `specks beads close` command

use specks_core::{BeadsCli, Config, find_project_root};

use crate::output::{JsonIssue, JsonResponse};

/// Close result data for JSON output
#[derive(Debug, serde::Serialize)]
pub struct BeadsCloseData {
    pub bead_id: String,
    pub closed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Run the beads close command
pub fn run_close(
    bead_id: String,
    reason: Option<String>,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    // Find project root
    let project_root = match find_project_root() {
        Ok(root) => root,
        Err(_) => {
            return output_error(json_output, "E009", ".specks directory not initialized", 9);
        }
    };

    // Load config
    let config = Config::load_from_project(&project_root).unwrap_or_default();
    let bd_path =
        std::env::var("SPECKS_BD_PATH").unwrap_or_else(|_| config.specks.beads.bd_path.clone());
    let beads = BeadsCli::new(bd_path);

    // Check if beads CLI is installed
    if !beads.is_installed() {
        return output_error(
            json_output,
            "E005",
            "beads CLI not installed or not found",
            5,
        );
    }

    // Attempt to close the bead
    match beads.close(&bead_id, reason.as_deref()) {
        Ok(()) => {
            let data = BeadsCloseData {
                bead_id: bead_id.clone(),
                closed: true,
                reason: reason.clone(),
            };

            if json_output {
                let response = JsonResponse::ok("beads close", data);
                println!("{}", serde_json::to_string_pretty(&response).unwrap());
            } else if !quiet {
                if let Some(r) = &reason {
                    println!("Closed bead {} (reason: {})", bead_id, r);
                } else {
                    println!("Closed bead {}", bead_id);
                }
            }

            Ok(0)
        }
        Err(e) => {
            let error_msg = format!("failed to close bead: {}", e);
            output_error(json_output, "E016", &error_msg, 16)
        }
    }
}

/// Output an error in JSON or text format
fn output_error(
    json_output: bool,
    code: &str,
    message: &str,
    exit_code: i32,
) -> Result<i32, String> {
    if json_output {
        let issues = vec![JsonIssue {
            code: code.to_string(),
            severity: "error".to_string(),
            message: message.to_string(),
            file: None,
            line: None,
            anchor: None,
        }];
        let data = BeadsCloseData {
            bead_id: String::new(),
            closed: false,
            reason: None,
        };
        let response: JsonResponse<BeadsCloseData> =
            JsonResponse::error("beads close", data, issues);
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else {
        eprintln!("error: {}", message);
    }
    Ok(exit_code)
}
