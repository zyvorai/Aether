// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! CLI handlers for the operator features in [`aether::ops`].

use aether::ops::{self, revisions};
use aether::output;
use anyhow::Result;
use serde::Serialize;

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
