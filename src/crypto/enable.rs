//! Helpers used when enabling or migrating encryption: creating the wrapped
//! identity files, adding Touch ID and verifying the passphrase wrap.

use std::path::{Path, PathBuf};

use age::secrecy::{ExposeSecret, SecretString};
use age::x25519;

use super::store::{self, UnwrapError};
use super::touch_id::SecureEnclaveBackend;
use super::ui::UnlockUi;
use super::CryptoError;

/// Asks for a new passphrase twice. Mismatch, empty input or cancel is an error.
pub fn prompt_new_passphrase(ui: &dyn UnlockUi) -> Result<SecretString, CryptoError> {
    let first = ui
        .request_passphrase("Enter a new passphrase for the stand key:")
        .ok_or_else(|| CryptoError::UnlockFailed("passphrase entry cancelled".to_string()))?;
    if first.expose_secret().is_empty() {
        return Err(CryptoError::UnlockFailed(
            "passphrase must not be empty".to_string(),
        ));
    }
    let second = ui
        .request_passphrase("Confirm the passphrase:")
        .ok_or_else(|| CryptoError::UnlockFailed("passphrase entry cancelled".to_string()))?;
    if first.expose_secret() != second.expose_secret() {
        return Err(CryptoError::UnlockFailed(
            "passphrases do not match".to_string(),
        ));
    }
    Ok(first)
}

/// Generates a new project identity, asks for a passphrase twice and writes `<P>.age`.
///
/// The identity is bound to `project_dir` (see [`store::identity_file_contents`]).
///
/// Returns the identity and its public key `P`. `work_factor` of `None` uses
/// the calibrated default.
pub fn create_passphrase_identity_in(
    config_dir: &Path,
    ui: &dyn UnlockUi,
    work_factor: Option<u8>,
    project_dir: &Path,
) -> Result<(x25519::Identity, String), CryptoError> {
    let passphrase = prompt_new_passphrase(ui)?;
    let identity = x25519::Identity::generate();
    let public_key = identity.to_public().to_string();
    store::write_passphrase_identity_in(
        config_dir,
        &public_key,
        &identity,
        &passphrase,
        work_factor,
        project_dir,
    )?;
    Ok((identity, public_key))
}

/// [`create_passphrase_identity_in`] for the current process environment.
pub fn create_passphrase_identity(
    ui: &dyn UnlockUi,
    work_factor: Option<u8>,
    project_dir: &Path,
) -> Result<(x25519::Identity, String), CryptoError> {
    create_passphrase_identity_in(&store::config_dir()?, ui, work_factor, project_dir)
}

/// Creates a Secure Enclave key and writes `<P>.se-identity` and `<P>.se.age`.
///
/// Errors if `<P>.se.age` already exists. Returns `(se_identity_path, se_wrapped_path)`.
pub fn add_touch_id_in(
    config_dir: &Path,
    backend: &dyn SecureEnclaveBackend,
    identity: &x25519::Identity,
    public_key: &str,
    project_dir: &Path,
) -> Result<(PathBuf, PathBuf), CryptoError> {
    let plaintext = store::identity_file_contents(identity, project_dir)?;
    add_touch_id_with_plaintext_in(
        config_dir,
        backend,
        plaintext.expose_secret().as_bytes(),
        public_key,
    )
}

/// Like [`add_touch_id_in`] but wraps the given identity file bytes verbatim.
///
/// Used when re-wrapping an existing identity: the original plaintext (with its
/// recorded `# project:` line) is kept, never regenerated for the current project.
pub fn add_touch_id_with_plaintext_in(
    config_dir: &Path,
    backend: &dyn SecureEnclaveBackend,
    plaintext: &[u8],
    public_key: &str,
) -> Result<(PathBuf, PathBuf), CryptoError> {
    store::validate_public_key(public_key)?;
    let wrapped_path = store::se_wrapped_path_in(config_dir, public_key);
    if wrapped_path.exists() {
        return Err(CryptoError::StoreError(format!(
            "Touch ID is already set up ({} exists)",
            wrapped_path.display()
        )));
    }
    store::ensure_identities_dir_in(config_dir)?;

    let material = backend.keygen()?;
    let wrapped = backend.wrap(&material.recipient, plaintext)?;

    let identity_path = store::se_identity_path_in(config_dir, public_key);
    store::write_private_file(&identity_path, material.identity_file.as_bytes())?;
    store::write_private_file(&wrapped_path, &wrapped)?;
    Ok((identity_path, wrapped_path))
}

