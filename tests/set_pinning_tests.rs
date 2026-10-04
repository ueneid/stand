//! `stand set --encrypt` warns when no local identity matches the project key.

use assert_cmd::Command;
use std::fs;

fn run(with_legacy_keys: bool) -> String {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("project");
    let xdg = tmp.path().join("xdg");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(&xdg).unwrap();
    let kp = stand::crypto::generate_key_pair();
    fs::write(
        dir.join(".stand.toml"),
        format!(
            "version = \"1.0\"\n\n[encryption]\npublic_key = \"{}\"\n\n[environments.dev]\ndescription = \"Dev\"\n",
            kp.public_key
        ),
    )
    .unwrap();
    if with_legacy_keys {
        stand::crypto::keys::save_private_key(&dir.join(".stand.keys"), &kp.private_key).unwrap();
    }
    let out = Command::cargo_bin("stand")
        .unwrap()
        .current_dir(&dir)
        .env_clear()
        .env("HOME", tmp.path())
        .env("XDG_CONFIG_HOME", &xdg)
        .args(["set", "-e", "dev", "TOKEN", "value"])
        .assert()
        .success()
        .get_output()
        .clone();
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[test]
fn test_set_encrypt_warns_when_no_local_identity_matches() {
    let stderr = run(false);
    assert!(
        stderr.contains("does not match any local identity"),
        "{stderr}"
    );
    assert!(stderr.contains("[encryption].public_key"), "{stderr}");
}

#[test]
fn test_set_encrypt_does_not_warn_with_legacy_key_file() {
    let stderr = run(true);
    assert!(!stderr.contains("local identity"), "{stderr}");
}
