// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! CLI handlers for the operator features in [`aether::ops`].

use aether::ops::{self, holds, revisions};
use aether::output;
use aether::state::{StateStore, WorkloadState};
use anyhow::Result;
use serde::Serialize;

use crate::cli::HoldAction;

fn managed(name: &str) -> Result<WorkloadState> {
    ops::check_name(name)?;
    StateStore::load(&StateStore::default_path())?
        .get(name)
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Workload '{name}' not found.\nHint: Run `aether list` to see deployed workloads."
            )
        })
}

/// Print `value` as JSON or YAML when asked for, else the text rendering.
fn emit<T: Serialize>(value: &T, text: impl FnOnce() -> String) -> Result<()> {
    if output::is_json() {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else if output::is_yaml() {
        print!("{}", serde_yaml::to_string(value)?);
    } else {
        print!("{}", text());
    }
    Ok(())
}

pub(crate) fn history_command(name: &str, show: Option<u64>, diff: &[u64]) -> Result<()> {
    // History outlives the workload, so it need not be in the state file.
    let log = revisions::load(&ops::data_dir(), name)?;
    if let [from, to] = *diff {
        let plan = log.diff(from, to)?;
        return emit(&plan, || plan.render_text());
    }
    if let Some(id) = show {
        let view = log.view(id)?;
        return emit(&view, || view.spec_yaml.clone());
    }
    let summaries = log.summaries();
    emit(&summaries, || revisions::format_history(name, &summaries))
}

pub(crate) fn hold_command(action: HoldAction) -> Result<()> {
    let dir = ops::data_dir();
    match action {
        HoldAction::Set {
            name,
            reason,
            duration,
        } => {
            managed(&name)?;
            let secs = duration
                .as_deref()
                .map(ops::parse_duration_secs)
                .transpose()?;
            let hold = holds::set(&dir, &name, &reason, secs)?;
            emit(&hold, || {
                format!(
                    "'{}' is on hold until {}. Aether will refuse changes to it.\n",
                    hold.workload,
                    hold.expires_at.as_deref().unwrap_or("released")
                )
            })
        }
        HoldAction::Release { name } => match holds::release(&dir, &name)? {
            Some(hold) => emit(&hold, || format!("Released the hold on '{name}'\n")),
            None => anyhow::bail!("'{name}' is not on hold"),
        },
        HoldAction::List => {
            let list = holds::active(&dir)?;
            emit(&list, || {
                if list.is_empty() {
                    return "No workload is on hold.\n".to_string();
                }
                list.iter()
                    .map(|h| {
                        format!(
                            "{:<24} until {:<32} by {:<16} {}\n",
                            h.workload,
                            h.expires_at.as_deref().unwrap_or("released"),
                            h.actor,
                            h.reason
                        )
                    })
                    .collect()
            })
        }
    }
}