/// [`add_touch_id_in`] for the current process environment.
pub fn add_touch_id(
    backend: &dyn SecureEnclaveBackend,
    identity: &x25519::Identity,
    public_key: &str,
    project_dir: &Path,
) -> Result<(PathBuf, PathBuf), CryptoError> {
    add_touch_id_in(
        &store::config_dir()?,
        backend,
        identity,
        public_key,
        project_dir,
    )
}

/// Unwraps `<P>.age` in memory with `passphrase` and checks it yields `public_key`.
///
/// Use this right after writing a wrap, before deleting any plaintext key.
pub fn verify_passphrase_wrap_in(
    config_dir: &Path,
    public_key: &str,
    passphrase: &SecretString,
    max_work_factor: Option<u8>,
) -> Result<(), CryptoError> {
    store::validate_public_key(public_key)?;
    let bytes = std::fs::read(store::identity_path_in(config_dir, public_key))?;
    let plain = store::unwrap_with_passphrase(&bytes, passphrase, max_work_factor).map_err(
        |e| match e {
            UnwrapError::WrongPassphrase => {
                CryptoError::UnlockFailed("passphrase does not unlock the stored key".to_string())
            }
            UnwrapError::Other(m) => CryptoError::UnlockFailed(m),
        },
    )?;
    let identity = store::parse_identity_file_contents(&plain)?;
    if identity.to_public().to_string() != public_key {
        return Err(CryptoError::IdentityMismatch {
            expected: public_key.to_string(),
        });
    }
    Ok(())
}

