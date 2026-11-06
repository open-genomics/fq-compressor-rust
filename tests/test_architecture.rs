//! Dependency-direction guards for the module boundaries.
//!
//! Rust allows module cycles, so the four-gate checks alone cannot catch a new
//! `archive -> algo` import. These tests make the intended direction explicit
//! without adding a CI workflow or an external dependency.

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files
}

fn assert_no_imports(dir: &str, forbidden: &[&str]) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(dir);
    let root = if base.is_dir() { base } else { base.with_extension("rs") };
    let files = if root.is_dir() { rust_files(&root) } else { vec![root] };
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "{} must not depend on `{needle}`; found in {}",
                dir,
                file.display()
            );
        }
    }
}

#[test]
fn archive_and_algo_only_meet_through_block() {
    assert_no_imports("archive", &["crate::algo"]);
    assert_no_imports("algo", &["crate::archive"]);
    assert_no_imports("block", &["crate::algo", "crate::archive"]);
}

#[test]
fn facade_and_verify_do_not_depend_on_command_layer() {
    assert_no_imports("api", &["crate::commands"]);
    assert_no_imports("verify", &["crate::commands"]);
}

#[test]
fn low_level_io_and_parser_do_not_depend_on_higher_layers() {
    let higher = &[
        "crate::archive",
        "crate::algo",
        "crate::pipeline",
        "crate::engine",
        "crate::commands",
    ];
    assert_no_imports("io", higher);
    assert_no_imports("fastq", higher);
    assert_no_imports("memory", higher);
    assert_no_imports("plan", higher);
}
