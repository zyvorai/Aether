//! Stop, start, restart and update of Podman and Docker workloads, through the
//! API and the CLI.
//!
//! The real binary runs against a stand-in container engine (see
//! `tests/fixtures/fake-container-cli.sh`) in a temporary home directory.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const SPEC: &str = r#"apiVersion: aether/v1
kind: Workload
metadata:
  name: smoke
  owner: ops
  project: shop
build:
  context: .
  dockerfile: Dockerfile
  registry: localhost
requirements:
  cpu: "500m"
  memory: 256Mi
  storage: 1Gi
runtime:
  preferred: container
  allow: [container]
network:
  ports:
    - containerPort: 8080
      servicePort: 8080
"#;

struct Host {
    dir: tempfile::TempDir,
    engine: &'static str,
}

impl Host {
    /// A home with `smoke` deployed on `engine`.
    fn with_smoke(engine: &'static str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let fake =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-container-cli.sh");
        std::os::unix::fs::symlink(fake, bin.join(engine)).unwrap();
        std::fs::write(dir.path().join("Dockerfile"), "FROM scratch\n").unwrap();
        std::fs::write(dir.path().join("smoke.yaml"), SPEC).unwrap();
        let host = Self { dir, engine };
        host.ok(&[
            "--skip-policy",
            "--spec",
            "smoke.yaml",
            "run",
            "--runtime",
            engine,
        ]);
        host
    }

