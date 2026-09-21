use std::fs;
use std::path::Path;

fn source_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            files.extend(source_files(&path));
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files
}

fn assert_no_forbidden(root: &Path, forbidden: &[&str]) {
    for path in source_files(root) {
        let source = fs::read_to_string(&path).unwrap();
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "{} contains forbidden import or I/O `{needle}`",
                path.display()
            );
        }
    }
}

#[test]
fn domain_does_not_depend_on_adapters_or_io() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain");
    assert_no_forbidden(
        &root,
        &[
            "crate::adapters",
            "std::fs",
            "std::net",
            "std::os::unix::net",
            "std::process",
            "crossterm",
            "ratatui",
        ],
    );
}

#[test]
fn application_does_not_depend_on_adapters_or_io() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/application");
    assert_no_forbidden(
        &root,
        &[
            "crate::adapters",
            "std::fs",
            "std::net",
            "std::os::unix::net",
            "std::process",
            "crossterm",
            "ratatui",
        ],
    );
}

#[test]
fn domain_does_not_import_application() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/domain");
    assert_no_forbidden(&root, &["crate::application"]);
}
