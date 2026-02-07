//! Shell completion generation

use clap::Command;
use clap_complete::{generate, Shell};
use std::io;

/// Generate shell completions
pub fn generate_completions(shell: Shell, cmd: &mut Command) {
    let bin_name = "orchestr8";
    generate(shell, cmd, bin_name, &mut io::stdout());
}

/// Print available shells
pub fn list_shells() {
    println!("Available shells:");
    println!("  bash");
    println!("  zsh");
    println!("  fish");
    println!("  powershell");
    println!("  elvish");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_shells() {
        // Verify list_shells runs without panicking
        list_shells();
    }

    #[test]
    fn test_generate_completions_bash() {
        use clap::{Command, Arg};
        let mut cmd = Command::new("test-app")
            .arg(Arg::new("verbose").short('v'));
        generate_completions(Shell::Bash, &mut cmd);
    }
}
