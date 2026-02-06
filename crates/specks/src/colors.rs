//! Semantic color theme for consistent terminal output
//!
//! This module provides centralized color constants with semantic meaning:
//! - `ACTIVE` => blue - Spinners, headers, active elements
//! - `SUCCESS` => green - Completed operations, success messages
//! - `WARNING` => yellow - Warnings, suggestions, medium-priority items
//! - `FAIL` => red - Errors, critical issues, high-priority items
//!
//! These map to punch list severity: HIGH=FAIL, MEDIUM=WARNING, LOW=ACTIVE.

use std::sync::LazyLock;

use owo_colors::Style;

/// Semantic color definitions for terminal output
pub struct SemanticColors {
    /// Blue - spinners, headers, active elements
    pub active: Style,
    /// Green - completed operations, success messages
    pub success: Style,
    /// Yellow - warnings, suggestions, medium-priority items
    pub warning: Style,
    /// Red - errors, critical issues, high-priority items
    pub fail: Style,
}

impl Default for SemanticColors {
    fn default() -> Self {
        Self {
            active: Style::new().blue(),
            success: Style::new().green(),
            warning: Style::new().yellow(),
            fail: Style::new().red(),
        }
    }
}

/// Global default theme
pub static COLORS: LazyLock<SemanticColors> = LazyLock::new(SemanticColors::default);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_colors_default() {
        let colors = SemanticColors::default();
        // Verify each style is created (we can't easily test the actual color,
        // but we can verify they're valid styles by using them)
        let _ = colors.active;
        let _ = colors.success;
        let _ = colors.warning;
        let _ = colors.fail;
    }

    #[test]
    fn test_colors_global_is_accessible() {
        // Verify the global COLORS is accessible
        let _ = &COLORS.active;
        let _ = &COLORS.success;
        let _ = &COLORS.warning;
        let _ = &COLORS.fail;
    }
}
