//! Acknowledging events over the API, which Viewers may not do.
//!
//! Runs the real binary on a temporary home directory.

use aether::events::{EventCategory, EventSeverity};
use aether::rbac::{RbacStore, Role};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Server {
    child: Child,
    port: u16,
}

impl Server {
    fn request(&self, method: &str, path: &str, key: &str) -> Option<(u16, String)> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).ok()?;
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .ok()?;
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {key}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
        )
        .ok()?;
        let mut reply = String::new();
        stream.read_to_string(&mut reply).ok()?;
        let status = reply.split_whitespace().nth(1)?.parse().ok()?;
        let (_, body) = reply.split_once("\r\n\r\n")?;
        Some((status, body.to_string()))
    }

    fn unacknowledged(&self, key: &str) -> u64 {
        let (_, body) = self.request("GET", "/api/events/summary", key).unwrap();
        let json = &body[body.find('{').unwrap()..=body.rfind('}').unwrap()];
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        v["data"]["unacknowledged"].as_u64().unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn operators_acknowledge_events_and_viewers_cannot() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join(".aether");
    std::fs::create_dir_all(&dir).unwrap();
    aether::ops::journal::event(
        &dir,
        EventSeverity::Critical,
        EventCategory::HealthCheck,
        Some("web"),
        "Incident opened",
        "web is not ready",
    )
    .unwrap();
    let mut keys = RbacStore::new();
    let viewer = keys.create_key("viewer", Role::Viewer);
    let operator = keys.create_key("operator", Role::Operator);
    keys.save(&dir.join("rbac.json")).unwrap();

    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let child = Command::new(env!("CARGO_BIN_EXE_aether"))
        .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
        .env("HOME", home.path())
        .env("XDG_RUNTIME_DIR", home.path())
        .env("KUBECONFIG", "/dev/null")
        .env("AETHER_OBSERVE_SECS", "0")
        .env("AETHER_DEMO_AUTH", "0")
        .env_remove("AETHER_API_KEY")
        .env_remove("AETHER_STATE_DATABASE_URL")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start aether serve");
    let server = Server { child, port };
    let started = Instant::now();
    while server.request("GET", "/health", "").is_none() {
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "the server did not start"
        );
        std::thread::sleep(Duration::from_millis(200));
    }

    assert_eq!(server.unacknowledged(&viewer), 1);
    let (code, body) = server
        .request("POST", "/api/events/1/acknowledge", &viewer)
        .unwrap();
    assert_eq!(code, 403, "{body}");
    assert_eq!(server.unacknowledged(&viewer), 1);

    let (code, body) = server
        .request("POST", "/api/events/1/acknowledge", &operator)
        .unwrap();
    assert_eq!(code, 200, "{body}");
    assert_eq!(server.unacknowledged(&viewer), 0);

    let (code, _) = server
        .request("POST", "/api/events/999/acknowledge", &operator)
        .unwrap();
    assert_eq!(code, 404);
}
