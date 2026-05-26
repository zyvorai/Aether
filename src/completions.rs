// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Shell completion generation and help display

use clap::Command;
use clap_complete::{generate, Shell};
use std::io;

/// Generate shell completions
pub fn generate_completions(shell: Shell, cmd: &mut Command) {
    let bin_name = "aether";
    generate(shell, cmd, bin_name, &mut io::stdout());
}

/// Print available shells
pub fn list_shells() {
    use crate::output;
    output::section_with_icon("🐚", "Available Shells");
    output::bullet_list(&["bash", "zsh", "fish", "powershell", "elvish"]);
}

/// Display the enhanced help command reference (HyperSDK-style tabular help)
pub fn show_help() {
    use crate::output;

    output::logo();
    output::muted("  Universal Runtime Control Plane - One spec, four runtimes");
    output::muted(&format!("  Version {}\n", env!("CARGO_PKG_VERSION")));

    // Core Operations
    println!(
        "{}",
        output::table(
            &["📋 Core Operations", "Description", "Example"],
            vec![
                vec!["validate".into(), "Validate workload spec".into(), "aether validate".into()],
                vec!["build".into(), "Build workload image".into(), "aether build".into()],
                vec!["run".into(), "Deploy workload".into(), "aether run".into()],
                vec!["stop <name>".into(), "Stop running instance".into(), "aether stop my-app".into()],
                vec!["status <name>".into(), "Get instance status".into(), "aether status my-app".into()],
                vec!["logs <name>".into(), "View instance logs".into(), "aether logs my-app -f".into()],
                vec!["delete <name>".into(), "Delete instance".into(), "aether delete my-app".into()],
                vec!["list".into(), "List all instances".into(), "aether list".into()],
                vec!["deploy <dir>".into(), "Deploy all specs in dir".into(), "aether deploy ./specs".into()],
            ],
        )
    );

    // Migration & Rollback
    println!(
        "{}",
        output::table(
            &["🔄 Migration & Rollback", "Description", "Example"],
            vec![
                vec!["migrate <name>".into(), "Migrate to new runtime".into(), "aether migrate app -t kube".into()],
                vec!["rollback <name>".into(), "Rollback to snapshot".into(), "aether rollback my-app".into()],
                vec!["diff <name>".into(), "Compare spec vs live".into(), "aether diff my-app".into()],
                vec!["drift <name>".into(), "Detect config drift".into(), "aether drift my-app".into()],
            ],
        )
    );

    // AI Intelligence
    println!(
        "{}",
        output::table(
            &["🤖 AI Intelligence", "Description", "Example"],
            vec![
                vec!["recommend".into(), "AI runtime recommendation".into(), "aether recommend".into()],
                vec!["profile".into(), "Workload profiling".into(), "aether profile".into()],
                vec!["analyze-logs <name>".into(), "Log anomaly detection".into(), "aether analyze-logs app".into()],
                vec!["migration-advice".into(), "Migration risk analysis".into(), "aether migration-advice app kube".into()],
                vec!["scaling-advice".into(), "Predictive scaling".into(), "aether scaling-advice".into()],
                vec!["affinity".into(), "Runtime affinity learning".into(), "aether affinity recommend web-service".into()],
            ],
        )
    );

    // Operations & Monitoring
    println!(
        "{}",
        output::table(
            &["📊 Operations & Monitoring", "Description", "Example"],
            vec![
                vec!["tui".into(), "Interactive dashboard".into(), "aether tui".into()],
                vec!["metrics".into(), "Prometheus metrics".into(), "aether metrics".into()],
                vec!["audit".into(), "Audit trail".into(), "aether audit --summary".into()],
                vec!["events".into(), "Event log".into(), "aether events --last 10".into()],
                vec!["orchestrate".into(), "Health monitoring".into(), "aether orchestrate watch".into()],
                vec!["schedule".into(), "Workload scheduling".into(), "aether schedule place app".into()],
                vec!["sla".into(), "SLA compliance".into(), "aether sla check app --uptime 99.9".into()],
            ],
        )
    );

    // Configuration & Templates
    println!(
        "{}",
        output::table(
            &["⚙️  Configuration", "Description", "Example"],
            vec![
                vec!["config".into(), "Show/init configuration".into(), "aether config --init".into()],
                vec!["template".into(), "Generate from template".into(), "aether template web-app".into()],
                vec!["policy-check".into(), "Policy compliance".into(), "aether policy-check -p production".into()],
                vec!["deps".into(), "Dependency management".into(), "aether deps show".into()],
                vec!["secrets".into(), "Secret management".into(), "aether secrets list".into()],
                vec!["env".into(), "Environment management".into(), "aether env list".into()],
            ],
        )
    );

    // Infrastructure
    println!(
        "{}",
        output::table(
            &["🔧 Infrastructure", "Description", "Example"],
            vec![
                vec!["serve".into(), "Start API server".into(), "aether serve --port 8080".into()],
                vec!["backup".into(), "Backup state".into(), "aether backup -n pre-deploy".into()],
                vec!["restore".into(), "Restore from backup".into(), "aether restore backup.tar.gz".into()],
                vec!["cost".into(), "Cost estimation".into(), "aether cost -p all".into()],
                vec!["webhook".into(), "Webhook notifications".into(), "aether webhook list".into()],
                vec!["completions".into(), "Shell completions".into(), "aether completions bash".into()],
                vec!["list-backups".into(), "List available backups".into(), "aether list-backups".into()],
                vec!["help-all".into(), "This command reference".into(), "aether help-all".into()],
            ],
        )
    );

    // New commands
    println!(
        "{}",
        output::table(
            &["🚀 Developer Workflow", "Description", "Example"],
            vec![
                vec!["exec <name>".into(), "Shell into workload".into(), "aether exec my-app".into()],
                vec!["port-forward <name>".into(), "Forward local ports".into(), "aether port-forward my-app 8080:80".into()],
                vec!["watch".into(), "Auto-redeploy on changes".into(), "aether watch".into()],
                vec!["compare".into(), "Cross-runtime comparison".into(), "aether compare".into()],
                vec!["init".into(), "First-time setup wizard".into(), "aether init".into()],
                vec!["compose up".into(), "Deploy from compose file".into(), "aether compose up".into()],
                vec!["compose down".into(), "Stop compose workloads".into(), "aether compose down".into()],
                vec!["plugin list".into(), "List runtime plugins".into(), "aether plugin list".into()],
                vec!["plugin discover".into(), "Scan for plugins".into(), "aether plugin discover".into()],
                vec!["health <name>".into(), "Health timeline & uptime".into(), "aether health my-app".into()],
            ],
        )
    );

    // Output modes
    output::section("Output Modes");
    output::bullet_list(&[
        "--quiet      Suppress all output except errors (machine-friendly)",
        "--json       Output results as JSON (machine-readable)",
        "--output     Format: table (default), json, yaml, wide",
        "--verbose    Enable debug logging",
        "--dry-run    Show what would happen without executing",
        "--yes        Skip confirmation prompts (for CI)",
    ]);

    // Runtime icons
    println!();
    output::section("Supported Runtimes");
    use crate::runtime::RuntimeKind;
    println!("  {} {} {} {}",
        output::runtime_display(&RuntimeKind::Podman),
        output::runtime_display(&RuntimeKind::Kubernetes),
        output::runtime_display(&RuntimeKind::KubeVirt),
        output::runtime_display(&RuntimeKind::Metal3),
    );
    println!();
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
