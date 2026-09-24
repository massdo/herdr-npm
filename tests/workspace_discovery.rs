mod workspace_fixture;

use herdr_npm::domain::catalog::ProjectCatalog;
use herdr_npm::domain::error::AppError;
use workspace_fixture::Fixture;

#[test]
fn journal_from_root_member_and_intermediate_directory() {
    let f = Fixture::journal();
    for origin in ["", "apps/mcp", "apps", "apps/mcp/src"] {
        let workspace = f.workspace(origin);
        assert_eq!(workspace.packages.len(), 6);
        assert_eq!(
            workspace
                .packages
                .iter()
                .map(|p| p.relative_path.to_str().unwrap())
                .collect::<Vec<_>>(),
            [
                ".",
                "apps/auth",
                "apps/cli",
                "apps/mcp",
                "packages/core",
                "packages/infrastructure"
            ]
        );
        for p in &workspace.packages {
            assert_eq!(p.catalog.as_ref().unwrap().manager.as_str(), "pnpm");
        }
        assert!(
            workspace.packages[4]
                .catalog
                .as_ref()
                .unwrap()
                .scripts
                .is_empty()
        );
        assert!(
            workspace.packages[5]
                .catalog
                .as_ref()
                .unwrap()
                .scripts
                .is_empty()
        );
        assert_eq!(
            workspace.packages[3]
                .catalog
                .as_ref()
                .unwrap()
                .scripts
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            ["dev", "start"]
        );
    }
    assert_eq!(
        f.workspace("apps/mcp").active_package,
        Some(f.path("apps/mcp"))
    );
}

