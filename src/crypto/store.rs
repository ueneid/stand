//! On-disk identity store for Stand.
//!
//! The project X25519 identity is kept outside the project directory, wrapped
//! (encrypted) so that it can only be unlocked by a human:
//!
//! - `<P>.age`         plaintext identity file encrypted with an age passphrase (scrypt)
//! - `<P>.se.age`      the same plaintext encrypted to a Secure Enclave recipient
//! - `<P>.se-identity` the Secure Enclave identity (hardware-bound handle)
//!
//! `<P>` is the project public key (`age1...`). Files are written atomically
//! with mode 0600 inside a 0700 directory.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use age::secrecy::{ExposeSecret, SecretString};
use age::x25519;

use super::CryptoError;

/// Current UTC time as an RFC 3339 string (used in the identity file header).
fn created_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Builds the plaintext age identity file for an identity.
///
/// The result is compatible with `age -d -i` once decrypted.
pub fn identity_file_contents(identity: &x25519::Identity) -> SecretString {
    SecretString::from(format!(
        "# created: {}\n# public key: {}\n{}\n",
        created_timestamp(),
        identity.to_public(),
        identity.to_string().expose_secret()
    ))
}

/// Extracts the X25519 identity from decrypted identity file contents.
pub fn parse_identity_file_contents(contents: &[u8]) -> Result<x25519::Identity, CryptoError> {
    let text = std::str::from_utf8(contents)
        .map_err(|e| CryptoError::InvalidPrivateKey(format!("identity is not UTF-8: {}", e)))?;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("AGE-SECRET-KEY-1") {
            return line
                .parse::<x25519::Identity>()
                .map_err(|e| CryptoError::InvalidPrivateKey(e.to_string()));
        }
    }
    Err(CryptoError::InvalidPrivateKey(
        "no AGE-SECRET-KEY found in identity file".to_string(),
    ))
}

/// Returns the Stand config directory.
///
/// `$XDG_CONFIG_HOME/stand` when `XDG_CONFIG_HOME` is set, non-empty and
/// absolute; otherwise `$HOME/.config/stand`.
pub fn config_dir() -> Result<PathBuf, CryptoError> {
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    let home = std::env::var_os("HOME");
    config_dir_from(xdg.as_deref(), home.as_deref())
}

/// Pure variant of [`config_dir`] for explicit inputs.
pub fn config_dir_from(
    xdg_config_home: Option<&std::ffi::OsStr>,
    home: Option<&std::ffi::OsStr>,
) -> Result<PathBuf, CryptoError> {
    if let Some(xdg) = xdg_config_home {
        let path = PathBuf::from(xdg);
        if !xdg.is_empty() && path.is_absolute() {
            return Ok(path.join("stand"));
        }
    }
    match home {
        Some(h) if !h.is_empty() => Ok(PathBuf::from(h).join(".config").join("stand")),
        _ => Err(CryptoError::StoreError(
            "cannot determine config directory (set XDG_CONFIG_HOME or HOME)".to_string(),
        )),
    }
}

/// Identities directory inside a config directory.
pub fn identities_dir_in(config_dir: &Path) -> PathBuf {
    config_dir.join("identities")
}

/// Identities directory for the current process environment.
pub fn identities_dir() -> Result<PathBuf, CryptoError> {
    Ok(identities_dir_in(&config_dir()?))
}

/// Path of the passphrase-wrapped identity `<P>.age`.
pub fn identity_path_in(config_dir: &Path, public_key: &str) -> PathBuf {
    identities_dir_in(config_dir).join(format!("{}.age", public_key))
}

/// Path of the Secure Enclave-wrapped identity `<P>.se.age`.
pub fn se_wrapped_path_in(config_dir: &Path, public_key: &str) -> PathBuf {
    identities_dir_in(config_dir).join(format!("{}.se.age", public_key))
}

/// Path of the Secure Enclave identity `<P>.se-identity`.
pub fn se_identity_path_in(config_dir: &Path, public_key: &str) -> PathBuf {
    identities_dir_in(config_dir).join(format!("{}.se-identity", public_key))
}

