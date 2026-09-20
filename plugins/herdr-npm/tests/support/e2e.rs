use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde_json::Value;

use super::live::{LiveHerdr, env, env_path, is_explorer, is_npm_sidebar, pane_id, tab_id};
use super::world::BddWorld;

pub fn requested() -> bool {
    std::env::var("HERDR_NPM_E2E").ok().as_deref() == Some("1")
}

pub fn active() -> bool {
    requested() && std::env::var("HERDR_NPM_E2E_SESSION").is_ok()
}

pub fn require_isolated() {
    assert!(
        requested(),
        "@e2e requires HERDR_NPM_E2E=1 and the isolated Herdr recipe profile; refusing to mutate anything else"
    );
    let session = env("HERDR_NPM_E2E_SESSION");
    assert!(
        session.starts_with("herdr-npm-e2e-"),
        "e2e session {session} is not herdr-npm-e2e-<run>"
    );
    let socket = env_path("HERDR_NPM_E2E_SOCKET");
    let config = env_path("HERDR_NPM_E2E_CONFIG");
    let xdg = env_path("HERDR_NPM_E2E_XDG");
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let user_socket = home.join(".config/herdr/herdr.sock");
    let user_config = home.join(".config/herdr/config.toml");
    assert_ne!(
        socket, user_socket,
        "e2e socket must not be the user default session"
    );
    assert_ne!(
        config, user_config,
        "e2e config must not be the personal config.toml"
    );
    assert!(
        socket.starts_with(&xdg),
        "e2e socket {} is outside XDG {}",
        socket.display(),
        xdg.display()
    );
    let injected = PathBuf::from(env("HERDR_SOCKET_PATH"));
    assert_eq!(
        injected, socket,
        "HERDR_SOCKET_PATH must match the e2e socket"
    );
}

pub fn prepare(world: &mut BddWorld) {
    require_isolated();
    world.fs_root = env_path("HERDR_NPM_E2E_FS_ROOT");
    world.argv_file = env_path("HERDR_NPM_E2E_ARGV");
    let _ = fs::create_dir_all(&world.fs_root);
    reset_layout(world);
}

pub fn live() -> LiveHerdr {
    LiveHerdr::from_env()
}

pub fn fixture() -> PathBuf {
    env_path("HERDR_NPM_E2E_FIXTURE")
}

pub fn hold_log() -> PathBuf {
    env_path("HERDR_NPM_E2E_HOLD")
}

pub fn reset_layout(world: &mut BddWorld) {
    let herdr = live();
    let tabs = herdr.tabs();
    if tabs.is_empty() {
        herdr.json(&[
            "workspace",
            "create",
            "--cwd",
            fixture().to_str().expect("fixture"),
            "--label",
            "e2e",
            "--no-focus",
        ]);
    }
    let tabs = herdr.tabs();
    let keep = tabs
        .first()
        .and_then(|tab| tab.get("tab_id"))
        .and_then(Value::as_str)
        .unwrap_or("w1:t1")
        .to_string();
    for tab in herdr.tabs() {
        let id = tab_id(&tab).to_string();
        if id != keep {
            let _ = herdr.output(&["tab", "close", &id]);
        }
    }
    herdr.wait_until(Duration::from_secs(5), |herdr| herdr.tabs().len() == 1);
    for pane in herdr.panes() {
        if is_npm_sidebar(&pane) {
            let id = pane_id(&pane);
            let _ = herdr.output(&["plugin", "pane", "close", id]);
            let _ = herdr.output(&["pane", "close", id]);
        } else if is_explorer(&pane) {
            let _ = herdr.output(&["pane", "close", pane_id(&pane)]);
        }
    }
    herdr.wait_until(Duration::from_secs(5), |herdr| {
        herdr
            .panes()
            .iter()
            .all(|pane| !is_npm_sidebar(pane) && !is_explorer(pane))
    });
    if let Some(pane) = herdr.panes().first() {
        let id = pane_id(pane).to_string();
        world.e2e_working_pane = Some(id);
    }
    world.e2e_sidebar_pane = None;
    world.e2e_script_tab = None;
    world.e2e_script_pane = None;
    world.e2e_script_pid = None;
    world.e2e_restored_pane = None;
    world.e2e_explorer_pane = None;
    world.workspace_id = herdr
        .panes()
        .first()
        .and_then(|pane| pane.get("workspace_id"))
        .and_then(Value::as_str)
        .unwrap_or("w1")
        .to_string();
    let _ = fs::remove_file(&world.argv_file);
    let _ = fs::remove_file(hold_log());
    let _ = fs::remove_file(pid_file());
}

