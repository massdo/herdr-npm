use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use herdr_npm::adapters::herdr_socket::HerdrSocket;
use herdr_npm::application::ports::HerdrPort;
use herdr_npm::domain::error::AppError;
use serde_json::{Value, json};

fn temp_sock() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "herdr-npm-sock-{}-{}.sock",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&path);
    path
}

fn serve(path: PathBuf, handler: impl Fn(Value) -> Value + Send + 'static) {
    let listener = UnixListener::bind(&path).expect("bind");
    thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(mut stream) = incoming else { break };
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() {
                break;
            }
            if line.is_empty() {
                break;
            }
            let req: Value = serde_json::from_str(&line).unwrap_or(json!({}));
            let resp = handler(req);
            let _ = stream.write_all(format!("{resp}\n").as_bytes());
        }
        let _ = std::fs::remove_file(&path);
    });
    thread::sleep(Duration::from_millis(20));
}

#[test]
fn pane_list_parses_result() {
    let path = temp_sock();
    serve(path.clone(), |req| {
        json!({
            "id": req["id"],
            "result": {
                "panes": [{
                    "pane_id": "w1:p1",
                    "workspace_id": "w1",
                    "tab_id": "w1:t1",
                    "focused": true
                }]
            }
        })
    });
    let client = HerdrSocket::new(path);
    let panes = client.list_panes(None).unwrap();
    assert_eq!(panes[0].pane_id, "w1:p1");
}

#[test]
fn herdr_error_is_not_uncertain() {
    let path = temp_sock();
    serve(path.clone(), |req| {
        json!({
            "id": req["id"],
            "error": { "code": "invalid_params", "message": "nope" }
        })
    });
    let client = HerdrSocket::new(path);
    match client.list_panes(None) {
        Err(AppError::Herdr { code, .. }) => assert_eq!(code, "invalid_params"),
        other => panic!("expected herdr error, got {other:?}"),
    }
}

#[test]
fn mismatched_id_is_uncertain() {
    let path = temp_sock();
    serve(
        path.clone(),
        |_req| json!({ "id": "other", "result": { "panes": [] } }),
    );
    let client = HerdrSocket::new(path);
    assert!(matches!(
        client.list_panes(None),
        Err(AppError::Uncertain { .. })
    ));
}

#[test]
fn missing_result_is_uncertain() {
    let path = temp_sock();
    serve(path.clone(), |req| json!({ "id": req["id"] }));
    let client = HerdrSocket::new(path);
    assert!(matches!(
        client.list_panes(None),
        Err(AppError::Uncertain { .. })
    ));
}
