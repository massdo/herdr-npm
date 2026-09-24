use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use herdr_npm::adapters::fs_project::FsProject;
use herdr_npm::application::ports::{LoadedCatalog, ProjectPort};
use herdr_npm::domain::catalog::{ProjectCatalog, WorkspaceCatalog};

pub struct Fixture(PathBuf);

impl Fixture {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "herdr-npm-workspace-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }

    pub fn journal() -> Self {
        let f = Self::new();
        f.write(
            "pnpm-workspace.yaml",
            "packages:\n  - apps/*\n  - packages/*\nuseNodeVersion: 22.0.0\n",
        );
        f.package("", r#"{"name":"journal-fixture","packageManager":"pnpm@10.10.0","scripts":{"root":"echo root"}}"#);
        for path in ["apps/auth", "apps/cli", "apps/mcp"] {
            f.package(path, r#"{"name":"same-name","scripts":{"dev":"echo witness >> witness.txt","start":"echo start"}}"#);
        }
        f.package("packages/core", r#"{"name":"@journal/core"}"#);
        f.package(
            "packages/infrastructure",
            r#"{"name":"@journal/infrastructure","scripts":{}}"#,
        );
        fs::create_dir_all(f.path("apps/mcp/src")).unwrap();
        f
    }

    pub fn path(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }

    pub fn write(&self, path: &str, content: &str) {
        let path = self.path(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    pub fn package(&self, path: &str, json: &str) {
        self.write(
            &format!("{}/package.json", path.trim_end_matches('/')).trim_start_matches('/'),
            json,
        );
    }

    pub fn load(&self, origin: &str) -> LoadedCatalog {
        FsProject::capped(self.0.clone()).load_catalog(&self.path(origin))
    }

    pub fn workspace(&self, origin: &str) -> WorkspaceCatalog {
        match self.load(origin).catalog.unwrap() {
            ProjectCatalog::Workspace(w) => w,
            other => panic!("expected workspace, got {other:?}"),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
