//! Integration tests for specks-core
//!
//! These tests validate the parser and validator against fixture files.

use specks_core::{parse_speck, validate_speck, Severity};
use std::fs;

const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");
const GOLDEN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures/golden");

#[test]
fn test_valid_minimal_fixture() {
    let content = fs::read_to_string(format!("{}/valid/minimal.md", FIXTURES_DIR))
        .expect("Failed to read minimal.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse minimal speck");
    let result = validate_speck(&speck);

    // Check no errors
    let errors: Vec<_> = result.issues.iter().filter(|i| i.severity == Severity::Error).collect();
    assert!(
        errors.is_empty(),
        "Valid minimal speck should have no errors, got: {:?}",
        errors
    );
}

#[test]
fn test_invalid_missing_metadata_fixture() {
    let content = fs::read_to_string(format!("{}/invalid/missing-metadata.md", FIXTURES_DIR))
        .expect("Failed to read missing-metadata.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse speck");
    let result = validate_speck(&speck);

    // Should have E002 errors for missing metadata fields
    let e002_errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.code == "E002" && i.severity == Severity::Error)
        .collect();

    assert!(
        !e002_errors.is_empty(),
        "Missing metadata speck should have E002 errors"
    );
    assert!(!result.valid, "Speck with missing metadata should be invalid");
}

#[test]
fn test_invalid_circular_deps_fixture() {
    let content = fs::read_to_string(format!("{}/invalid/circular-deps.md", FIXTURES_DIR))
        .expect("Failed to read circular-deps.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse speck");
    let result = validate_speck(&speck);

    // Should have E011 error for circular dependency
    let e011_errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.code == "E011" && i.severity == Severity::Error)
        .collect();

    assert!(
        !e011_errors.is_empty(),
        "Circular deps speck should have E011 error"
    );
    assert!(!result.valid, "Speck with circular deps should be invalid");
}

#[test]
fn test_invalid_duplicate_anchors_fixture() {
    let content = fs::read_to_string(format!("{}/invalid/invalid-anchors.md", FIXTURES_DIR))
        .expect("Failed to read invalid-anchors.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse speck");
    let result = validate_speck(&speck);

    // Should have E006 error for duplicate anchor
    let e006_errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.code == "E006" && i.severity == Severity::Error)
        .collect();

    assert!(
        !e006_errors.is_empty(),
        "Duplicate anchor speck should have E006 error"
    );
    assert!(!result.valid, "Speck with duplicate anchors should be invalid");
}

#[test]
fn test_parser_handles_all_fixtures() {
    // Test that the parser doesn't panic on any fixture files
    let valid_dir = format!("{}/valid", FIXTURES_DIR);
    let invalid_dir = format!("{}/invalid", FIXTURES_DIR);

    for dir in [valid_dir, invalid_dir] {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "md") {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("Failed to read {:?}", path));
                    let result = parse_speck(&content);
                    assert!(
                        result.is_ok(),
                        "Parser should not panic on {:?}: {:?}",
                        path,
                        result.err()
                    );
                }
            }
        }
    }
}

#[test]
fn test_valid_complete_fixture() {
    let content = fs::read_to_string(format!("{}/valid/complete.md", FIXTURES_DIR))
        .expect("Failed to read complete.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse complete speck");
    let result = validate_speck(&speck);

    // Check no errors
    let errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Valid complete speck should have no errors, got: {:?}",
        errors
    );

    // Verify structure was parsed correctly
    assert!(speck.phase_title.is_some(), "Should have phase title");
    assert!(speck.metadata.owner.is_some(), "Should have owner");
    assert!(!speck.decisions.is_empty(), "Should have decisions");
    assert!(!speck.steps.is_empty(), "Should have steps");
}

