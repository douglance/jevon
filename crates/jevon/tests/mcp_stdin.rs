//! That the MCP server never reads standard input behind the caller's back.
//!
//! Over MCP stdio that stream carries JSON-RPC frames. A command that calls
//! `read_to_string(stdin)` because no item source was named blocks its own tool
//! call forever and then starts consuming frames meant for the server — which
//! is what `classify` did: the call never answered, a frame sent two seconds
//! later still got a reply, and one sent four seconds later did not.
//!
//! This drives the real binary because that is the only place the bug exists.
//! Nothing in-process shares a standard input with the protocol.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::io::{BufRead as _, BufReader, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// How long to wait for any one reply. Generous: the point is to tell a reply
/// from no reply, not to measure latency.
const PATIENCE: Duration = Duration::from_secs(10);

fn frame(id: u32, method: &str, params: serde_json::Value) -> String {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string()
}

/// Collects answered request ids off the server's stdout until it goes quiet.
fn answered(stdout: ChildStdout) -> mpsc::Receiver<u32> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(id) = value.get("id").and_then(serde_json::Value::as_u64) {
                    #[expect(clippy::cast_possible_truncation, reason = "ids here are small")]
                    let _ = tx.send(id as u32);
                }
            }
        }
    });
    rx
}

/// Waits for one id, or gives up and says which ids did arrive.
fn expect_reply(rx: &mpsc::Receiver<u32>, want: u32, seen: &mut Vec<u32>) {
    let deadline = std::time::Instant::now() + PATIENCE;
    while std::time::Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(id) => {
                seen.push(id);
                if id == want {
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    panic!("no reply to request {want}; ids answered: {seen:?}");
}

/// Opens the session, as request 1.
fn handshake(stdin: &mut impl Write) {
    let hello = serde_json::json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {},
        "clientInfo": {"name": "regression", "version": "1"},
    });
    writeln!(stdin, "{}", frame(1, "initialize", hello)).unwrap();
    writeln!(
        stdin,
        "{}",
        serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})
    )
    .unwrap();
    stdin.flush().unwrap();
}

fn start() -> (Child, mpsc::Receiver<u32>) {
    let mut server = Command::new(env!("CARGO_BIN_EXE_jev"))
        .arg("--mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary must start");
    let rx = answered(server.stdout.take().expect("stdout is piped"));
    (server, rx)
}

/// The regression. `classify` with no item source must answer with an error,
/// and every later frame must still be answered — proving nothing swallowed
/// the stream.
#[test]
fn classify_without_an_item_source_answers_instead_of_eating_the_protocol() {
    let (mut server, rx) = start();
    let mut stdin = server.stdin.take().expect("stdin is piped");
    let mut seen = Vec::new();

    handshake(&mut stdin);
    expect_reply(&rx, 1, &mut seen);

    // No --items, --items-file or --stdin. This is the call that used to hang.
    let call = serde_json::json!({
        "name": "call_read_tool",
        "arguments": {"name": "classify", "arguments": {"noul": "Is this a greeting?"}},
    });
    writeln!(stdin, "{}", frame(2, "tools/call", call)).unwrap();
    stdin.flush().unwrap();
    expect_reply(&rx, 2, &mut seen);

    // The frame-eating showed up only after a pause, so the pause is the test.
    std::thread::sleep(Duration::from_secs(4));
    writeln!(stdin, "{}", frame(3, "tools/list", serde_json::json!({}))).unwrap();
    stdin.flush().unwrap();
    expect_reply(&rx, 3, &mut seen);

    server.kill().expect("the server must stop");
}
