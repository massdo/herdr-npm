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

#[test]
fn tab_create_parses_tab_and_root_pane() {
    let path = temp_sock();
    serve(path.clone(), |req| {
        json!({
            "id": req["id"],
            "result": {
                "type": "tab_created",
                "tab": { "tab_id": "tab-dev" },
                "root_pane": { "pane_id": "tab-dev:p1" }
            }
        })
    });
    let client = HerdrSocket::new(path);
    let created = client
        .create_tab(herdr_npm::application::ports::CreateTab {
            workspace_id: "main".into(),
            cwd: "/work/app".into(),
            label: "npm run -- dev".into(),
            focus: false,
        })
        .unwrap();
    assert_eq!(created.tab_id.as_str(), "tab-dev");
    assert_eq!(created.root_pane_id.as_str(), "tab-dev:p1");
}

#[test]
fn tab_create_error_is_not_a_confirmed_launch() {
    let path = temp_sock();
    serve(path.clone(), |req| {
        json!({
            "id": req["id"],
            "error": { "code": "failed", "message": "cannot create tab" }
        })
    });
    let client = HerdrSocket::new(path);
    match client.create_tab(herdr_npm::application::ports::CreateTab {
        workspace_id: "main".into(),
        cwd: "/work/app".into(),
        label: "npm run -- dev".into(),
        focus: false,
    }) {
        Err(AppError::Herdr { code, .. }) => assert_eq!(code, "failed"),
        other => panic!("expected herdr error, got {other:?}"),
    }
}

#[test]
fn tab_create_missing_root_pane_is_uncertain() {
    let path = temp_sock();
    serve(path.clone(), |req| {
        json!({
            "id": req["id"],
            "result": { "tab": { "tab_id": "tab-dev" } }
        })
    });
    let client = HerdrSocket::new(path);
    assert!(matches!(
        client.create_tab(herdr_npm::application::ports::CreateTab {
            workspace_id: "main".into(),
            cwd: "/work/app".into(),
            label: "npm run -- dev".into(),
            focus: false,
        }),
        Err(AppError::Uncertain { .. })
    ));
}

#[test]
fn tab_create_non_string_ids_are_uncertain() {
    let path = temp_sock();
    serve(path.clone(), |req| {
        json!({
            "id": req["id"],
            "result": {
                "tab": { "tab_id": 1 },
                "root_pane": { "pane_id": true }
            }
        })
    });
    let client = HerdrSocket::new(path);
    assert!(matches!(
        client.create_tab(herdr_npm::application::ports::CreateTab {
            workspace_id: "main".into(),
            cwd: "/work/app".into(),
            label: "npm run -- dev".into(),
            focus: false,
        }),
        Err(AppError::Uncertain { .. })
    ));
}

#[test]
fn timeout_after_tab_create_does_not_retry_send() {
    use std::sync::{Arc, Mutex};

    use herdr_npm::application::run_script::run_script;
    use herdr_npm::domain::catalog::{PackageCatalog, PackageManager, Script};

    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_for_server = Arc::clone(&seen);
    let path = temp_sock();
    serve(path.clone(), move |req| {
        seen_for_server
            .lock()
            .unwrap()
            .push(req["method"].as_str().unwrap_or("").to_string());
        if req["method"] == "tab.create" {
            return json!({
                "id": req["id"],
                "result": {
                    "type": "tab_created",
                    "tab": { "tab_id": "tab-dev" },
                    "root_pane": { "pane_id": "tab-dev:p1" }
                }
            });
        }
        thread::sleep(Duration::from_secs(6));
        json!({ "id": req["id"], "result": { "type": "ok" } })
    });
    let client = HerdrSocket::new(path);
    let catalog = PackageCatalog {
        root: "/work/app".into(),
        display_name: "app".into(),
        manager: PackageManager::Npm,
        scripts: vec![Script {
            name: "dev".into(),
            command: "vite".into(),
        }],
    };
    match run_script(&client, &catalog, "main", "dev") {
        Err(AppError::LaunchNotConfirmed { tab_id }) => {
            assert_eq!(tab_id.as_deref(), Some("tab-dev"));
        }
        other => panic!("expected LaunchNotConfirmed, got {other:?}"),
    }
    assert_eq!(*seen.lock().unwrap(), ["tab.create", "pane.send_input"]);
}

#[test]
fn send_input_is_forwarded_once() {
    use std::sync::{Arc, Mutex};
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_for_server = Arc::clone(&seen);
    let path = temp_sock();
    serve(path.clone(), move |req| {
        seen_for_server.lock().unwrap().push(req.clone());
        json!({ "id": req["id"], "result": { "type": "ok" } })
    });
    let client = HerdrSocket::new(path);
    client
        .send_input(
            &herdr_npm::domain::ids::PaneId("tab-dev:p1".into()),
            "npm run -- dev",
            &["Enter"],
        )
        .unwrap();
    let calls = seen.lock().unwrap().clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["method"], "pane.send_input");
    assert_eq!(calls[0]["params"]["text"], "npm run -- dev");
    assert_eq!(calls[0]["params"]["keys"][0], "Enter");
}