#[test]
fn test_valid_with_substeps_fixture() {
    let content = fs::read_to_string(format!("{}/valid/with-substeps.md", FIXTURES_DIR))
        .expect("Failed to read with-substeps.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse with-substeps speck");
    let result = validate_speck(&speck);

    // Check no errors
    let errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Valid with-substeps speck should have no errors, got: {:?}",
        errors
    );

    // Verify substeps were parsed
    let step_with_substeps = speck.steps.iter().find(|s| !s.substeps.is_empty());
    assert!(
        step_with_substeps.is_some(),
        "Should have at least one step with substeps"
    );

    let step = step_with_substeps.unwrap();
    assert!(
        step.substeps.len() >= 2,
        "Step with substeps should have multiple substeps"
    );
}

#[test]
fn test_valid_agent_output_example_fixture() {
    let content = fs::read_to_string(format!("{}/valid/agent-output-example.md", FIXTURES_DIR))
        .expect("Failed to read agent-output-example.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse agent-output-example speck");
    let result = validate_speck(&speck);

    // Check no errors (warnings are OK for bead IDs when beads not enabled)
    let errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Valid agent-output-example speck should have no errors, got: {:?}",
        errors
    );

    // Verify bead IDs were parsed
    assert!(
        speck.metadata.beads_root_id.is_some(),
        "Should have beads root ID"
    );

    let step_with_bead = speck.steps.iter().find(|s| s.bead_id.is_some());
    assert!(
        step_with_bead.is_some(),
        "Should have at least one step with bead ID"
    );

    // Verify some checkboxes are checked (showing progress)
    let checked_count: usize = speck
        .steps
        .iter()
        .flat_map(|s| s.tasks.iter().chain(s.tests.iter()).chain(s.checkpoints.iter()))
        .filter(|c| c.checked)
        .count();
    assert!(
        checked_count > 0,
        "Should have some checked items showing progress"
    );
}

#[test]
fn test_invalid_duplicate_anchors_dedicated_fixture() {
    let content = fs::read_to_string(format!("{}/invalid/duplicate-anchors.md", FIXTURES_DIR))
        .expect("Failed to read duplicate-anchors.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse speck");
    let result = validate_speck(&speck);

    // Should have E006 error for duplicate anchor
    let e006_errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.code == "E006" && i.severity == Severity::Error)
        .collect();

    assert!(
        !e006_errors.is_empty(),
        "Duplicate anchor speck should have E006 error"
    );
    assert!(
        !result.valid,
        "Speck with duplicate anchors should be invalid"
    );
}

#[test]
fn test_invalid_missing_references_fixture() {
    let content = fs::read_to_string(format!("{}/invalid/missing-references.md", FIXTURES_DIR))
        .expect("Failed to read missing-references.md fixture");

    let speck = parse_speck(&content).expect("Failed to parse speck");
    let result = validate_speck(&speck);

    // Should have E010 error for broken references
    let e010_errors: Vec<_> = result
        .issues
        .iter()
        .filter(|i| i.code == "E010" && i.severity == Severity::Error)
        .collect();

    assert!(
        !e010_errors.is_empty(),
        "Missing references speck should have E010 errors, got issues: {:?}",
        result.issues
    );
    assert!(
        !result.valid,
        "Speck with missing references should be invalid"
    );
}

// Golden tests - compare validation output against expected JSON
mod golden_tests {
    use super::*;
    use serde_json::Value;

