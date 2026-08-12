mod support;

use std::thread;
use std::time::Duration;

use predicates::prelude::*;
use support::Fixture;
use tiny_http::{Response, Server};

fn server_with_status(status: u16) -> (String, thread::JoinHandle<()>) {
    let server = Server::http("127.0.0.1:0").expect("start test server");
    let address = format!("http://{}", server.server_addr());
    let handle = thread::spawn(move || {
        if let Ok(request) = server.recv() {
            request
                .respond(Response::empty(status))
                .expect("respond to doctor");
        }
    });
    (address, handle)
}

#[test]
fn doctor_checks_models_without_launching_inference() {
    let fixture = Fixture::new();
    let marker = fixture.marker_path();
    fixture.set_fake_claude(&format!("#!/bin/zsh\nprint ran > {}\n", marker.display()));
    let (base_url, server) = server_with_status(200);
    fixture.set_base_url(&base_url);

    fixture
        .command()
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS proxy-models"))
        .stdout(predicate::str::contains("live-inference").not());
    server.join().expect("join test server");
    assert!(!marker.exists(), "default doctor must not run inference");
}

#[test]
fn doctor_live_launches_claude_code_in_print_mode() {
    let fixture = Fixture::new();
    fixture.set_fake_claude(
        r#"#!/bin/zsh
set -e
[[ "$1" == "--model" ]]
[[ "$3" == "-p" ]]
[[ "$4" == "--no-session-persistence" ]]
[[ "$5" == "Return exactly CLAUDEX_DOCTOR_OK" ]]
print -r -- "CLAUDEX_DOCTOR_OK"
"#,
    );
    let (base_url, server) = server_with_status(200);
    fixture.set_base_url(&base_url);

    fixture
        .command()
        .args(["doctor", "--live"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PASS live-inference"));
    server.join().expect("join test server");
}

#[test]
fn doctor_classifies_http_failure() {
    let fixture = Fixture::new();
    let (base_url, server) = server_with_status(503);
    fixture.set_base_url(&base_url);

    fixture
        .command()
        .arg("doctor")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("HTTP 503"));
    server.join().expect("join test server");
}

#[test]
fn doctor_classifies_timeout() {
    let fixture = Fixture::new();
    let server = Server::http("127.0.0.1:0").expect("start slow server");
    let address = format!("http://{}", server.server_addr());
    fixture.set_base_url(&address);
    let handle = thread::spawn(move || {
        if let Ok(request) = server.recv() {
            thread::sleep(Duration::from_millis(250));
            let _ = request.respond(Response::empty(200));
        }
    });

    fixture
        .command()
        .env("CLAUDEX_DOCTOR_TIMEOUT_MS", "50")
        .arg("doctor")
        .assert()
        .code(3)
        .stderr(predicate::str::contains("timed out"));
    handle.join().expect("join slow server");
}

#[test]
fn doctor_live_classifies_wrong_response() {
    let fixture = Fixture::new();
    fixture.set_fake_claude("#!/bin/zsh\nprint -r -- WRONG\n");
    let (base_url, server) = server_with_status(200);
    fixture.set_base_url(&base_url);

    fixture
        .command()
        .args(["doctor", "--live"])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("expected CLAUDEX_DOCTOR_OK"));
    server.join().expect("join test server");
}

#[test]
fn doctor_live_times_out_and_classifies_launch_failures() {
    let fixture = Fixture::new();
    fixture.set_fake_claude("#!/bin/zsh\nsleep 1\nprint -r -- CLAUDEX_DOCTOR_OK\n");
    let (base_url, server) = server_with_status(200);
    fixture.set_base_url(&base_url);

    fixture
        .command()
        .env("CLAUDEX_DOCTOR_TIMEOUT_MS", "50")
        .args(["doctor", "--live"])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("timed out"));
    server.join().expect("join test server");
}

#[test]
fn doctor_live_classifies_spawn_and_nonzero_failures() {
    let fixture = Fixture::new();
    fixture.set_fake_claude("#!/does/not/exist\n");
    let (base_url, server) = server_with_status(200);
    fixture.set_base_url(&base_url);
    fixture
        .command()
        .args(["doctor", "--live"])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("cannot launch Claude Code"));
    server.join().expect("join test server");

    let fixture = Fixture::new();
    fixture.set_fake_claude("#!/bin/zsh\nexit 9\n");
    let (base_url, server) = server_with_status(200);
    fixture.set_base_url(&base_url);
    fixture
        .command()
        .args(["doctor", "--live"])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("exited with"));
    server.join().expect("join test server");
}
