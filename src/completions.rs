//! Shell completion generation and help display

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
                vec!["validate".into(), "Validate workload spec".into(), "orchestr8 validate".into()],
                vec!["build".into(), "Build workload image".into(), "orchestr8 build".into()],
                vec!["run".into(), "Deploy workload".into(), "orchestr8 run".into()],
                vec!["stop <name>".into(), "Stop running instance".into(), "orchestr8 stop my-app".into()],
                vec!["status <name>".into(), "Get instance status".into(), "orchestr8 status my-app".into()],
                vec!["logs <name>".into(), "View instance logs".into(), "orchestr8 logs my-app -f".into()],
                vec!["delete <name>".into(), "Delete instance".into(), "orchestr8 delete my-app".into()],
                vec!["list".into(), "List all instances".into(), "orchestr8 list".into()],
                vec!["deploy <dir>".into(), "Deploy all specs in dir".into(), "orchestr8 deploy ./specs".into()],
            ],
        )
    );

    // Migration & Rollback
    println!(
        "{}",
        output::table(
            &["🔄 Migration & Rollback", "Description", "Example"],
            vec![
                vec!["migrate <name>".into(), "Migrate to new runtime".into(), "orchestr8 migrate app -t kube".into()],
                vec!["rollback <name>".into(), "Rollback to snapshot".into(), "orchestr8 rollback my-app".into()],
                vec!["diff <name>".into(), "Compare spec vs live".into(), "orchestr8 diff my-app".into()],
                vec!["drift <name>".into(), "Detect config drift".into(), "orchestr8 drift my-app".into()],
            ],
        )
    );

    // AI Intelligence
    println!(
        "{}",
        output::table(
            &["🤖 AI Intelligence", "Description", "Example"],
            vec![
                vec!["recommend".into(), "AI runtime recommendation".into(), "orchestr8 recommend".into()],
                vec!["profile".into(), "Workload profiling".into(), "orchestr8 profile".into()],
                vec!["analyze-logs <name>".into(), "Log anomaly detection".into(), "orchestr8 analyze-logs app".into()],
                vec!["migration-advice".into(), "Migration risk analysis".into(), "orchestr8 migration-advice app kube".into()],
                vec!["scaling-advice".into(), "Predictive scaling".into(), "orchestr8 scaling-advice".into()],
                vec!["affinity".into(), "Runtime affinity learning".into(), "orchestr8 affinity recommend web-service".into()],
            ],
        )
    );

    // Operations & Monitoring
    println!(
        "{}",
        output::table(
            &["📊 Operations & Monitoring", "Description", "Example"],
            vec![
                vec!["tui".into(), "Interactive dashboard".into(), "orchestr8 tui".into()],
                vec!["metrics".into(), "Prometheus metrics".into(), "orchestr8 metrics".into()],
                vec!["audit".into(), "Audit trail".into(), "orchestr8 audit --summary".into()],
                vec!["events".into(), "Event log".into(), "orchestr8 events --last 10".into()],
                vec!["orchestrate".into(), "Health monitoring".into(), "orchestr8 orchestrate watch".into()],
                vec!["schedule".into(), "Workload scheduling".into(), "orchestr8 schedule place app".into()],
                vec!["sla".into(), "SLA compliance".into(), "orchestr8 sla check app --uptime 99.9".into()],
            ],
        )
    );

    // Configuration & Templates
    println!(
        "{}",
        output::table(
            &["⚙️  Configuration", "Description", "Example"],
            vec![
                vec!["config".into(), "Show/init configuration".into(), "orchestr8 config --init".into()],
                vec!["template".into(), "Generate from template".into(), "orchestr8 template web-app".into()],
                vec!["policy-check".into(), "Policy compliance".into(), "orchestr8 policy-check -p production".into()],
                vec!["deps".into(), "Dependency management".into(), "orchestr8 deps show".into()],
                vec!["secrets".into(), "Secret management".into(), "orchestr8 secrets list".into()],
                vec!["env".into(), "Environment management".into(), "orchestr8 env list".into()],
            ],
        )
    );

    // Infrastructure
    println!(
        "{}",
        output::table(
            &["🔧 Infrastructure", "Description", "Example"],
            vec![
                vec!["serve".into(), "Start API server".into(), "orchestr8 serve --port 8080".into()],
                vec!["backup".into(), "Backup state".into(), "orchestr8 backup -n pre-deploy".into()],
                vec!["restore".into(), "Restore from backup".into(), "orchestr8 restore backup.tar.gz".into()],
                vec!["cost".into(), "Cost estimation".into(), "orchestr8 cost -p all".into()],
                vec!["webhook".into(), "Webhook notifications".into(), "orchestr8 webhook list".into()],
                vec!["completions".into(), "Shell completions".into(), "orchestr8 completions bash".into()],
                vec!["list-backups".into(), "List available backups".into(), "orchestr8 list-backups".into()],
                vec!["help-all".into(), "This command reference".into(), "orchestr8 help-all".into()],
            ],
        )
    );

    // Output modes
    output::section("Output Modes");
    output::bullet_list(&[
        "--quiet    Suppress all output except errors (machine-friendly)",
        "--json     Output results as JSON (machine-readable)",
        "--verbose  Enable debug logging",
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