#[test]
fn npm_formats_globs_exclusions_duplicates_and_unlisted_package() {
    for workspaces in [
        r#"["apps/**", "apps/*", "libs/{a,b}[12]?", "!apps/excluded"]"#,
        r#"{"packages":["!apps/excluded", "apps/**", "apps/*", "libs/{a,b}[12]?"]}"#,
    ] {
        let f = Fixture::new();
        f.write("package.json", &format!(r#"{{"workspaces":{workspaces}}}"#));
        for path in [
            "apps/one",
            "apps/deep/two",
            "apps/excluded",
            "libs/a1x",
            "libs/b2y",
            "libs/a3x",
            "unlisted",
            "node_modules/bad",
            "apps/node_modules/bad",
            ".git/bad",
        ] {
            f.package(path, r#"{"scripts":{"dev":"echo ok"}}"#);
        }
        let w = f.workspace("");
        assert_eq!(
            w.packages
                .iter()
                .map(|p| p.relative_path.to_str().unwrap())
                .collect::<Vec<_>>(),
            [".", "apps/deep/two", "apps/one", "libs/a1x", "libs/b2y"]
        );
        assert!(matches!(
            f.load("unlisted").catalog,
            Ok(ProjectCatalog::Package(_))
        ));
    }
}

#[test]
fn pnpm_authority_nested_workspace_and_git_boundary() {
    let f = Fixture::journal();
    f.write("package.json", r#"{"workspaces":["unlisted"]}"#);
    f.package("unlisted", r#"{"scripts":{"dev":"echo ok"}}"#);
    assert_eq!(f.workspace("").packages.len(), 6);
    f.write("apps/mcp/pnpm-workspace.yaml", "packages: [sub/*]\n");
    f.package("apps/mcp/sub/one", "{}");
    assert_eq!(f.workspace("apps/mcp/sub/one").root, f.path("apps/mcp"));
    f.write("apps/auth/.git", "gitdir: irrelevant\n");
    let loaded = f.load("apps/auth").catalog.unwrap();
    assert!(matches!(loaded, ProjectCatalog::Package(_)));
    assert_eq!(loaded.first_package().unwrap().manager.as_str(), "npm");
}

#[test]
fn local_manager_signals_win_and_inheritance_stops_at_root() {
    let f = Fixture::journal();
    f.package(
        "apps/auth",
        r#"{"packageManager":"npm@10","scripts":{"dev":"echo ok"}}"#,
    );
    f.write("apps/mcp/package-lock.json", "{}");
    let w = f.workspace("");
    assert_eq!(
        w.packages[1].catalog.as_ref().unwrap().manager.as_str(),
        "npm"
    );
    assert_eq!(
        w.packages[2].catalog.as_ref().unwrap().manager.as_str(),
        "pnpm"
    );
    assert_eq!(
        w.packages[3].catalog.as_ref().unwrap().manager.as_str(),
        "npm"
    );
    f.write("apps/pnpm-lock.yaml", "lockfileVersion: '9.0'");
    f.write("apps/mcp/pnpm-workspace.yaml", "packages: [sub/*]");
    f.package("apps/mcp/sub/one", r#"{"scripts":{"dev":"echo ok"}}"#);
    std::fs::remove_file(f.path("apps/mcp/package-lock.json")).unwrap();
    assert_eq!(
        f.workspace("apps/mcp").packages[1]
            .catalog
            .as_ref()
            .unwrap()
            .manager
            .as_str(),
        "npm"
    );
}

#[test]
fn invalid_declarations_have_paths_and_never_fall_back() {
    for yaml in [
        "packages: [",
        "packages: wrong",
        "packages: [42]",
        "useNodeVersion: 22",
        "packages: ['[']",
    ] {
        let f = Fixture::journal();
        f.write("pnpm-workspace.yaml", yaml);
        let err = f.load("apps/mcp").catalog.unwrap_err();
        assert!(
            matches!(err, AppError::InvalidWorkspace { .. }),
            "{yaml}: {err}"
        );
        assert!(err.to_string().contains("pnpm-workspace.yaml"));
    }
    let f = Fixture::new();
    f.write(
        "package.json",
        r#"{"workspaces":{"wrong":[]},"scripts":{"dev":"echo ok"}}"#,
    );
    assert!(matches!(
        f.load("").catalog,
        Err(AppError::InvalidWorkspace { .. })
    ));
}

#[test]
fn local_errors_and_rootless_yaml_keep_valid_members() {
    let f = Fixture::journal();
    f.package("apps/auth", "{");
    std::fs::remove_file(f.path("apps/cli/package.json")).unwrap();
    std::fs::create_dir(f.path("apps/cli/package.json")).unwrap();
    let w = f.workspace("");
    assert!(matches!(
        w.packages[1].catalog,
        Err(AppError::InvalidPackageJson)
    ));
    assert!(matches!(
        w.packages[2].catalog,
        Err(AppError::CannotReadPackageJson { .. })
    ));
    assert_eq!(w.packages[3].catalog.as_ref().unwrap().scripts.len(), 2);
    std::fs::remove_file(f.path("package.json")).unwrap();
    assert_eq!(f.workspace("apps/mcp").packages.len(), 5);
}

#[test]
fn symlink_escape_cycles_ignored_and_internal_aliases_deduplicated() {
    use std::os::unix::fs::symlink;
    let f = Fixture::journal();
    let outside = Fixture::new();
    outside.package("", r#"{"scripts":{"dev":"echo outside"}}"#);
    symlink(outside.path(""), f.path("apps/external")).unwrap();
    symlink(f.path("apps"), f.path("apps/loop")).unwrap();
    symlink(f.path("apps/mcp"), f.path("apps/alias")).unwrap();
    f.package("node_modules/hidden", "{}");
    symlink(f.path("node_modules/hidden"), f.path("apps/hidden")).unwrap();
    assert_eq!(f.workspace("").packages.len(), 6);
}

#[test]
fn snapshot_stays_frozen_after_manifests_and_managers_change() {
    let f = Fixture::journal();
    let w = f.workspace("apps/mcp");
    f.package(
        "apps/mcp",
        r#"{"packageManager":"npm","scripts":{"new":"echo changed"}}"#,
    );
    assert_eq!(
        w.packages[3].catalog.as_ref().unwrap().scripts[0].name,
        "dev"
    );
    assert_eq!(
        w.packages[3].catalog.as_ref().unwrap().manager.as_str(),
        "pnpm"
    );
}

#[test]
fn unreadable_member_directory_remains_a_local_error() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::journal();
    let member = f.path("apps/auth");
    let original = std::fs::metadata(&member).unwrap().permissions();
    std::fs::set_permissions(&member, std::fs::Permissions::from_mode(0o000)).unwrap();
    let denied = std::fs::read(member.join("package.json")).is_err();
    let loaded = f.load("");
    std::fs::set_permissions(&member, original).unwrap();
    let ProjectCatalog::Workspace(workspace) = loaded.catalog.unwrap() else {
        panic!("lost workspace")
    };
    assert_eq!(workspace.packages.len(), 6);
    assert_eq!(workspace.packages[1].catalog.is_err(), denied);
    assert_eq!(
        workspace.packages[3]
            .catalog
            .as_ref()
            .unwrap()
            .scripts
            .len(),
        2
    );
}

#[test]
fn dangling_links_do_not_invalidate_the_workspace() {
    let f = Fixture::journal();
    std::os::unix::fs::symlink("missing", f.path("apps/dangling")).unwrap();
    assert_eq!(f.workspace("").packages.len(), 6);
}

#[test]
fn unreadable_non_member_subtree_does_not_block_the_workspace() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::journal();
    // Exercise an actual traversal error even when shallow globs are pruned.
    f.write(
        "pnpm-workspace.yaml",
        "packages: ['apps/*', 'packages/*', 'apps/**/member', 'cache/**/member']",
    );
    for relative in ["apps/auth/src/secret", "cache"] {
        let directory = f.path(relative);
        std::fs::create_dir_all(&directory).unwrap();
        let original = std::fs::metadata(&directory).unwrap().permissions();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o000)).unwrap();
        let loaded = f.load("");
        std::fs::set_permissions(&directory, original).unwrap();
        let ProjectCatalog::Workspace(workspace) = loaded.catalog.unwrap() else {
            panic!("lost workspace with unreadable {relative}")
        };
        assert_eq!(workspace.packages.len(), 6);
        assert!(
            workspace
                .packages
                .iter()
                .all(|package| package.catalog.is_ok())
        );
    }
}

#[test]
fn standalone_lookup_crosses_git_boundary_without_adopting_parent_workspace() {
    let f = Fixture::journal();
    f.write("nested-repo/.git", "gitdir: elsewhere");
    std::fs::create_dir_all(f.path("nested-repo/src")).unwrap();
    let ProjectCatalog::Package(package) = f.load("nested-repo/src").catalog.unwrap() else {
        panic!("a workspace outside the Git root was adopted")
    };
    assert_eq!(package.root, f.path(""));
    assert_eq!(package.display_name, "journal-fixture");
    assert_eq!(package.scripts[0].name, "root");
    f.write("pnpm-workspace.yaml", "packages: [");
    assert!(matches!(
        f.load("nested-repo/src").catalog,
        Ok(ProjectCatalog::Package(_))
    ));
}

#[test]
fn pruned_discovery_preserves_globset_matches() {
    use globset::GlobBuilder;

    let candidates = [
        "apps/deep/two",
        "apps/one",
        "classes/a/c",
        "classes/axc",
        "escaped/*/pkg1",
        "libs/deep/group/pkg1",
        "libs/flat/pkg2",
        "literal/only",
        "literal/only/child",
        "target/build/leaf",
    ];
    for patterns in [
        vec!["apps/*", "literal/only"],
        vec!["apps/*", "libs/**"],
        vec!["apps/**"],
        vec!["libs/{flat,deep/group}/pkg?"],
        vec!["classes/a[!b]c"],
        vec![r"escaped/\*/pkg?"],
        vec!["**/leaf"],
        vec![],
    ] {
        let f = Fixture::new();
        f.write(
            "package.json",
            &serde_json::json!({"workspaces": patterns}).to_string(),
        );
        for path in candidates {
            f.package(path, "{}");
        }
        let matchers: Vec<_> = patterns
            .iter()
            .map(|pattern| {
                GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .build()
                    .unwrap()
                    .compile_matcher()
            })
            .collect();
        let expected: Vec<_> = std::iter::once(".")
            .chain(
                candidates
                    .into_iter()
                    .filter(|path| matchers.iter().any(|m| m.is_match(path))),
            )
            .collect();
        let w = f.workspace("");
        let actual: Vec<_> = w
            .packages
            .iter()
            .map(|p| p.relative_path.to_str().unwrap())
            .collect();
        assert_eq!(actual, expected, "{patterns:?}");
    }
}

#[test]
fn included_alias_can_reach_a_pruned_directory_with_canonical_exclusions() {
    let f = Fixture::new();
    f.write("pnpm-workspace.yaml", "packages: ['apps/*']");
    f.package("", "{}");
    f.package("unlisted/package", "{}");
    std::fs::create_dir(f.path("apps")).unwrap();
    std::os::unix::fs::symlink(f.path("unlisted/package"), f.path("apps/alias")).unwrap();
    let w = f.workspace("");
    assert_eq!(w.packages.len(), 2);
    assert_eq!(w.packages[1].root, f.path("unlisted/package"));
    f.write(
        "pnpm-workspace.yaml",
        "packages: ['apps/*', '!unlisted/package']",
    );
    assert_eq!(f.workspace("").packages.len(), 1);
}
