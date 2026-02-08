//! Application state for TUI

use crate::{
    adapters::{KubernetesRuntime, PodmanRuntime},
    runtime::{RuntimeKind, Status},
    state::{StateStore, WorkloadState},
    Runtime,
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

        let mut updated_workloads = Vec::new();

        for state in workload_states {
            // Fetch current status
            let status = self.fetch_status(&state).await.ok();

            updated_workloads.push(WorkloadInfo {
                state,
                status,
                last_updated: Instant::now(),
            });
        }

        self.workloads = updated_workloads;
        self.last_refresh = Instant::now();

        Ok(())
    }

    async fn fetch_status(&self, state: &WorkloadState) -> anyhow::Result<Status> {
        match state.runtime {
            RuntimeKind::Podman => {
                let runtime = PodmanRuntime::new()?;
                runtime.status(&state.instance).await
            }
            RuntimeKind::Kubernetes => {
                let runtime = KubernetesRuntime::new().await?;
                runtime.status(&state.instance).await
            }
            _ => anyhow::bail!("Runtime not yet implemented"),
        }
    }

    pub async fn load_logs(&mut self, workload_name: &str) -> anyhow::Result<()> {
        if let Some(workload) = self.workloads.iter().find(|w| w.state.name == workload_name) {
            let logs = match workload.state.runtime {
                RuntimeKind::Podman => {
                    let runtime = PodmanRuntime::new()?;
                    runtime.logs(&workload.state.instance, false).await?
                }
                RuntimeKind::Kubernetes => {
                    let runtime = KubernetesRuntime::new().await?;
                    runtime.logs(&workload.state.instance, false).await?
                }
                _ => anyhow::bail!("Runtime not yet implemented"),
            };

            self.logs_buffer = logs.lines().map(|s| s.to_string()).collect();
        }

        Ok(())
    }

    pub fn select_next(&mut self) {
        if !self.workloads.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.workloads.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.workloads.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.workloads.len().saturating_sub(1);
            }
        }
    }

    pub fn selected_workload(&self) -> Option<&WorkloadInfo> {
        self.workloads.get(self.selected_index)
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
        Self::new().expect("Failed to create App")
    }
}
