use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn devcontainer_config_pins_node_24_and_owned_caches() {
    let config = fs::read_to_string(repo_root().join(".devcontainer/devcontainer.json")).unwrap();

    assert!(config.contains(r#""ghcr.io/devcontainers/features/node:1""#));
    assert!(config.contains(r#""version": "24""#));
    assert!(config.contains(r"source=fqc-cargo-registry,target=/home/vscode/.cargo/registry,type=volume"));
    assert!(config.contains(r"source=fqc-cargo-git,target=/home/vscode/.cargo/git,type=volume"));
    assert!(config.contains(r"source=fqc-target-cache,target=${containerWorkspaceFolder}/target,type=volume"));
    assert!(config.contains(r"source=fqc-npm-cache,target=/home/vscode/.npm,type=volume"));
    assert!(config.contains(
        r#""postCreateCommand": "bash \"${containerWorkspaceFolder}/.devcontainer/scripts/container-setup.sh\" create""#
    ));
    assert!(config.contains(
        r#""postStartCommand": "bash \"${containerWorkspaceFolder}/.devcontainer/scripts/container-setup.sh\" start""#
    ));
}

#[test]
fn devcontainer_dockerfile_installs_only_minimum_helper_surface() {
    let dockerfile = fs::read_to_string(repo_root().join(".devcontainer/Dockerfile")).unwrap();

    assert!(dockerfile.contains("taplo-cli --version 0.10.0"));
    assert!(!dockerfile.contains("bacon"));
    assert!(!dockerfile.contains("cargo-deny"));
}

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, body).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(unix)]
fn make_dir(path: &Path) {
    fs::create_dir(path).unwrap();
}

#[cfg(unix)]
fn base_command(
    workspace: &Path,
    fake_bin: &Path,
    fake_home: &Path,
    include_system_path: bool,
) -> std::process::Command {
    let mut command = std::process::Command::new("/bin/bash");
    command
        .arg(repo_root().join(".devcontainer/scripts/container-setup.sh"))
        .arg("create")
        .current_dir(workspace)
        .env("WORKSPACE", workspace)
        .env("HOME", fake_home)
        .env("PATH", {
            if include_system_path {
                format!("{}:{}", fake_bin.display(), std::env::var("PATH").unwrap())
            } else {
                fake_bin.display().to_string()
            }
        });
    command
}

#[cfg(unix)]
#[test]
fn container_setup_create_surfaces_hook_failures() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().join("workspace");
    let workspace_scripts = workspace.join("scripts");
    let fake_bin = temp_dir.path().join("fake-bin");
    let fake_home = temp_dir.path().join("home");

    make_dir(&workspace);
    make_dir(&workspace_scripts);
    make_dir(&fake_bin);
    make_dir(&fake_home);

    write_executable(
        &workspace_scripts.join("setup-hooks.sh"),
        "#!/usr/bin/env bash\nset -euo pipefail\necho 'hook setup failed' >&2\nexit 23\n",
    );
    write_executable(&fake_bin.join("git"), "#!/usr/bin/env bash\nexit 0\n");
    write_executable(&fake_bin.join("cargo"), "#!/usr/bin/env bash\nexit 0\n");

    let output = base_command(&workspace, &fake_bin, &fake_home, true).output().unwrap();

    assert_eq!(output.status.code(), Some(23));
    assert!(String::from_utf8_lossy(&output.stderr).contains("hook setup failed"));
}

#[cfg(unix)]
#[test]
fn container_setup_defaults_workspace_from_script_location() {
    let temp_dir = tempfile::tempdir().unwrap();
    let fake_bin = temp_dir.path().join("fake-bin");
    let fake_home = temp_dir.path().join("home");
    let unrelated_cwd = temp_dir.path().join("elsewhere");

    make_dir(&fake_bin);
    make_dir(&fake_home);
    make_dir(&unrelated_cwd);

    write_executable(&fake_bin.join("git"), "#!/usr/bin/env bash\nexit 0\n");
    write_executable(&fake_bin.join("grep"), "#!/usr/bin/env bash\nexit 1\n");

    let output = std::process::Command::new("/bin/bash")
        .arg(repo_root().join(".devcontainer/scripts/container-setup.sh"))
        .arg("start")
        .current_dir(&unrelated_cwd)
        .env("HOME", &fake_home)
        .env(
            "PATH",
            format!("{}:{}", fake_bin.display(), std::env::var("PATH").unwrap()),
        )
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[cfg(unix)]
#[test]
fn container_setup_create_requires_hook_script() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().join("workspace");
    let workspace_scripts = workspace.join("scripts");
    let fake_bin = temp_dir.path().join("fake-bin");
    let fake_home = temp_dir.path().join("home");

    make_dir(&workspace);
    make_dir(&workspace_scripts);
    make_dir(&fake_bin);
    make_dir(&fake_home);

    write_executable(&fake_bin.join("git"), "#!/usr/bin/env bash\nexit 0\n");
    write_executable(&fake_bin.join("cargo"), "#!/usr/bin/env bash\nexit 0\n");
    write_executable(&fake_bin.join("npm"), "#!/usr/bin/env bash\nexit 0\n");

    let output = base_command(&workspace, &fake_bin, &fake_home, true).output().unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Missing or non-executable hook setup script"));
}

#[cfg(unix)]
#[test]
fn container_setup_create_surfaces_missing_npm() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().join("workspace");
    let workspace_scripts = workspace.join("scripts");
    let fake_bin = temp_dir.path().join("fake-bin");
    let fake_home = temp_dir.path().join("home");

    make_dir(&workspace);
    make_dir(&workspace_scripts);
    make_dir(&fake_bin);
    make_dir(&fake_home);

    write_executable(&workspace_scripts.join("setup-hooks.sh"), "#!/bin/bash\nexit 0\n");
    fs::write(workspace.join("package.json"), "{\n  \"name\": \"fqc-test\"\n}\n").unwrap();
    write_executable(&fake_bin.join("git"), "#!/bin/bash\nexit 0\n");
    write_executable(
        &fake_bin.join("npm"),
        "#!/bin/bash\necho 'npm: command not found' >&2\nexit 127\n",
    );

    let output = base_command(&workspace, &fake_bin, &fake_home, true).output().unwrap();

    assert_eq!(output.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&output.stderr).contains("npm: command not found"));
}

#[cfg(unix)]
#[test]
fn container_setup_create_surfaces_missing_cargo() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().join("workspace");
    let workspace_scripts = workspace.join("scripts");
    let fake_bin = temp_dir.path().join("fake-bin");
    let fake_home = temp_dir.path().join("home");

    make_dir(&workspace);
    make_dir(&workspace_scripts);
    make_dir(&fake_bin);
    make_dir(&fake_home);

    write_executable(&workspace_scripts.join("setup-hooks.sh"), "#!/bin/bash\nexit 0\n");
    fs::write(
        workspace.join("Cargo.toml"),
        "[package]\nname = \"fqc-test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    write_executable(&fake_bin.join("git"), "#!/bin/bash\nexit 0\n");
    write_executable(
        &fake_bin.join("cargo"),
        "#!/bin/bash\necho 'cargo: command not found' >&2\nexit 127\n",
    );

    let output = base_command(&workspace, &fake_bin, &fake_home, true).output().unwrap();

    assert_eq!(output.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cargo: command not found"));
}
