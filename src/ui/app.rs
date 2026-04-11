//! Application state for TUI

use crate::{
    runtime::{self, RuntimeKind, Status},
    state::{StateStore, WorkloadState},
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Screen {
    #[default]
    Dashboard,
    Logs(String), // workload name
}

#[derive(Debug, Clone)]
pub struct WorkloadInfo {
    pub state: WorkloadState,
    pub status: Option<Status>,
    pub last_updated: Instant,
}

pub struct App {
    pub state: AppState,
    pub workloads: Vec<WorkloadInfo>,
    pub selected_index: usize,
    pub screen: Screen,
    pub logs_buffer: Vec<String>,
    pub should_quit: bool,
    pub last_refresh: Instant,
    pub status_message: Option<String>,
    /// Search/filter string for workload list
    pub search_filter: String,
    /// Whether the search input is active
    pub search_active: bool,
}

pub struct AppState {
    pub state_store: StateStore,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let state_store = StateStore::load(&StateStore::default_path())?;

        Ok(Self {
            state: AppState { state_store },
            workloads: Vec::new(),
            selected_index: 0,
            screen: Screen::Dashboard,
            logs_buffer: Vec::new(),
            should_quit: false,
            last_refresh: Instant::now(),
            status_message: None,
            search_filter: String::new(),
            search_active: false,
        })
    }

    pub async fn refresh_workloads(&mut self) -> anyhow::Result<()> {
        let workload_states: Vec<WorkloadState> = self
            .state
            .state_store
            .list()
            .into_iter()
            .cloned()
            .collect();

        // Group workloads by runtime to reuse clients
        let mut by_runtime: std::collections::HashMap<RuntimeKind, Vec<WorkloadState>> =
            std::collections::HashMap::new();
        for state in workload_states {
            by_runtime.entry(state.runtime).or_default().push(state);
        }

        let mut updated_workloads = Vec::new();
        for (kind, states) in by_runtime {
            let rt = runtime::create_runtime(&kind).await.ok();
            for state in states {
                let status = match &rt {
                    Some(r) => r.status(&state.instance).await.ok(),
                    None => None,
                };
                updated_workloads.push(WorkloadInfo {
                    state,
                    status,
                    last_updated: Instant::now(),
                });
            }
        }

        self.workloads = updated_workloads;
        self.last_refresh = Instant::now();

        Ok(())
    }

    pub async fn load_logs(&mut self, workload_name: &str) -> anyhow::Result<()> {
        if let Some(workload) = self.workloads.iter().find(|w| w.state.name == workload_name) {
            let rt = runtime::create_runtime(&workload.state.runtime).await?;
            let logs = rt.logs(&workload.state.instance, false).await?;
            self.logs_buffer = logs.lines().map(|s| s.to_string()).collect();
        }

        Ok(())
    }

    pub fn select_next(&mut self) {
        let len = self.filtered_workloads().len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    pub fn select_previous(&mut self) {
        let len = self.filtered_workloads().len();
        if len > 0 {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = len - 1;
            }
        }
    }

    pub fn selected_workload(&self) -> Option<&WorkloadInfo> {
        let filtered = self.filtered_workloads();
        filtered.get(self.selected_index).copied()
    }

    /// Return workloads matching the current search filter
    pub fn filtered_workloads(&self) -> Vec<&WorkloadInfo> {
        if self.search_filter.is_empty() {
            self.workloads.iter().collect()
        } else {
            let query = self.search_filter.to_lowercase();
            self.workloads
                .iter()
                .filter(|w| {
                    w.state.name.to_lowercase().contains(&query)
                        || w.state.runtime.to_string().to_lowercase().contains(&query)
                })
                .collect()
        }
    }

    /// Toggle search mode on/off
    pub fn toggle_search(&mut self) {
        self.search_active = !self.search_active;
        if !self.search_active {
            self.search_filter.clear();
        }
    }

    /// Append a character to search filter
    pub fn search_push(&mut self, ch: char) {
        self.search_filter.push(ch);
        self.selected_index = 0;
    }

    /// Remove last character from search filter
    pub fn search_pop(&mut self) {
        self.search_filter.pop();
        self.selected_index = 0;
    }

    pub fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() > Duration::from_secs(5)
    }

    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }
}

impl Default for App {
    fn default() -> Self {
        match Self::new() {
            Ok(app) => app,
            Err(_) => Self {
                state: AppState {
                    state_store: StateStore::default(),
                },
                workloads: Vec::new(),
                selected_index: 0,
                screen: Screen::Dashboard,
                logs_buffer: Vec::new(),
                should_quit: false,
                last_refresh: Instant::now(),
                status_message: None,
                search_filter: String::new(),
                search_active: false,
            },
        }
    }
}
