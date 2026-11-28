//! The API and the CLI report what a workload was last observed doing, not
//! merely that it was once deployed.
//!
//! These run the real binary on a temporary home directory.

use aether::ops::incidents::{self, Observation};
use aether::runtime::{Instance, InstanceState, RuntimeKind};
use aether::state::{StateStore, WorkloadState};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const EXPECTED: [(&str, &str); 4] = [
    ("api", "running"),
    ("cache", "stopped"),
    ("web", "stopped unexpectedly"),
    ("worker", "unknown"),
];

/// Four deployed workloads: one running, one stopped on request, one found
/// stopped and one never checked.
fn seed(home: &Path) {
    let dir = home.join(".aether");
    std::fs::create_dir_all(&dir).unwrap();
    let mut state = StateStore::new();
    for (name, _) in EXPECTED {
        let instance = Instance::new(name.into(), name.into(), RuntimeKind::Podman, name.into());
        state.upsert(
            name.into(),
            WorkloadState::new(
                name.into(),
                RuntimeKind::Podman,
                instance,
                dir.join(format!("{name}.yaml")),
            ),
        );
    }
    state.save(&dir.join("state.json")).unwrap();
    let see = |name: &str, state: InstanceState, ready: bool| {
        incidents::record(
            &dir,
            &Observation {
                workload: name.into(),
                runtime: RuntimeKind::Podman,
                state,
                ready,
                restart_count: 0,
                message: None,
                at: aether::resources::now_rfc3339(),
            },
        )
        .unwrap();
    };
    see("api", InstanceState::Running, true);
    see("cache", InstanceState::Running, true);
    incidents::expect_down(&dir, "cache", RuntimeKind::Podman, "stopped").unwrap();
    see("web", InstanceState::Stopped, false);
    see("web", InstanceState::Stopped, false);
}

fn command(home: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_aether"));
    c.env("HOME", home)
        .env("XDG_RUNTIME_DIR", home)
        .env("KUBECONFIG", "/dev/null")
        .env("NO_COLOR", "1")
        .env("AETHER_OBSERVE_SECS", "0")
        .env("AETHER_DEMO_AUTH", "0")
        .env_remove("AETHER_API_KEY")
        .env_remove("AETHER_STATE_DATABASE_URL");
    c
}

fn status_of<'a>(list: &'a [serde_json::Value], name: &str) -> &'a str {
    list.iter()
        .find(|w| w["name"] == name)
        .and_then(|w| w["status"].as_str())
        .unwrap_or("<no status>")
}

#[test]
fn cli_list_reports_observed_status() {
    let home = tempfile::tempdir().unwrap();
    seed(home.path());
    let o = command(home.path())
        .args(["--output", "json", "list"])
        .output()
        .expect("run aether");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let list: Vec<serde_json::Value> = serde_json::from_slice(&o.stdout).expect("valid JSON");
    for (name, status) in EXPECTED {
        assert_eq!(status_of(&list, name), status, "{name}");
    }

    let o = command(home.path())
        .arg("list")
        .output()
        .expect("run aether");
    let table = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(table.contains("stopped unexpectedly"), "{table}");
    assert!(!table.contains("Running Workloads"), "{table}");
}

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn get(port: u16, path: &str) -> Option<serde_json::Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .ok()?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .ok()?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).ok()?;
    let (_, body) = reply.split_once("\r\n\r\n")?;
    // The envelope may arrive chunked; take the JSON document inside it.
    let json = &body[body.find('{')?..=body.rfind('}')?];
    serde_json::from_str(json).ok()
}

#[test]
fn api_reports_observed_status() {
    let home = tempfile::tempdir().unwrap();
    seed(home.path());
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let _server = Server(
        command(home.path())
            .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start aether serve"),
    );
    let started = Instant::now();
    while get(port, "/health").is_none() {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the server did not start"
        );
        std::thread::sleep(Duration::from_millis(200));
    }

    let reply = get(port, "/api/workloads").expect("list workloads");
    let list = reply["data"].as_array().expect("a list").clone();
    for (name, status) in EXPECTED {
        assert_eq!(status_of(&list, name), status, "{name}");
        let one = get(port, &format!("/api/workloads/{name}")).expect("one workload");
        assert_eq!(one["data"]["status"], status, "{name}");
        assert!(
            one["data"]["status_detail"].is_string(),
            "{name} says what the status rests on"
        );
    }
}