    fn load_golden(name: &str) -> Value {
        let path = format!("{}/{}.validated.json", GOLDEN_DIR, name);
        let content = fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!("Failed to read golden file: {}", path)
        });
        serde_json::from_str(&content).expect("Failed to parse golden JSON")
    }

    #[test]
    fn test_golden_minimal_valid() {
        let content = fs::read_to_string(format!("{}/valid/minimal.md", FIXTURES_DIR))
            .expect("Failed to read minimal.md fixture");

        let speck = parse_speck(&content).expect("Failed to parse speck");
        let result = validate_speck(&speck);

        let golden = load_golden("minimal");

        assert_eq!(
            golden["status"].as_str().unwrap(),
            "ok",
            "Golden expects ok status"
        );
        assert!(result.valid, "Validation should pass");
        assert_eq!(
            result.issues.iter().filter(|i| i.severity == Severity::Error).count(),
            0,
            "Should have no errors"
        );
    }

    #[test]
    fn test_golden_complete_valid() {
        let content = fs::read_to_string(format!("{}/valid/complete.md", FIXTURES_DIR))
            .expect("Failed to read complete.md fixture");

        let speck = parse_speck(&content).expect("Failed to parse speck");
        let result = validate_speck(&speck);

        let golden = load_golden("complete");

        assert_eq!(
            golden["status"].as_str().unwrap(),
            "ok",
            "Golden expects ok status"
        );
        assert!(result.valid, "Validation should pass");
    }

    #[test]
    fn test_golden_missing_metadata_invalid() {
        let content = fs::read_to_string(format!("{}/invalid/missing-metadata.md", FIXTURES_DIR))
            .expect("Failed to read missing-metadata.md fixture");

        let speck = parse_speck(&content).expect("Failed to parse speck");
        let result = validate_speck(&speck);

        let golden = load_golden("missing-metadata");

        assert_eq!(
            golden["status"].as_str().unwrap(),
            "error",
            "Golden expects error status"
        );
        assert!(!result.valid, "Validation should fail");

        // Check error count matches golden
        let golden_error_count = golden["data"]["files"][0]["error_count"]
            .as_u64()
            .unwrap_or(0) as usize;
        let actual_error_count = result
            .issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count();
        assert_eq!(
            actual_error_count, golden_error_count,
            "Error count should match golden"
        );
    }

    #[test]
    fn test_golden_duplicate_anchors_invalid() {
        let content = fs::read_to_string(format!("{}/invalid/duplicate-anchors.md", FIXTURES_DIR))
            .expect("Failed to read duplicate-anchors.md fixture");

        let speck = parse_speck(&content).expect("Failed to parse speck");
        let result = validate_speck(&speck);

        let golden = load_golden("duplicate-anchors");

        assert_eq!(
            golden["status"].as_str().unwrap(),
            "error",
            "Golden expects error status"
        );
        assert!(!result.valid, "Validation should fail");

        // Verify E006 error code matches
        let golden_codes: Vec<&str> = golden["issues"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|i| i["code"].as_str()).collect())
            .unwrap_or_default();
        assert!(
            golden_codes.contains(&"E006"),
            "Golden should contain E006 error code"
        );

        let has_e006 = result.issues.iter().any(|i| i.code == "E006");
        assert!(has_e006, "Result should have E006 error");
    }
}

// Full workflow integration test
#[test]
fn test_full_validation_workflow() {
    // Test the complete workflow: parse -> validate -> check results for all fixtures
    let valid_fixtures = ["minimal", "complete", "with-substeps", "agent-output-example"];
    let invalid_fixtures = [
        "missing-metadata",
        "circular-deps",
        "invalid-anchors",
        "duplicate-anchors",
        "missing-references",
        "bad-anchors",
    ];

    // All valid fixtures should pass validation (no errors)
    for name in valid_fixtures {
        let path = format!("{}/valid/{}.md", FIXTURES_DIR, name);
        if let Ok(content) = fs::read_to_string(&path) {
            let speck = parse_speck(&content).expect(&format!("Failed to parse {}", name));
            let result = validate_speck(&speck);

            let errors: Vec<_> = result
                .issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .collect();

            assert!(
                errors.is_empty(),
                "Valid fixture {} should have no errors, got: {:?}",
                name,
                errors
            );
        }
    }

    // All invalid fixtures should fail validation (have errors)
    for name in invalid_fixtures {
        let path = format!("{}/invalid/{}.md", FIXTURES_DIR, name);
        if let Ok(content) = fs::read_to_string(&path) {
            let speck = parse_speck(&content).expect(&format!("Failed to parse {}", name));
            let result = validate_speck(&speck);

            assert!(
                !result.valid || result.issues.iter().any(|i| i.severity == Severity::Error),
                "Invalid fixture {} should have errors or be marked invalid",
                name
            );
        }
    }
}
