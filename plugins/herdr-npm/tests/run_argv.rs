use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use herdr_npm::domain::catalog::PackageManager;
use herdr_npm::domain::run_command::run_invocation;

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