pub fn working_pane(world: &BddWorld) -> String {
    world
        .e2e_working_pane
        .clone()
        .or_else(|| {
            live()
                .panes()
                .into_iter()
                .find(|pane| !is_npm_sidebar(pane) && !is_explorer(pane))
                .map(|pane| pane_id(&pane).to_string())
        })
        .expect("working pane")
}

pub fn invoke_toggle() {
    let herdr = live();
    let fixture = fixture();
    if let Some(pane) = herdr.panes().into_iter().find(|pane| {
        !is_npm_sidebar(pane)
            && !is_explorer(pane)
            && pane.get("cwd").and_then(Value::as_str).is_some_and(|cwd| {
                Path::new(cwd) == fixture.as_path()
                    || cwd.ends_with("work/app")
                    || Path::new(cwd) == fixture.canonicalize().unwrap_or(fixture.clone())
            })
    }) {
        herdr.focus_pane(pane_id(&pane));
    } else if let Some(pane) = herdr
        .panes()
        .into_iter()
        .find(|pane| !is_npm_sidebar(pane) && !is_explorer(pane))
    {
        herdr.focus_pane(pane_id(&pane));
    }
    herdr.json(&["plugin", "action", "invoke", "herdr-npm.toggle"]);
}

pub fn open_sidebar(world: &mut BddWorld) {
    if let Some(pane) = sidebar_pane() {
        let _ = live().output(&["plugin", "pane", "close", &pane]);
        wait_sidebar_gone();
    }
    invoke_toggle();
    live().wait_until(Duration::from_secs(8), |_| sidebar_pane().is_some());
    if let Some(pane) = sidebar_pane() {
        let _ = live().output(&["plugin", "pane", "focus", &pane]);
        world.e2e_sidebar_pane = Some(pane);
    }
}

pub fn sidebar_pane() -> Option<String> {
    live()
        .panes()
        .into_iter()
        .find(is_npm_sidebar)
        .map(|pane| pane_id(&pane).to_string())
}

pub fn wait_sidebar_gone() {
    live().wait_until(Duration::from_secs(8), |herdr| {
        !herdr.panes().iter().any(is_npm_sidebar)
    });
}

pub fn read_pane(pane: &str) -> String {
    live().stdout(&[
        "pane", "read", pane, "--source", "recent", "--format", "text",
    ])
}

/// Command shown in the TUI footer (the line immediately above `h/l scroll`).
pub fn footer_command(pane: &str) -> String {
    let text = read_pane(pane);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if let Some(index) = lines.iter().position(|line| line.contains("h/l scroll"))
        && index > 0
    {
        return lines[index - 1].to_string();
    }
    String::new()
}

pub fn catalog_ready(pane: &str) -> bool {
    let text = read_pane(pane);
    text.contains("dev") && text.contains("build") && text.contains("h/l scroll")
}

pub fn send_keys(pane: &str, keys: &[&str]) {
    let mut args = vec!["pane", "send-keys", pane];
    args.extend_from_slice(keys);
    let output = live().output(&args);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "herdr {args:?} failed ({})\nstdout={stdout}\nstderr={stderr}",
        output.status
    );
}

pub fn run_script(world: &mut BddWorld, name: &str) {
    open_sidebar(world);
    let sidebar = world
        .e2e_sidebar_pane
        .clone()
        .or_else(sidebar_pane)
        .expect("sidebar pane");
    select_script(&sidebar, name);
    let before: Vec<String> = live()
        .tabs()
        .iter()
        .map(|tab| tab_id(tab).to_string())
        .collect();
    send_keys(&sidebar, &["Enter"]);
    live().wait_until(Duration::from_secs(8), |herdr| {
        herdr
            .tabs()
            .iter()
            .any(|tab| !before.iter().any(|id| id == tab_id(tab)))
    });
    let created = live()
        .tabs()
        .into_iter()
        .find(|tab| !before.iter().any(|id| id == tab_id(tab)))
        .expect("new script tab");
    world.e2e_script_tab = Some(tab_id(&created).to_string());
    world.e2e_script_pane = live()
        .panes()
        .into_iter()
        .find(|pane| pane.get("tab_id").and_then(Value::as_str) == Some(tab_id(&created)))
        .map(|pane| pane_id(&pane).to_string());
    if name == "dev" {
        live().wait_until(Duration::from_secs(8), |_| pid_file().is_file());
    }
    if let Ok(raw) = fs::read_to_string(pid_file())
        && let Ok(pid) = raw.trim().parse::<i32>()
    {
        world.e2e_script_pid = Some(pid);
    }
}

