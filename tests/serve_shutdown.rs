// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! `aether serve` must exit on SIGTERM even while a dashboard holds the event
//! stream open. If it only closes the listener, its background loops keep
//! writing state next to whatever replaced it.
#![cfg(unix)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn sigterm_ends_the_server_while_an_event_stream_is_open() {
    let home = tempfile::tempdir().unwrap();
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut server = Server(
        Command::new(env!("CARGO_BIN_EXE_aether"))
            .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
            .current_dir(home.path())
            .env("HOME", home.path())
            .env("KUBECONFIG", "/dev/null")
            .env("AETHER_DEMO_AUTH", "0")
            .env_remove("AETHER_API_KEY")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut stream = loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(stream) => break stream,
            Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => panic!("the server did not start: {error}"),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .write_all(b"GET /api/events/stream HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .unwrap();
    let mut head = [0_u8; 15];
    stream.read_exact(&mut head).unwrap();
    assert_eq!(&head, b"HTTP/1.1 200 OK");

    let sent = Command::new("kill")
        .args(["-TERM", &server.0.id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());

    // The client is still connected; only the server may end this.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = server.0.try_wait().unwrap() {
            assert!(status.success(), "unclean exit: {status}");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the server was still running 10s after SIGTERM"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    drop(stream);
}
