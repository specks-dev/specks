//! Splash screen display for specks CLI
//!
//! Shows a compact startup banner with ASCII spectacles logo and version info.

use owo_colors::OwoColorize;
use std::io::{IsTerminal, Write};

use crate::colors::COLORS;

/// ASCII art spectacles logo
const SPECTACLES: &[&str] = &["  ○━━○ ○━━○", "    ╲───╱  "];

/// Display the splash screen
pub fn show_splash() {
    if !std::io::stdout().is_terminal() {
        // Non-TTY: just show version
        println!("specks v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let mut stdout = std::io::stdout();
    let version = env!("CARGO_PKG_VERSION");

    // Print spectacles with info on the right (using semantic ACTIVE color)
    writeln!(
        stdout,
        "{}   {} v{}",
        COLORS.active.style(SPECTACLES[0]),
        COLORS.active.style("specks").bold(),
        version.dimmed()
    )
    .ok();
    writeln!(
        stdout,
        "{}   {}",
        COLORS.active.style(SPECTACLES[1]),
        "Multi-agent orchestration".dimmed()
    )
    .ok();
    writeln!(stdout).ok();

    stdout.flush().ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectacles_art() {
        // Verify the spectacles art has correct dimensions
        assert_eq!(SPECTACLES.len(), 2);
        // First line is the glasses, second is the bridge
        assert!(SPECTACLES[0].contains("○━━○"));
        assert!(SPECTACLES[1].contains("╲───╱"));
    }
}