pub fn select_script(sidebar: &str, name: &str) {
    for _ in 0..5 {
        send_keys(sidebar, &["k"]);
    }
    let index = match name {
        "dev" | "hello" => 0,
        "build" => 1,
        "test" => 2,
        other => panic!("no e2e index for script {other}"),
    };
    for _ in 0..index {
        send_keys(sidebar, &["j"]);
        std::thread::sleep(Duration::from_millis(60));
    }
    let text = read_pane(sidebar);
    assert!(
        text.contains(name) || name == "dev" && text.contains("vite"),
        "sidebar does not show {name}:\n{text}"
    );
}

pub fn last_argv() -> Vec<String> {
    let path = env_path("HERDR_NPM_E2E_ARGV");
    live().wait_until(Duration::from_secs(8), |_| path.is_file());
    serde_json::from_slice(&fs::read(path).expect("argv file")).expect("argv json")
}

pub fn pid_alive(pid: i32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn pid_file() -> PathBuf {
    PathBuf::from(format!("{}.pid", hold_log().display()))
}

pub fn assert_binding_in_config(key: &str) {
    let text = fs::read_to_string(live().config()).expect("e2e config");
    assert!(
        text.contains(key) && text.contains("herdr-npm.toggle"),
        "config does not bind {key} to herdr-npm.toggle:\n{text}"
    );
}

pub fn open_explorer(world: &mut BddWorld) {
    if let Some(pane) = sidebar_pane() {
        let _ = live().output(&["plugin", "pane", "close", &pane]);
        wait_sidebar_gone();
    }
    let target = working_pane(world);
    live().json(&[
        "plugin",
        "pane",
        "open",
        "--plugin",
        "herdr-sidebar",
        "--entrypoint",
        "sidebar",
        "--placement",
        "split",
        "--target-pane",
        &target,
        "--direction",
        "right",
        "--no-focus",
    ]);
    live().wait_until(Duration::from_secs(8), |herdr| {
        herdr.panes().iter().any(is_explorer)
    });
    let explorer = live()
        .panes()
        .into_iter()
        .find(is_explorer)
        .expect("explorer pane");
    let explorer_id = pane_id(&explorer).to_string();
    live().json(&[
        "pane",
        "swap",
        "--source-pane",
        &explorer_id,
        "--target-pane",
        &target,
    ]);
    world.e2e_explorer_pane = Some(explorer_id);
    world.e2e_working_pane = live()
        .panes()
        .into_iter()
        .find(|pane| !is_explorer(pane) && !is_npm_sidebar(pane))
        .map(|pane| pane_id(&pane).to_string());
}

pub fn layout_for(pane: &str) -> Value {
    live().result(&["pane", "layout", "--pane", pane])
}

pub fn restart_server() {
    let herdr = live();
    let session = herdr.session().to_string();
    let _ = Command::new("herdr")
        .args(["session", "stop", &session])
        .env("HERDR_CONFIG_PATH", herdr.config())
        .env("XDG_CONFIG_HOME", env_path("HERDR_NPM_E2E_XDG"))
        .status();
    std::thread::sleep(Duration::from_millis(400));
    let log = env_path("HERDR_NPM_E2E_SERVER_LOG");
    let child = Command::new("herdr")
        .args(["--session", &session, "server"])
        .env("HERDR_CONFIG_PATH", herdr.config())
        .env("XDG_CONFIG_HOME", env_path("HERDR_NPM_E2E_XDG"))
        .stdout(fs::File::create(&log).expect("server log"))
        .stderr(fs::File::create(&log).expect("server log"))
        .spawn()
        .expect("restart herdr server");
    let _ = fs::write(env_path("HERDR_NPM_E2E_SERVER_PID"), child.id().to_string());
    std::mem::forget(child);
    live().wait_until(Duration::from_secs(10), |herdr| herdr.socket().exists());
    std::thread::sleep(Duration::from_millis(500));
}