/// [`identity_path_in`] for the current process environment.
pub fn identity_path(public_key: &str) -> Result<PathBuf, CryptoError> {
    Ok(identity_path_in(&config_dir()?, public_key))
}

/// [`se_wrapped_path_in`] for the current process environment.
pub fn se_wrapped_path(public_key: &str) -> Result<PathBuf, CryptoError> {
    Ok(se_wrapped_path_in(&config_dir()?, public_key))
}

/// [`se_identity_path_in`] for the current process environment.
pub fn se_identity_path(public_key: &str) -> Result<PathBuf, CryptoError> {
    Ok(se_identity_path_in(&config_dir()?, public_key))
}

/// Rejects public keys that could escape the identities directory.
pub(crate) fn validate_public_key(public_key: &str) -> Result<(), CryptoError> {
    let ok = !public_key.is_empty()
        && public_key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(CryptoError::InvalidPublicKey(
            "public key contains unexpected characters".to_string(),
        ))
    }
}

/// Creates the identities directory (mode 0700) and returns its path.
pub fn ensure_identities_dir_in(config_dir: &Path) -> Result<PathBuf, CryptoError> {
    let dir = identities_dir_in(config_dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)?;
        // Tighten the directories even if they pre-existed.
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        fs::set_permissions(config_dir, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Writes `data` to `path` atomically with mode 0600 (temp file + rename).
pub fn write_private_file(path: &Path, data: &[u8]) -> Result<(), CryptoError> {
    let parent = path
        .parent()
        .ok_or_else(|| CryptoError::StoreError("path has no parent directory".to_string()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| CryptoError::StoreError("path has no file name".to_string()))?
        .to_string_lossy();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let tmp = parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name,
        std::process::id(),
        nanos
    ));

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| -> std::io::Result<()> {
        let mut file = options.open(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp);
        return Err(CryptoError::IoError(e));
    }
    Ok(())
}

/// Encrypts `plaintext` with an age passphrase (binary age format).
///
/// `work_factor` of `None` uses age's calibrated default (about 1 second).
pub fn wrap_with_passphrase(
    plaintext: &[u8],
    passphrase: &SecretString,
    work_factor: Option<u8>,
) -> Result<Vec<u8>, CryptoError> {
    let mut recipient = age::scrypt::Recipient::new(passphrase.clone());
    if let Some(wf) = work_factor {
        recipient.set_work_factor(wf);
    }
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    let mut out = vec![];
    let mut writer = encryptor
        .wrap_output(&mut out)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    writer
        .write_all(plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    writer
        .finish()
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    Ok(out)
}

/// Outcome of a failed passphrase unwrap.
#[derive(Debug)]
pub enum UnwrapError {
    /// The passphrase did not match (the caller may retry).
    WrongPassphrase,
    /// The file is unusable (corrupt, work factor too high, ...).
    Other(String),
}

/// Decrypts a passphrase-wrapped age file in memory.
pub fn unwrap_with_passphrase(
    ciphertext: &[u8],
    passphrase: &SecretString,
    max_work_factor: Option<u8>,
) -> Result<Vec<u8>, UnwrapError> {
    let decryptor =
        age::Decryptor::new(ciphertext).map_err(|e| UnwrapError::Other(e.to_string()))?;
    let mut identity = age::scrypt::Identity::new(passphrase.clone());
    if let Some(max) = max_work_factor {
        identity.set_max_work_factor(max);
    }
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|e| match e {
            age::DecryptError::DecryptionFailed | age::DecryptError::NoMatchingKeys => {
                UnwrapError::WrongPassphrase
            }
            other => UnwrapError::Other(other.to_string()),
        })?;
    let mut out = vec![];
    reader
        .read_to_end(&mut out)
        .map_err(|e| UnwrapError::Other(e.to_string()))?;
    Ok(out)
}

/// Wraps `identity` with `passphrase` and writes `<P>.age` into the store at `config_dir`.
///
/// Overwrites an existing `<P>.age`. Returns the written path.
pub fn write_passphrase_identity_in(
    config_dir: &Path,
    public_key: &str,
    identity: &x25519::Identity,
    passphrase: &SecretString,
    work_factor: Option<u8>,
) -> Result<PathBuf, CryptoError> {
    validate_public_key(public_key)?;
    ensure_identities_dir_in(config_dir)?;
    let plaintext = identity_file_contents(identity);
    let wrapped = wrap_with_passphrase(
        plaintext.expose_secret().as_bytes(),
        passphrase,
        work_factor,
    )?;
    let path = identity_path_in(config_dir, public_key);
    write_private_file(&path, &wrapped)?;
    Ok(path)
}

/// [`write_passphrase_identity_in`] for the current process environment.
pub fn write_passphrase_identity(
    public_key: &str,
    identity: &x25519::Identity,
    passphrase: &SecretString,
    work_factor: Option<u8>,
) -> Result<PathBuf, CryptoError> {
    write_passphrase_identity_in(
        &config_dir()?,
        public_key,
        identity,
        passphrase,
        work_factor,
    )
}

/// Removes all identity files for `public_key`; returns the paths that were removed.
pub fn remove_identity_files_in(
    config_dir: &Path,
    public_key: &str,
) -> Result<Vec<PathBuf>, CryptoError> {
    validate_public_key(public_key)?;
    let mut removed = vec![];
    for path in [
        identity_path_in(config_dir, public_key),
        se_wrapped_path_in(config_dir, public_key),
        se_identity_path_in(config_dir, public_key),
    ] {
        if path.exists() {
            fs::remove_file(&path)?;
            removed.push(path);
        }
    }
    Ok(removed)
}

/// [`remove_identity_files_in`] for the current process environment.
pub fn remove_identity_files(public_key: &str) -> Result<Vec<PathBuf>, CryptoError> {
    remove_identity_files_in(&config_dir()?, public_key)
}

/// Checks that `path` is safe to overwrite in place: a regular file (not a
/// symlink) with a single hard link. Returns its `symlink_metadata`.
///
/// Overwriting through a symlink or a second hard link would destroy a file
/// that is not the legacy key (the project directory is agent-writable).
pub fn check_plain_single_link_file(path: &Path) -> Result<fs::Metadata, CryptoError> {
    let meta = fs::symlink_metadata(path)?;
    let shown = super::ui::sanitize_for_terminal(&path.display().to_string());
    if meta.file_type().is_symlink() || !meta.file_type().is_file() {
        return Err(CryptoError::StoreError(format!(
            "{} is not a regular file (symlink or special file); refusing to overwrite it",
            shown
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 {
            return Err(CryptoError::StoreError(format!(
                "{} has {} hard links; refusing to overwrite it",
                shown,
                meta.nlink()
            )));
        }
    }
    Ok(meta)
}

/// Overwrites a file with zeros, fsyncs it and deletes it.
///
/// Refuses symlinks and files with other hard links (see
/// [`check_plain_single_link_file`]). The opened file is compared with the
/// checked metadata so a swap between check and open is detected.
pub fn wipe_and_delete(path: &Path) -> Result<(), CryptoError> {
    let checked = check_plain_single_link_file(path)?;
    {
        let mut file = fs::OpenOptions::new().write(true).open(path)?;
        let opened = file.metadata()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if opened.dev() != checked.dev() || opened.ino() != checked.ino() || opened.nlink() != 1
            {
                return Err(CryptoError::StoreError(
                    "file changed while it was being opened; refusing to overwrite it".to_string(),
                ));
            }
        }
        let zeros = [0u8; 4096];
        let mut remaining = opened.len();
        while remaining > 0 {
            let n = remaining.min(zeros.len() as u64) as usize;
            file.write_all(&zeros[..n])?;
            remaining -= n as u64;
        }
        file.sync_all()?;
    }
    fs::remove_file(path)?;
    Ok(())
}

/// Removes a legacy key file. A plain single-link file is zero-overwritten
/// first; anything else (symlink, extra hard link) only has its own directory
/// entry unlinked, leaving any other file untouched. Returns whether the
/// content was wiped.
pub fn remove_legacy_key_file(path: &Path) -> Result<bool, CryptoError> {
    match check_plain_single_link_file(path) {
        Ok(_) => wipe_and_delete(path).map(|_| true),
        Err(CryptoError::StoreError(_)) => {
            fs::remove_file(path)?;
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::OsStr;
    use tempfile::tempdir;

    const TEST_WF: u8 = 8;

    fn pass(s: &str) -> SecretString {
        SecretString::from(s.to_string())
    }

    #[test]
    fn test_config_dir_prefers_absolute_xdg() {
        let dir = config_dir_from(Some(OsStr::new("/x/cfg")), Some(OsStr::new("/h"))).unwrap();
        assert_eq!(dir, PathBuf::from("/x/cfg/stand"));
    }

    #[test]
    fn test_config_dir_ignores_empty_or_relative_xdg() {
        let expected = PathBuf::from("/h/.config/stand");
        assert_eq!(
            config_dir_from(Some(OsStr::new("")), Some(OsStr::new("/h"))).unwrap(),
            expected
        );
        assert_eq!(
            config_dir_from(Some(OsStr::new("rel/cfg")), Some(OsStr::new("/h"))).unwrap(),
            expected
        );
        assert_eq!(
            config_dir_from(None, Some(OsStr::new("/h"))).unwrap(),
            expected
        );
    }

    #[test]
    fn test_config_dir_fails_without_home() {
        assert!(config_dir_from(None, None).is_err());
    }

    #[test]
    #[serial]
    fn test_config_dir_reads_process_env() {
        let tmp = tempdir().unwrap();
        let old = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let got = config_dir().unwrap();
        match old {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        assert_eq!(got, tmp.path().join("stand"));
    }

    #[test]
    fn test_paths_use_public_key_names() {
        let c = Path::new("/c");
        assert_eq!(
            identity_path_in(c, "age1abc"),
            Path::new("/c/identities/age1abc.age")
        );
        assert_eq!(
            se_wrapped_path_in(c, "age1abc"),
            Path::new("/c/identities/age1abc.se.age")
        );
        assert_eq!(
            se_identity_path_in(c, "age1abc"),
            Path::new("/c/identities/age1abc.se-identity")
        );
    }

    #[test]
    fn test_write_passphrase_identity_roundtrip_and_format() {
        let tmp = tempdir().unwrap();
        let id = x25519::Identity::generate();
        let p = id.to_public().to_string();
        let path =
            write_passphrase_identity_in(tmp.path(), &p, &id, &pass("pw"), Some(TEST_WF)).unwrap();
        assert_eq!(path, identity_path_in(tmp.path(), &p));

        let bytes = fs::read(&path).unwrap();
        let plain = unwrap_with_passphrase(&bytes, &pass("pw"), Some(TEST_WF)).unwrap();
        let text = String::from_utf8(plain.clone()).unwrap();
        assert!(text.starts_with("# created: "));
        assert!(text.contains(&format!("# public key: {}\n", p)));
        let parsed = parse_identity_file_contents(&plain).unwrap();
        assert_eq!(parsed.to_public().to_string(), p);
    }

    #[test]
    fn test_unwrap_with_wrong_passphrase_reports_wrong_passphrase() {
        let wrapped = wrap_with_passphrase(b"data", &pass("right"), Some(TEST_WF)).unwrap();
        let err = unwrap_with_passphrase(&wrapped, &pass("wrong"), Some(TEST_WF)).unwrap_err();
        assert!(matches!(err, UnwrapError::WrongPassphrase));
    }

    #[cfg(unix)]
    #[test]
    fn test_store_permissions_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempdir().unwrap();
        let id = x25519::Identity::generate();
        let p = id.to_public().to_string();
        let path =
            write_passphrase_identity_in(tmp.path(), &p, &id, &pass("pw"), Some(TEST_WF)).unwrap();
        let file_mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        let dir_mode = fs::metadata(identities_dir_in(tmp.path()))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(file_mode, 0o600);
        assert_eq!(dir_mode, 0o700);
    }

    #[test]
    fn test_write_leaves_no_temp_files() {
        let tmp = tempdir().unwrap();
        let id = x25519::Identity::generate();
        let p = id.to_public().to_string();
        write_passphrase_identity_in(tmp.path(), &p, &id, &pass("pw"), Some(TEST_WF)).unwrap();
        let names: Vec<_> = fs::read_dir(identities_dir_in(tmp.path()))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec![format!("{}.age", p)]);
    }

    #[test]
    fn test_rejects_path_traversal_in_public_key() {
        let tmp = tempdir().unwrap();
        let id = x25519::Identity::generate();
        let r =
            write_passphrase_identity_in(tmp.path(), "../evil", &id, &pass("pw"), Some(TEST_WF));
        assert!(matches!(r, Err(CryptoError::InvalidPublicKey(_))));
    }

    #[test]
    fn test_remove_identity_files_returns_removed_paths() {
        let tmp = tempdir().unwrap();
        let id = x25519::Identity::generate();
        let p = id.to_public().to_string();
        write_passphrase_identity_in(tmp.path(), &p, &id, &pass("pw"), Some(TEST_WF)).unwrap();
        fs::write(se_identity_path_in(tmp.path(), &p), "x").unwrap();

        let removed = remove_identity_files_in(tmp.path(), &p).unwrap();
        assert_eq!(removed.len(), 2);
        assert!(!identity_path_in(tmp.path(), &p).exists());
        assert!(remove_identity_files_in(tmp.path(), &p).unwrap().is_empty());
    }

    #[test]
    fn test_wipe_and_delete_removes_file() {
        let tmp = tempdir().unwrap();
        let f = tmp.path().join("secret");
        fs::write(&f, vec![7u8; 10_000]).unwrap();
        wipe_and_delete(&f).unwrap();
        assert!(!f.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_wipe_and_delete_refuses_symlink_and_keeps_target() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("target");
        fs::write(&target, b"important").unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(wipe_and_delete(&link).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"important");
        assert!(link.symlink_metadata().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn test_wipe_and_delete_refuses_extra_hard_link() {
        let tmp = tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        fs::write(&a, b"important").unwrap();
        fs::hard_link(&a, &b).unwrap();
        assert!(wipe_and_delete(&a).is_err());
        assert_eq!(fs::read(&a).unwrap(), b"important");
        assert_eq!(fs::read(&b).unwrap(), b"important");
    }

    #[cfg(unix)]
    #[test]
    fn test_remove_legacy_key_file_unlinks_symlink_only() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("target");
        fs::write(&target, b"important").unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(!remove_legacy_key_file(&link).unwrap());
        assert!(link.symlink_metadata().is_err());
        assert_eq!(fs::read(&target).unwrap(), b"important");

        let plain = tmp.path().join("plain");
        fs::write(&plain, b"k").unwrap();
        assert!(remove_legacy_key_file(&plain).unwrap());
        assert!(!plain.exists());
    }

    #[test]
    fn test_write_private_file_uses_create_new_temp_and_replaces_target() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("f");
        write_private_file(&path, b"one").unwrap();
        write_private_file(&path, b"two").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        #[cfg(unix)]
        {
            // A pre-planted symlink at the target is replaced, not followed.
            let victim = tmp.path().join("victim");
            fs::write(&victim, b"victim").unwrap();
            let linked = tmp.path().join("linked");
            std::os::unix::fs::symlink(&victim, &linked).unwrap();
            write_private_file(&linked, b"new").unwrap();
            assert_eq!(fs::read(&victim).unwrap(), b"victim");
            assert_eq!(fs::read(&linked).unwrap(), b"new");
        }
    }

    #[test]
    fn test_parse_identity_file_requires_secret_key() {
        assert!(parse_identity_file_contents(b"# nothing\n").is_err());
    }
}
