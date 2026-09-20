use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// CLI wrapper for the isolated e2e Herdr session.
pub struct LiveHerdr {
    session: String,
    config: PathBuf,
    xdg: PathBuf,
    socket: PathBuf,
}

impl LiveHerdr {
    pub fn from_env() -> Self {
        Self {
            session: env("HERDR_NPM_E2E_SESSION"),
            config: PathBuf::from(env("HERDR_NPM_E2E_CONFIG")),
            xdg: PathBuf::from(env("HERDR_NPM_E2E_XDG")),
            socket: PathBuf::from(env("HERDR_NPM_E2E_SOCKET")),
        }
    }

    pub fn session(&self) -> &str {
        &self.session
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    pub fn config(&self) -> &Path {
        &self.config
    }

    pub fn output(&self, args: &[&str]) -> Output {
        Command::new("herdr")
            .arg("--session")
            .arg(&self.session)
            .args(args)
            .env("HERDR_CONFIG_PATH", &self.config)
            .env("XDG_CONFIG_HOME", &self.xdg)
            .env("HERDR_SOCKET_PATH", &self.socket)
            .output()
            .unwrap_or_else(|error| panic!("herdr {args:?}: {error}"))
    }

    pub fn stdout(&self, args: &[&str]) -> String {
        let output = self.output(args);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "herdr {args:?} failed ({})\nstdout={stdout}\nstderr={stderr}",
            output.status
        );
        stdout
    }

    pub fn json(&self, args: &[&str]) -> Value {
        let output = self.output(args);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "herdr {args:?} failed ({})\nstdout={stdout}\nstderr={stderr}",
            output.status
        );
        serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
            panic!("herdr {args:?} produced non-JSON stdout: {error}\n{stdout}")
        })
    }

    pub fn result(&self, args: &[&str]) -> Value {
        self.json(args)
            .get("result")
            .cloned()
            .unwrap_or_else(|| panic!("herdr {args:?} has no result"))
    }

    pub fn rpc(&self, method: &str, params: Value) -> Value {
        let mut stream =
            UnixStream::connect(&self.socket).unwrap_or_else(|error| panic!("connect: {error}"));
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        let request = json!({
            "id": format!("e2e:{method}"),
            "method": method,
            "params": params,
        });
        stream
            .write_all(format!("{request}\n").as_bytes())
            .expect("write rpc");
        let mut line = String::new();
        BufReader::new(stream)
            .read_line(&mut line)
            .expect("read rpc");
        serde_json::from_str(&line).unwrap_or_else(|error| panic!("rpc json: {error} ({line})"))
    }

    pub fn focus_pane(&self, pane_id: &str) {
        let response = self.rpc("pane.focus", json!({ "pane_id": pane_id }));
        if response.get("error").is_some() {
            panic!("pane.focus {pane_id} failed: {response}");
        }
    }

    pub fn panes(&self) -> Vec<Value> {
        self.result(&["pane", "list"])
            .get("panes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    pub fn tabs(&self) -> Vec<Value> {
        self.result(&["tab", "list"])
            .get("tabs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    pub fn wait_until(&self, timeout: Duration, mut probe: impl FnMut(&Self) -> bool) {
        let start = Instant::now();
        loop {
            if probe(self) {
                return;
            }
            if start.elapsed() > timeout {
                panic!("timed out after {timeout:?} waiting on live Herdr");
            }
            std::thread::sleep(Duration::from_millis(80));
        }
    }
}

pub fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} is required for @e2e"))
}

pub fn env_path(name: &str) -> PathBuf {
    PathBuf::from(env(name))
}

pub fn pane_id(pane: &Value) -> &str {
    pane.get("pane_id")
        .and_then(Value::as_str)
        .expect("pane_id")
}

pub fn tab_id(value: &Value) -> &str {
    value.get("tab_id").and_then(Value::as_str).expect("tab_id")
}

pub fn is_npm_sidebar(pane: &Value) -> bool {
    pane.get("tokens")
        .and_then(|tokens| tokens.get("herdr_npm_sidebar"))
        .and_then(Value::as_str)
        == Some("v1")
}

pub fn is_explorer(pane: &Value) -> bool {
    pane.get("tokens")
        .and_then(|tokens| tokens.get("herdr-sidebar-explorer"))
        .is_some()
        || matches!(
            pane.get("label").and_then(Value::as_str),
            Some("Sidebar" | "Explorer")
        )
}