    fn containers(&self) -> PathBuf {
        self.dir.path().join("containers")
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_aether"));
        c.args(args)
            .current_dir(self.dir.path())
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.dir.path().join("bin").display()),
            )
            .env("FAKE_CONTAINERS", self.containers())
            .env("HOME", self.dir.path())
            .env("XDG_RUNTIME_DIR", self.dir.path())
            .env("KUBECONFIG", "/dev/null")
            .env("NO_COLOR", "1")
            .env("AETHER_OBSERVE_SECS", "0")
            .env("AETHER_DEMO_AUTH", "0")
            .env_remove("AETHER_API_KEY")
            .env_remove("AETHER_STATE_DATABASE_URL");
        c
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("run aether")
    }

    fn ok(&self, args: &[&str]) {
        let o = self.run(args);
        assert!(
            o.status.success(),
            "{} {args:?}: {}{}",
            self.engine,
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
    }

    /// `name=state` of every container, sorted.
    fn engine_state(&self) -> Vec<String> {
        let mut all: Vec<String> = std::fs::read_dir(self.containers().join("c"))
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                let line = std::fs::read_to_string(e.path()).unwrap();
                format!(
                    "{}={}",
                    e.file_name().to_string_lossy(),
                    line.split('|').next().unwrap()
                )
            })
            .collect();
        all.sort();
        all
    }

    fn fail_runs(&self, fail: bool) {
        let flag = self.containers().join("fail_run");
        if fail {
            std::fs::write(flag, "").unwrap();
        } else {
            std::fs::remove_file(flag).unwrap();
        }
    }

    fn serve(&self) -> Server {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let child = self
            .command(&["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("start aether serve");
        let server = Server { child, port };
        let started = Instant::now();
        while server.request("GET", "/health").is_none() {
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "the server did not start"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
        server
    }
}

struct Server {
    child: Child,
    port: u16,
}

impl Server {
    fn request(&self, method: &str, path: &str) -> Option<(u16, String)> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).ok()?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .ok()?;
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
        )
        .ok()?;
        let mut reply = String::new();
        stream.read_to_string(&mut reply).ok()?;
        let status = reply.split_whitespace().nth(1)?.parse().ok()?;
        let (_, body) = reply.split_once("\r\n\r\n")?;
        Some((status, body.to_string()))
    }

    fn post(&self, action: &str) -> (u16, String) {
        self.request("POST", &format!("/api/workloads/smoke/{action}"))
            .expect("an answer")
    }

    fn status_of_smoke(&self) -> String {
        let (_, body) = self.request("GET", "/api/workloads/smoke").unwrap();
        let json = &body[body.find('{').unwrap()..=body.rfind('}').unwrap()];
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        v["data"]["status"].as_str().unwrap_or_default().to_string()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn api_stop_start_restart(engine: &'static str) {
    let host = Host::with_smoke(engine);
    let server = host.serve();

    let (code, body) = server.post("stop");
    assert_eq!(code, 200, "{body}");
    assert_eq!(host.engine_state(), ["smoke=exited"]);
    assert_eq!(server.status_of_smoke(), "stopped");

    let (code, body) = server.post("start");
    assert_eq!(code, 200, "{body}");
    assert_eq!(host.engine_state(), ["smoke=running"]);
    assert_eq!(server.status_of_smoke(), "running");

    let (code, body) = server.post("restart");
    assert_eq!(code, 200, "{body}");
    assert_eq!(host.engine_state(), ["smoke=running"]);
    assert_eq!(server.status_of_smoke(), "running");

    // Starting what already runs is refused and changes nothing.
    let (code, body) = server.post("start");
    assert_eq!(code, 500, "{body}");
    assert!(body.contains("already running"), "{body}");
    assert_eq!(host.engine_state(), ["smoke=running"]);
}

#[test]
fn api_stop_start_restart_on_podman() {
    api_stop_start_restart("podman");
}

#[test]
fn api_stop_start_restart_on_docker() {
    api_stop_start_restart("docker");
}

#[test]
fn a_failed_start_keeps_the_stopped_container() {
    let host = Host::with_smoke("podman");
    let server = host.serve();
    assert_eq!(server.post("stop").0, 200);

    host.fail_runs(true);
    let (code, body) = server.post("start");
    assert_eq!(code, 500, "{body}");
    assert!(body.contains("fake run failure"), "{body}");
    assert_eq!(host.engine_state(), ["smoke=exited"]);
    // The start was asked for, so being down is no longer intended.
    assert_ne!(server.status_of_smoke(), "stopped");

    host.fail_runs(false);
    let (code, body) = server.post("start");
    assert_eq!(code, 200, "{body}");
    assert_eq!(host.engine_state(), ["smoke=running"]);
}

fn cli_restart_update_and_run_again(engine: &'static str) {
    let host = Host::with_smoke(engine);
    host.ok(&["restart", "smoke"]);
    assert_eq!(host.engine_state(), ["smoke=running"]);

    host.ok(&["stop", "smoke"]);
    host.ok(&["restart", "smoke"]);
    assert_eq!(host.engine_state(), ["smoke=running"]);

    host.ok(&["stop", "smoke"]);
    host.ok(&[
        "--skip-policy",
        "--spec",
        "smoke.yaml",
        "run",
        "--runtime",
        engine,
    ]);
    assert_eq!(host.engine_state(), ["smoke=running"]);

    // A replacement that cannot start leaves the old container serving.
    host.fail_runs(true);
    let o = host.run(&["--spec", "smoke.yaml", "update", "smoke"]);
    assert!(!o.status.success());
    assert_eq!(host.engine_state(), ["smoke=running"]);
    host.fail_runs(false);
    host.ok(&["--spec", "smoke.yaml", "update", "smoke"]);
    assert_eq!(host.engine_state(), ["smoke=running"]);
}

#[test]
fn cli_lifecycle_on_podman() {
    cli_restart_update_and_run_again("podman");
}

#[test]
fn cli_lifecycle_on_docker() {
    cli_restart_update_and_run_again("docker");
}

#[test]
fn a_container_aether_did_not_create_is_left_alone() {
    let host = Host::with_smoke("podman");
    host.ok(&["stop", "smoke"]);
    // Somebody else's container now owns the name.
    let meta = host.containers().join("meta/smoke");
    std::fs::write(&meta, "foreign|\n").unwrap();
    let o = host.run(&[
        "--skip-policy",
        "--spec",
        "smoke.yaml",
        "run",
        "--runtime",
        "podman",
    ]);
    assert!(!o.status.success());
    assert_eq!(std::fs::read_to_string(&meta).unwrap(), "foreign|\n");
    assert_eq!(host.engine_state(), ["smoke=exited"]);
}
