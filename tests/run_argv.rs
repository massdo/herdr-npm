use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use herdr_npm::domain::catalog::PackageManager;
use herdr_npm::domain::run_command::run_invocation;

#[allow(dead_code)]
#[path = "support/fake_herdr.rs"]
mod fake_herdr;
mod workspace_fixture;

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "herdr-npm-argv-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn install_fake_manager(bin: &Path) {
    fs::create_dir_all(bin).unwrap();
    let body = r#"#!/usr/bin/env python3
import json, os, sys
with open(os.environ["HERDR_NPM_ARGV"], "w") as handle:
    json.dump(sys.argv, handle)
"#;
    for name in ["npm", "pnpm"] {
        let path = bin.join(name);
        fs::write(&path, body).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn run_with_shell(shell: &str, invocation: &str, bin: &Path, argv_file: &Path) -> Vec<String> {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let status = Command::new(shell)
        .arg("-c")
        .arg(invocation)
        .env("PATH", path)
        .env("HERDR_NPM_ARGV", argv_file)
        .status()
        .unwrap_or_else(|error| panic!("{shell}: {error}"));
    assert!(status.success(), "{shell} failed for {invocation}");
    serde_json::from_slice(&fs::read(argv_file).unwrap()).unwrap()
}

fn assert_literal(shell: &str, script: &str) {
    let root = temp_dir();
    let bin = root.join("bin");
    let argv_file = root.join("argv.json");
    install_fake_manager(&bin);
    let invocation = run_invocation(PackageManager::Npm, script);
    let argv = run_with_shell(shell, &invocation, &bin, &argv_file);
    assert_eq!(&argv[1..], ["run", "--", script], "{shell}: {argv:?}");
    fs::remove_dir_all(&root).ok();
}

const SCRIPTS: &[&str] = &[
    "build",
    "build:prod",
    "test watch",
    "say\"hi\"",
    "it's",
    "$HOME",
    "x$(id)",
    "a; id",
    "--prod",
    "-dev",
];

#[test]
fn sh_forwards_script_names_literally() {
    for script in SCRIPTS {
        assert_literal("sh", script);
    }
}

#[test]
fn bash_forwards_script_names_literally() {
    if Command::new("bash").arg("-c").arg("true").status().is_err() {
        return;
    }
    for script in SCRIPTS {
        assert_literal("bash", script);
    }
}

#[test]
fn zsh_forwards_script_names_literally() {
    if Command::new("zsh").arg("-c").arg("true").status().is_err() {
        return;
    }
    for script in SCRIPTS {
        assert_literal("zsh", script);
    }
}

fn assert_real_manager(manager: &str) {
    let root = temp_dir();
    fs::write(
        root.join("package.json"),
        r#"{"name":"argv-test","scripts":{"--prod":"echo prod","-dev":"echo dev","test watch":"echo tw"}}"#,
    )
    .unwrap();
    for script in ["--prod", "-dev", "test watch"] {
        let output = Command::new(manager)
            .args(["run", "--", script])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{manager} run -- {script:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn real_npm_sees_dashed_names_as_scripts_when_present() {
    assert_real_manager("npm");
}

#[test]
fn real_pnpm_sees_dashed_names_as_scripts_when_present() {
    assert_real_manager("pnpm");
}

#[test]
fn workspace_homonyms_run_once_in_their_own_packages_with_their_managers() {
    use herdr_npm::adapters::fs_project::FsProject;
    use herdr_npm::adapters::tui::{app::SidebarApp, flush_intents};
    use herdr_npm::application::list_scripts::{list_scripts, origin_for_paths};
    use herdr_npm::domain::catalog::RunIntent;

    let f = workspace_fixture::Fixture::journal();
    f.package("apps/auth", r#"{"name":"same-name","packageManager":"npm@10","scripts":{"dev":"echo witness >> witness.txt"}}"#);
    assert_eq!(f.workspace("").packages.len(), 6);
    let bin = f.path("bin");
    fs::create_dir(&bin).unwrap();
    for manager in ["npm", "pnpm"] {
        let output = Command::new("sh")
            .args(["-c", &format!("command -v {manager}")])
            .output()
            .unwrap();
        assert!(output.status.success());
        let real = String::from_utf8(output.stdout).unwrap().trim().to_string();
        let body = format!(
            r#"#!/usr/bin/env python3
import json, os, sys
with open("invocations.jsonl", "a") as handle:
    handle.write(json.dumps({{"manager": {manager:?}, "argv": sys.argv[1:], "cwd": os.getcwd()}}) + "\n")
real = {real}
os.execv(real, [real, *sys.argv[1:]])
"#,
            real = serde_json::to_string(&real).unwrap()
        );
        fs::write(bin.join(manager), body).unwrap();
        fs::set_permissions(bin.join(manager), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let listed = list_scripts(
        &FsProject::capped(f.path("")),
        &origin_for_paths(Some(f.path("")), None),
    );
    let mut app = SidebarApp::new(listed);
    app.workspace_id = "captured-workspace".into();
    for path in ["apps/auth", "apps/mcp"] {
        app.run_intents.push(RunIntent {
            package_root: f.path(path),
            script_name: "dev".into(),
        });
    }
    let herdr = fake_herdr::FakeHerdr::default();
    herdr.set_shell_exec(bin, f.path("unused.json"));
    flush_intents(&mut app, &herdr);
    assert!(app.launch_error.is_none(), "{:?}", app.launch_error);
    flush_intents(&mut app, &herdr);
    let tabs = herdr.created_tabs();
    assert_eq!(tabs.len(), 2);
    for (index, (path, manager)) in [("apps/auth", "npm"), ("apps/mcp", "pnpm")]
        .iter()
        .enumerate()
    {
        assert_eq!(tabs[index].cwd, f.path(path));
        assert_eq!(tabs[index].workspace_id, "captured-workspace");
        assert!(!tabs[index].focus);
        assert_eq!(tabs[index].label, format!("{manager} run -- dev"));
        assert_eq!(
            fs::read_to_string(f.path(&format!("{path}/witness.txt"))).unwrap(),
            "witness\n"
        );
        let lines = fs::read_to_string(f.path(&format!("{path}/invocations.jsonl"))).unwrap();
        assert_eq!(lines.lines().count(), 1);
        let record: serde_json::Value = serde_json::from_str(lines.trim()).unwrap();
        assert_eq!(record["manager"], *manager);
        assert_eq!(record["argv"], serde_json::json!(["run", "--", "dev"]));
        assert_eq!(record["cwd"], f.path(path).to_str().unwrap());
    }
}

#[test]
fn unavailable_workspace_identity_cannot_launch_and_uncertainty_is_not_retried() {
    use herdr_npm::application::run_script::run_intent;
    use herdr_npm::domain::catalog::RunIntent;
    use herdr_npm::domain::error::AppError;
    let f = workspace_fixture::Fixture::journal();
    let project = f.load("").catalog.unwrap();
    let herdr = fake_herdr::FakeHerdr::default();
    for (package, script) in [
        ("unlisted", "dev"),
        ("apps/mcp", "missing"),
        ("packages/core", "dev"),
    ] {
        let intent = RunIntent {
            package_root: f.path(package),
            script_name: script.into(),
        };
        assert!(matches!(
            run_intent(&herdr, &project, "w1", &intent),
            Err(AppError::ScriptUnavailable { .. })
        ));
    }
    assert!(herdr.calls().is_empty());
    herdr.set_send_timeout();
    let intent = RunIntent {
        package_root: f.path("apps/mcp"),
        script_name: "dev".into(),
    };
    assert!(matches!(
        run_intent(&herdr, &project, "w1", &intent),
        Err(AppError::LaunchNotConfirmed { .. })
    ));
    assert_eq!(herdr.created_tabs().len(), 1);
    assert_eq!(
        herdr
            .calls()
            .iter()
            .filter(|call| call.method == "pane.send_input")
            .count(),
        1
    );
}
