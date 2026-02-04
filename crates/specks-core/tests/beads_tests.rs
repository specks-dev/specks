//! Integration tests for beads functionality

use specks_core::beads::is_valid_bead_id;

#[test]
fn test_bead_id_validation() {
    // Valid IDs
    assert!(is_valid_bead_id("bd-abc123"));
    assert!(is_valid_bead_id("bd-fake-1"));
    assert!(is_valid_bead_id("bd-fake-1.1"));
    assert!(is_valid_bead_id("bd-fake-1.2.3"));
    assert!(is_valid_bead_id("gt-abc1"));
    assert!(is_valid_bead_id("prefix-xyz99"));

    // Invalid IDs
    assert!(!is_valid_bead_id(""));
    assert!(!is_valid_bead_id("bd"));
    assert!(!is_valid_bead_id("bd-"));
    assert!(!is_valid_bead_id("-abc123"));
    assert!(!is_valid_bead_id("BD-ABC123")); // Must be lowercase
    assert!(!is_valid_bead_id("bd_abc123")); // Underscores not allowed in format
}

#[test]
fn test_issue_json_parsing() {
    use specks_core::beads::Issue;

    let json = r#"{"id":"bd-fake-1","title":"Test Issue","description":"","status":"open","priority":2,"issue_type":"task"}"#;
    let issue: Issue = serde_json::from_str(json).expect("Failed to parse Issue JSON");

    assert_eq!(issue.id, "bd-fake-1");
    assert_eq!(issue.title, "Test Issue");
    assert_eq!(issue.status, "open");
    assert_eq!(issue.priority, 2);
    assert_eq!(issue.issue_type, "task");
}

#[test]
fn test_issue_details_json_parsing_array() {
    use specks_core::beads::IssueDetails;

    // bd show returns array
    let json = r#"[{"id":"bd-fake-1","title":"Test Issue","description":"","status":"open","priority":2,"issue_type":"task","dependencies":[]}]"#;
    let issues: Vec<IssueDetails> = serde_json::from_str(json).expect("Failed to parse IssueDetails array");

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, "bd-fake-1");
}

#[test]
fn test_issue_details_json_parsing_object() {
    use specks_core::beads::IssueDetails;

    // bd show may return single object
    let json = r#"{"id":"bd-fake-1","title":"Test Issue","description":"","status":"closed","priority":2,"issue_type":"task","dependencies":[{"id":"bd-fake-0","dependency_type":"blocks"}]}"#;
    let issue: IssueDetails = serde_json::from_str(json).expect("Failed to parse IssueDetails object");

    assert_eq!(issue.id, "bd-fake-1");
    assert_eq!(issue.status, "closed");
    assert_eq!(issue.dependencies.len(), 1);
    assert_eq!(issue.dependencies[0].id, "bd-fake-0");
}

#[test]
fn test_bead_status_display() {
    use specks_core::beads::BeadStatus;

    assert_eq!(format!("{}", BeadStatus::Complete), "complete");
    assert_eq!(format!("{}", BeadStatus::Ready), "ready");
    assert_eq!(format!("{}", BeadStatus::Blocked), "blocked");
    assert_eq!(format!("{}", BeadStatus::Pending), "pending");
}

#[test]
fn test_dep_result_json_parsing() {
    use specks_core::beads::DepResult;

    let json = r#"{"status":"added","issue_id":"bd-fake-1","depends_on_id":"bd-fake-0","type":"blocks"}"#;
    let result: DepResult = serde_json::from_str(json).expect("Failed to parse DepResult");

    assert_eq!(result.status, "added");
    assert_eq!(result.issue_id, "bd-fake-1");
    assert_eq!(result.depends_on_id, "bd-fake-0");
}
