// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! CLI handlers for the operator features in [`aether::ops`].

use aether::ops::{self, attention, changes, explain, holds, incidents, revisions};
use aether::output;
use aether::state::{StateStore, WorkloadState};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::Path;

use crate::cli::{HoldAction, IncidentAction};

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

pub(crate) fn explain_command(name: &str) -> Result<()> {
    let ws = managed(name)?;
    let e = explain::explain(&ops::data_dir(), &ws)?;
    emit(&e, || explain::format(&e))
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

pub(crate) fn changes_command(since: &str, workload: Option<String>, limit: usize) -> Result<()> {
    let since = ops::parse_since(since, chrono::Utc::now())?;
    let feed = changes::collect(
        &ops::data_dir(),
        &changes::Query::since(since, workload, limit.max(1)),
    )?;
    emit(&feed, || changes::format_feed(&feed))
}

pub(crate) fn attention_command() -> Result<()> {
    let state = StateStore::load(&StateStore::default_path())?;
    let queue = attention::build(&ops::data_dir(), &state)?;
    emit(&queue, || attention::format_queue(&queue))
}

pub(crate) fn incidents_command(action: Option<IncidentAction>) -> Result<()> {
    let dir = ops::data_dir();
    match action.unwrap_or(IncidentAction::List {
        workload: None,
        open: false,
    }) {
        IncidentAction::List { workload, open } => {
            let store = incidents::load(&dir)?;
            let list: Vec<_> = store
                .incidents
                .iter()
                .rev()
                .filter(|i| workload.as_deref().is_none_or(|w| w == i.workload))
                .filter(|i| !open || i.is_open())
                .collect();
            emit(&list, || incidents::format_list(&list))
        }
        IncidentAction::Show { id } => {
            let detail = incidents::detail(&dir, id)?;
            emit(&detail, || incidents::markdown(&detail))
        }
        IncidentAction::Report { id, out } => {
            let detail = incidents::detail(&dir, id)?;
            write_or_print(&incidents::markdown(&detail), out.as_deref())
        }
        IncidentAction::Ack { id } => {
            let i = incidents::acknowledge(&dir, id)?;
            emit(&i, || {
                let by = i.acknowledged.as_ref().map_or("-", |a| a.actor.as_str());
                format!("Incident {id} acknowledged by {by}\n")
            })
        }
        IncidentAction::Note { id, text } => {
            let i = incidents::add_note(&dir, id, &text.join(" "))?;
            emit(&i, || format!("Note added to incident {id}\n"))
        }
    }
}

fn write_or_print(content: &str, out: Option<&Path>) -> Result<()> {
    match out {
        Some(path) => {
            std::fs::write(path, content)
                .with_context(|| format!("failed to write {}", path.display()))?;
            output::success(&format!("Wrote {}", path.display()));
        }
        None => print!("{content}"),
    }
    Ok(())
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