/// [`verify_passphrase_wrap_in`] for the current process environment.
pub fn verify_passphrase_wrap(
    public_key: &str,
    passphrase: &SecretString,
    max_work_factor: Option<u8>,
) -> Result<(), CryptoError> {
    verify_passphrase_wrap_in(
        &store::config_dir()?,
        public_key,
        passphrase,
        max_work_factor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::testing::{MockSe, MockUi};
    use tempfile::tempdir;

    const WF: u8 = 8;

    #[test]
    fn test_create_passphrase_identity_writes_wrap_that_verifies() {
        let tmp = tempdir().unwrap();
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        let (identity, p) =
            create_passphrase_identity_in(tmp.path(), ui.as_ref(), Some(WF), tmp.path()).unwrap();
        assert_eq!(identity.to_public().to_string(), p);
        assert!(store::identity_path_in(tmp.path(), &p).exists());
        verify_passphrase_wrap_in(
            tmp.path(),
            &p,
            &SecretString::from("pw".to_string()),
            Some(WF),
        )
        .unwrap();
    }

    #[test]
    fn test_create_passphrase_identity_rejects_mismatch() {
        let tmp = tempdir().unwrap();
        let ui = MockUi::new(vec![Some("a"), Some("b")]);
        let r = create_passphrase_identity_in(tmp.path(), ui.as_ref(), Some(WF), tmp.path());
        assert!(matches!(r, Err(CryptoError::UnlockFailed(_))));
        assert!(!store::identities_dir_in(tmp.path()).exists());
    }

    #[test]
    fn test_create_passphrase_identity_rejects_empty_and_cancel() {
        let tmp = tempdir().unwrap();
        let ui = MockUi::new(vec![Some(""), Some("")]);
        assert!(
            create_passphrase_identity_in(tmp.path(), ui.as_ref(), Some(WF), tmp.path()).is_err()
        );
        let ui = MockUi::new(vec![None]);
        assert!(
            create_passphrase_identity_in(tmp.path(), ui.as_ref(), Some(WF), tmp.path()).is_err()
        );
    }

    #[test]
    fn test_verify_passphrase_wrap_rejects_wrong_passphrase() {
        let tmp = tempdir().unwrap();
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        let (_, p) =
            create_passphrase_identity_in(tmp.path(), ui.as_ref(), Some(WF), tmp.path()).unwrap();
        let r = verify_passphrase_wrap_in(
            tmp.path(),
            &p,
            &SecretString::from("nope".to_string()),
            Some(WF),
        );
        assert!(matches!(r, Err(CryptoError::UnlockFailed(_))));
    }

    #[test]
    fn test_verify_passphrase_wrap_detects_public_key_mismatch() {
        let tmp = tempdir().unwrap();
        let other = x25519::Identity::generate();
        let claimed = x25519::Identity::generate().to_public().to_string();
        let pw = SecretString::from("pw".to_string());
        store::write_passphrase_identity_in(
            tmp.path(),
            &claimed,
            &other,
            &pw,
            Some(WF),
            tmp.path(),
        )
        .unwrap();
        let r = verify_passphrase_wrap_in(tmp.path(), &claimed, &pw, Some(WF));
        assert!(matches!(r, Err(CryptoError::IdentityMismatch { .. })));
    }

    #[test]
    fn test_add_touch_id_writes_both_files_with_private_mode() {
        let tmp = tempdir().unwrap();
        let identity = x25519::Identity::generate();
        let p = identity.to_public().to_string();
        let se = MockSe::new(true, false);
        let (id_path, wrapped_path) =
            add_touch_id_in(tmp.path(), &se, &identity, &p, tmp.path()).unwrap();
        assert_eq!(id_path, store::se_identity_path_in(tmp.path(), &p));
        assert_eq!(wrapped_path, store::se_wrapped_path_in(tmp.path(), &p));
        assert!(std::fs::read_to_string(&id_path)
            .unwrap()
            .contains("AGE-PLUGIN-SE-1"));
        assert!(wrapped_path.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&id_path, &wrapped_path] {
                let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600);
            }
        }
    }

    #[test]
    fn test_add_touch_id_wraps_plaintext_with_project_line() {
        let tmp = tempdir().unwrap();
        let project = tempdir().unwrap();
        let identity = x25519::Identity::generate();
        let p = identity.to_public().to_string();
        let se = MockSe::new(true, false);
        let (_, wrapped_path) =
            add_touch_id_in(tmp.path(), &se, &identity, &p, project.path()).unwrap();
        let wrapped = std::fs::read(wrapped_path).unwrap();
        // MockSe "encrypts" by prefixing.
        let plain = wrapped.strip_prefix(b"MOCK-SE:").unwrap();
        assert_eq!(
            store::parse_project_binding(plain).as_deref(),
            std::fs::canonicalize(project.path()).unwrap().to_str()
        );
    }

    #[test]
    fn test_create_passphrase_identity_binds_project() {
        let tmp = tempdir().unwrap();
        let project = tempdir().unwrap();
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        let (_, p) =
            create_passphrase_identity_in(tmp.path(), ui.as_ref(), Some(WF), project.path())
                .unwrap();
        let bytes = std::fs::read(store::identity_path_in(tmp.path(), &p)).unwrap();
        let plain =
            store::unwrap_with_passphrase(&bytes, &SecretString::from("pw".to_string()), Some(WF))
                .unwrap();
        assert_eq!(
            store::parse_project_binding(&plain).as_deref(),
            std::fs::canonicalize(project.path()).unwrap().to_str()
        );
    }

    #[test]
    fn test_add_touch_id_errors_when_already_present() {
        let tmp = tempdir().unwrap();
        let identity = x25519::Identity::generate();
        let p = identity.to_public().to_string();
        let se = MockSe::new(true, false);
        add_touch_id_in(tmp.path(), &se, &identity, &p, tmp.path()).unwrap();
        let again = add_touch_id_in(tmp.path(), &se, &identity, &p, tmp.path());
        assert!(matches!(again, Err(CryptoError::StoreError(_))));
    }
}
