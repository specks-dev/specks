//! Integration tests for specks-core
//!
//! These tests validate the parser and validator against fixture files.

use specks_core::{parse_speck, validate_speck, Severity};
use std::fs;

const FIXTURES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/fixtures");

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
