//! Splash screen display for specks CLI
//!
//! Shows a compact startup banner with ASCII spectacles logo and version info.

use owo_colors::OwoColorize;
use std::io::{IsTerminal, Write};

/// ASCII art spectacles logo
const SPECTACLES: &[&str] = &[
    "  ○━━○ ○━━○",
    "    ╲───╱  ",
];

/// Display the splash screen
pub fn show_splash() {
    if !std::io::stdout().is_terminal() {
        // Non-TTY: just show version
        println!("specks v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    let mut stdout = std::io::stdout();
    let version = env!("CARGO_PKG_VERSION");

    // Print spectacles with info on the right
    writeln!(
        stdout,
        "{}   {} v{}",
        SPECTACLES[0].cyan(),
        "specks".bold().cyan(),
        version.dimmed()
    )
    .ok();
    writeln!(
        stdout,
        "{}   {}",
        SPECTACLES[1].cyan(),
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
