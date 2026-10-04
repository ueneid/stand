//! Encryption management commands.

//!
//! Private keys are never stored in the project directory. `enable` creates a
//! passphrase-wrapped identity in the user's config directory (optionally with a
//! Touch ID wrap); `migrate` moves a legacy `.stand.keys` there. All core logic
//! takes an [`UnlockContext`] so it can run against mock UIs and backends; the
//! plain functions are thin wrappers using the production context.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use colored::Colorize;
use toml_edit::{DocumentMut, Item, Value};

use crate::crypto::enable as crypto_enable;
use crate::crypto::ui::sanitize_for_terminal;
use crate::crypto::{keys, store, CryptoError, DecryptPurpose, UnlockContext, UnlockUi};
use crate::crypto::{unlock_for_project_with, ENCRYPTED_PREFIX};

/// Escapes terminal control characters in values shown to the user.
fn san(value: &str) -> String {
    sanitize_for_terminal(value)
}

fn san_path(path: &Path) -> String {
    sanitize_for_terminal(&path.display().to_string())
}

const KEYS_FILE: &str = ".stand.keys";
const CONFIG_FILE: &str = ".stand.toml";

const PLUGIN_MISSING_HINT: &str =
    "age-plugin-se was not found in PATH. Install it with `brew install age-plugin-se`";

/// Enable encryption for the project (production wrapper).
///
/// Creates a passphrase-wrapped identity outside the project and adds the
/// public key to `.stand.toml`. With `touch_id`, also adds a Touch ID wrap.
pub fn enable_encryption(project_dir: &Path, touch_id: bool) -> Result<(), EncryptionCommandError> {
    let ctx = UnlockContext::production()?;
    enable_encryption_with(project_dir, touch_id, &ctx)
}

/// [`enable_encryption`] with an injected context.
pub fn enable_encryption_with(
    project_dir: &Path,
    touch_id: bool,
    ctx: &UnlockContext,
) -> Result<(), EncryptionCommandError> {
    let (config_path, mut doc) = load_doc(project_dir)?;
    if doc.get("encryption").is_some() {
        return Err(EncryptionCommandError::AlreadyEnabled);
    }
    let ui = require_ui(ctx)?;

    let (identity, public_key) = crypto_enable::create_passphrase_identity_in(
        &ctx.env.config_dir,
        ui.as_ref(),
        ctx.env.max_work_factor,
    )?;

    // Add [encryption] section using toml_edit (preserves formatting).
    let mut encryption_table = toml_edit::Table::new();
    encryption_table.insert("public_key", toml_edit::value(&public_key));
    doc.insert("encryption", Item::Table(encryption_table));

    // Write config LAST. If this fails, roll back the identity files.
    if let Err(e) = fs::write(&config_path, doc.to_string()) {
        if let Err(cleanup_err) = store::remove_identity_files_in(&ctx.env.config_dir, &public_key)
        {
            eprintln!(
                "Warning: Failed to clean up identity files after configuration write error: {}",
                cleanup_err
            );
        }
        return Err(e.into());
    }

    println!("{} Generated key pair", "✓".green());
    println!(
        "{} Saved passphrase-protected key to {}",
        "✓".green(),
        san_path(&store::identity_path_in(&ctx.env.config_dir, &public_key))
    );
    println!(
        "{} Added [encryption] section to {}",
        "✓".green(),
        CONFIG_FILE
    );

    if touch_id {
        let setup = if ctx.secure_enclave.is_available() {
            crypto_enable::add_touch_id_in(
                &ctx.env.config_dir,
                ctx.secure_enclave.as_ref(),
                &identity,
                &public_key,
            )
            .map(|_| ())
        } else {
            Err(CryptoError::SecureEnclaveUnavailable(
                PLUGIN_MISSING_HINT.to_string(),
            ))
        };
        match setup {
            Ok(()) => println!("{} Touch ID enabled", "✓".green()),
            Err(e) => return Err(EncryptionCommandError::TouchIdSetup(san(&e.to_string()))),
        }
    }

    Ok(())
}

/// Migrate a legacy `.stand.keys` into the wrapped identity store (production wrapper).
pub fn migrate_encryption(project_dir: &Path) -> Result<(), EncryptionCommandError> {
    let ctx = UnlockContext::production()?;
    migrate_encryption_with(project_dir, &ctx)
}

/// [`migrate_encryption`] with an injected context.
///
/// Values stay untouched: only the private key moves. The legacy file is
/// removed only after the new wrap was verified by unwrapping it again.
pub fn migrate_encryption_with(
    project_dir: &Path,
    ctx: &UnlockContext,
) -> Result<(), EncryptionCommandError> {
    let (_, doc) = load_doc(project_dir)?;
    let public_key = public_key_from_doc(&doc).ok_or(EncryptionCommandError::NotEnabled)?;
    store::validate_public_key(&public_key)?;

    let keys_path = project_dir.join(KEYS_FILE);
    if !keys_path.exists() {
        return Err(EncryptionCommandError::KeysFileNotFound);
    }
    let legacy = keys::parse_private_key(&keys::load_private_key(&keys_path)?)?;
    if legacy.to_public().to_string() != public_key {
        return Err(EncryptionCommandError::PublicKeyMismatch {
            expected: public_key,
        });
    }

    let identity_path = store::identity_path_in(&ctx.env.config_dir, &public_key);
    if identity_path.symlink_metadata().is_ok() {
        return Err(EncryptionCommandError::IdentityAlreadyExists(san_path(
            &identity_path,
        )));
    }

    let ui = require_ui(ctx)?;
    let passphrase = crypto_enable::prompt_new_passphrase(ui.as_ref())?;
    store::write_passphrase_identity_in(
        &ctx.env.config_dir,
        &public_key,
        &legacy,
        &passphrase,
        ctx.env.max_work_factor,
    )?;
    if let Err(e) = crypto_enable::verify_passphrase_wrap_in(
        &ctx.env.config_dir,
        &public_key,
        &passphrase,
        ctx.env.max_work_factor,
    ) {
        let _ = fs::remove_file(&identity_path);
        return Err(e.into());
    }
    println!(
        "{} Saved passphrase-protected key to {}",
        "✓".green(),
        san_path(&identity_path)
    );

    store::wipe_and_delete(&keys_path)?;
    println!("{} Removed {}", "✓".green(), KEYS_FILE);
    println!("Hint: run `stand encrypt add-touch-id` to unlock with Touch ID.");
    Ok(())
}

/// Add a Touch ID wrap to an existing passphrase identity (production wrapper).
pub fn add_touch_id(project_dir: &Path) -> Result<(), EncryptionCommandError> {
    let ctx = UnlockContext::production()?;
    add_touch_id_with(project_dir, &ctx)
}

/// [`add_touch_id`] with an injected context.
pub fn add_touch_id_with(
    project_dir: &Path,
    ctx: &UnlockContext,
) -> Result<(), EncryptionCommandError> {
    let (_, doc) = load_doc(project_dir)?;
    let public_key = public_key_from_doc(&doc).ok_or(EncryptionCommandError::NotEnabled)?;
    store::validate_public_key(&public_key)?;

    let config_dir = &ctx.env.config_dir;
    let identity_path = store::identity_path_in(config_dir, &public_key);
    if !identity_path.exists() {
        return Err(EncryptionCommandError::IdentityNotFound(san_path(
            &identity_path,
        )));
    }
    let wrapped_path = store::se_wrapped_path_in(config_dir, &public_key);
    if wrapped_path.exists() {
        return Err(CryptoError::StoreError(format!(
            "Touch ID is already set up ({} exists)",
            san_path(&wrapped_path)
        ))
        .into());
    }
    require_ui(ctx)?;
    if !ctx.secure_enclave.is_available() {
        return Err(CryptoError::SecureEnclaveUnavailable(PLUGIN_MISSING_HINT.to_string()).into());
    }

    let mut purpose = DecryptPurpose::new("stand encrypt add-touch-id", None);
    purpose.secret_count = 1;
    let identity = unlock_for_project_with(project_dir, &purpose, ctx)?;
    crypto_enable::add_touch_id_in(
        config_dir,
        ctx.secure_enclave.as_ref(),
        &identity,
        &public_key,
    )?;
    println!("{} Touch ID enabled", "✓".green());
    Ok(())
}

/// Disable encryption for the project (production wrapper).
pub fn disable_encryption(project_dir: &Path) -> Result<(), EncryptionCommandError> {
    let ctx = UnlockContext::production()?;
    disable_encryption_with(project_dir, &ctx)
}

/// [`disable_encryption`] with an injected context.
///
/// Prompts for confirmation (on the terminal when there is one, else on
/// stdin), then decrypts all encrypted values and removes the encryption
/// configuration. If the user declines, returns Ok(()) without changes.
pub fn disable_encryption_with(
    project_dir: &Path,
    ctx: &UnlockContext,
) -> Result<(), EncryptionCommandError> {
    let (_, doc) = load_doc(project_dir)?;
    if doc.get("encryption").is_none() {
        return Err(EncryptionCommandError::NotEnabled);
    }

    let message = "This will decrypt all encrypted values and remove encryption. Continue?";
    let confirmed = match ctx.ui.as_ref().filter(|_| !ctx.env.force_non_tty) {
        Some(ui) => ui.confirm(message).unwrap_or(false),
        None => {
            println!(
                "{} This will decrypt all encrypted values and remove encryption.",
                "⚠".yellow()
            );
            print!("Continue? [y/N]: ");
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input.trim().eq_ignore_ascii_case("y")
        }
    };
    if !confirmed {
        println!("Aborted.");
        return Ok(());
    }

    let result = disable_encryption_internal(project_dir, ctx)?;

    if result.decrypted_count > 0 {
        println!(
            "{} Decrypted {} value(s)",
            "✓".green(),
            result.decrypted_count
        );
    }
    println!("{} Removed [encryption] section", "✓".green());
    for path in &result.removed_files {
        println!("{} Removed {}", "✓".green(), san_path(path));
    }
    println!("{} Encryption disabled", "✓".green());
    Ok(())
}

/// Result of the disable_encryption_internal operation.
#[derive(Debug, Default)]
pub struct DisableEncryptionResult {
    /// Number of values successfully decrypted.
    pub decrypted_count: usize,
    /// Key files that were deleted (identity store files and legacy `.stand.keys`).
    pub removed_files: Vec<PathBuf>,
}

/// Disables encryption without asking for confirmation.
///
/// Separated from [`disable_encryption_with`] to allow testing without
/// interactive prompts. If no encrypted values exist, nothing is unlocked.
/// Files are deleted only after the configuration was written successfully.
pub fn disable_encryption_internal(
    project_dir: &Path,
    ctx: &UnlockContext,
) -> Result<DisableEncryptionResult, EncryptionCommandError> {
    let (config_path, mut doc) = load_doc(project_dir)?;
    let public_key = public_key_from_doc(&doc);
    if doc.get("encryption").is_none() {
        return Err(EncryptionCommandError::NotEnabled);
    }

    let mut result = DisableEncryptionResult::default();

    let secret_count = count_encrypted_values_in_doc(&doc);
    if secret_count > 0 {
        let mut purpose = DecryptPurpose::new("stand encrypt disable", None);
        purpose.secret_count = secret_count;
        let identity = unlock_for_project_with(project_dir, &purpose, ctx)?;

        let mut decrypt_table =
            |table: &mut toml_edit::Table| -> Result<(), EncryptionCommandError> {
                for (key, value) in table.iter_mut() {
                    let Some(val_str) = value.as_str() else {
                        continue;
                    };
                    if !val_str.starts_with(ENCRYPTED_PREFIX) {
                        continue;
                    }
                    let decrypted =
                        crate::crypto::decrypt_value(val_str, &identity).map_err(|e| {
                            EncryptionCommandError::DecryptionFailed {
                                variable: san(&key),
                                reason: san(&e.to_string()),
                            }
                        })?;
                    *value = Item::Value(Value::from(decrypted));
                    result.decrypted_count += 1;
                }
                Ok(())
            };

        if let Some(env_table) = doc.get_mut("environments").and_then(|e| e.as_table_mut()) {
            for (_name, env_config) in env_table.iter_mut() {
                if let Some(t) = env_config.as_table_mut() {
                    decrypt_table(t)?;
                }
            }
        }
        if let Some(t) = doc.get_mut("common").and_then(|c| c.as_table_mut()) {
            decrypt_table(t)?;
        }
    }

    doc.remove("encryption");
    fs::write(&config_path, doc.to_string())?;

    // Config is written; now delete the key material.
    if let Some(p) = public_key.as_deref() {
        if store::validate_public_key(p).is_ok() {
            result
                .removed_files
                .extend(store::remove_identity_files_in(&ctx.env.config_dir, p)?);
        }
    }
    let keys_path = project_dir.join(KEYS_FILE);
    if keys_path.exists() {
        store::wipe_and_delete(&keys_path)?;
        result.removed_files.push(keys_path);
    }

    Ok(result)
}

/// Reads and parses `.stand.toml` with toml_edit (preserving formatting).
fn load_doc(project_dir: &Path) -> Result<(PathBuf, DocumentMut), EncryptionCommandError> {
    let config_path = project_dir.join(CONFIG_FILE);
    if !config_path.exists() {
        return Err(EncryptionCommandError::ConfigNotFound);
    }
    let content = fs::read_to_string(&config_path)?;
    let doc: DocumentMut = content
        .parse()
        .map_err(|e| EncryptionCommandError::TomlParse(format!("{}", e)))?;
    Ok((config_path, doc))
}

fn public_key_from_doc(doc: &DocumentMut) -> Option<String> {
    doc.get("encryption")?
        .get("public_key")?
        .as_str()
        .map(str::to_string)
}

/// Returns the terminal UI or [`CryptoError::NoTerminal`].
fn require_ui(ctx: &UnlockContext) -> Result<std::sync::Arc<dyn UnlockUi>, CryptoError> {
    match (&ctx.ui, ctx.env.force_non_tty) {
        (Some(ui), false) => Ok(ui.clone()),
        _ => Err(CryptoError::NoTerminal),
    }
}

/// Counts encrypted values in [environments.*] and [common].
fn count_encrypted_values_in_doc(doc: &DocumentMut) -> usize {
    let count_in = |table: &toml_edit::Table| {
        table
            .iter()
            .filter(|(_, v)| v.as_str().is_some_and(|s| s.starts_with(ENCRYPTED_PREFIX)))
            .count()
    };
    let mut total = 0;
    if let Some(env_table) = doc.get("environments").and_then(|e| e.as_table()) {
        for (_, env_config) in env_table.iter() {
            if let Some(t) = env_config.as_table() {
                total += count_in(t);
            }
        }
    }
    if let Some(t) = doc.get("common").and_then(|c| c.as_table()) {
        total += count_in(t);
    }
    total
}

/// Error type for encryption commands.
#[derive(Debug, thiserror::Error)]
pub enum EncryptionCommandError {
    #[error("Configuration file not found. Run 'stand init' first.")]
    ConfigNotFound,

    #[error("Encryption is already enabled for this project")]
    AlreadyEnabled,

    #[error("Encryption is not enabled for this project")]
    NotEnabled,

    #[error("No legacy .stand.keys found in the project directory; nothing to migrate")]
    KeysFileNotFound,

    #[error("The key in .stand.keys does not match the project public key {expected}")]
    PublicKeyMismatch { expected: String },

    #[error("No passphrase-protected key found at {0}. Run 'stand encrypt migrate' (legacy .stand.keys) or 'stand encrypt enable' first.")]
    IdentityNotFound(String),

    #[error("A passphrase-protected key already exists at {0}; refusing to overwrite it. Move it away first if you really want to migrate again.")]
    IdentityAlreadyExists(String),

    #[error("Touch ID setup failed: {0}. Your passphrase setup is complete; run 'stand encrypt add-touch-id' once the problem is fixed.")]
    TouchIdSetup(String),

    #[error("Cryptographic error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("TOML parsing error: {0}")]
    TomlParse(String),

    #[error("Failed to decrypt variable '{variable}': {reason}. All values must be decryptable to disable encryption.")]
    DecryptionFailed { variable: String, reason: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::testing::{MockSe, MockUi};
    use crate::crypto::{store, UnlockEnv, UnlockUi};
    use age::secrecy::SecretString;
    use std::sync::Arc;
    use tempfile::{tempdir, TempDir};

    const WF: u8 = 8;
    const BASE_CONFIG: &str =
        "version = \"1.0\"\n\n[environments.dev]\ndescription = \"Development\"\n";

    struct Fixture {
        _tmp: TempDir,
        project: std::path::PathBuf,
        config: std::path::PathBuf,
    }

    fn fixture() -> Fixture {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        let config = tmp.path().join("config");
        fs::create_dir_all(&project).unwrap();
        Fixture {
            _tmp: tmp,
            project,
            config,
        }
    }

    fn ctx(f: &Fixture, ui: Option<Arc<MockUi>>, se_available: bool) -> UnlockContext {
        UnlockContext {
            env: UnlockEnv {
                ci: None,
                key_source: None,
                private_key: None,
                unlock_mode: None,
                force_non_tty: false,
                config_dir: f.config.clone(),
                max_work_factor: Some(WF),
            },
            ui: ui.map(|u| u as Arc<dyn UnlockUi>),
            secure_enclave: Box::new(MockSe::new(se_available, false)),
            warn: Arc::new(|_| {}),
        }
    }

    fn write_config(f: &Fixture, content: &str) {
        fs::write(f.project.join(CONFIG_FILE), content).unwrap();
    }

    fn read_config(f: &Fixture) -> String {
        fs::read_to_string(f.project.join(CONFIG_FILE)).unwrap()
    }

    fn public_key_of(f: &Fixture) -> String {
        crate::crypto::read_project_public_key(&f.project).unwrap()
    }

    /// Enables encryption through the real code path with passphrase "pw".
    fn enabled(f: &Fixture) -> String {
        write_config(f, BASE_CONFIG);
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        enable_encryption_with(&f.project, false, &ctx(f, Some(ui), true)).unwrap();
        public_key_of(f)
    }

    fn encrypt_for(p: &str, plain: &str) -> String {
        let r = crate::crypto::keys::parse_public_key(p).unwrap();
        crate::crypto::encrypt_value(plain, &r).unwrap()
    }

    // === enable ===

    #[test]
    fn test_enable_no_config() {
        let f = fixture();
        let ui = MockUi::new(vec![]);
        let r = enable_encryption_with(&f.project, false, &ctx(&f, Some(ui), true));
        assert!(matches!(r, Err(EncryptionCommandError::ConfigNotFound)));
    }

    #[test]
    fn test_enable_creates_wrapped_identity_and_no_keys_file() {
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        enable_encryption_with(&f.project, false, &ctx(&f, Some(ui), true)).unwrap();

        let config = read_config(&f);
        assert!(config.contains("[encryption]"));
        assert!(config.contains("public_key = \"age1"));
        assert!(config.contains("[environments.dev]"));
        let p = public_key_of(&f);
        assert!(store::identity_path_in(&f.config, &p).exists());
        assert!(!store::se_wrapped_path_in(&f.config, &p).exists());
        assert!(!f.project.join(KEYS_FILE).exists());
    }

    #[test]
    fn test_enable_already_enabled() {
        let f = fixture();
        write_config(
            &f,
            "version = \"1.0\"\n\n[encryption]\npublic_key = \"age1test\"\n",
        );
        let ui = MockUi::new(vec![]);
        let r = enable_encryption_with(&f.project, false, &ctx(&f, Some(ui), true));
        assert!(matches!(r, Err(EncryptionCommandError::AlreadyEnabled)));
    }

    #[test]
    fn test_enable_requires_terminal() {
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        let r = enable_encryption_with(&f.project, false, &ctx(&f, None, true));
        assert!(matches!(
            r,
            Err(EncryptionCommandError::Crypto(CryptoError::NoTerminal))
        ));
        assert_eq!(read_config(&f), BASE_CONFIG);
        assert!(!store::identities_dir_in(&f.config).exists());
    }

    #[test]
    fn test_enable_passphrase_mismatch_changes_nothing() {
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        let ui = MockUi::new(vec![Some("a"), Some("b")]);
        let r = enable_encryption_with(&f.project, false, &ctx(&f, Some(ui), true));
        assert!(r.is_err());
        assert_eq!(read_config(&f), BASE_CONFIG);
        assert!(!store::identities_dir_in(&f.config).exists());
    }

    #[test]
    fn test_enable_with_touch_id_writes_all_three_files() {
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        enable_encryption_with(&f.project, true, &ctx(&f, Some(ui), true)).unwrap();
        let p = public_key_of(&f);
        assert!(store::identity_path_in(&f.config, &p).exists());
        assert!(store::se_identity_path_in(&f.config, &p).exists());
        assert!(store::se_wrapped_path_in(&f.config, &p).exists());
    }

    #[test]
    fn test_enable_touch_id_without_plugin_keeps_passphrase_setup() {
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        let r = enable_encryption_with(&f.project, true, &ctx(&f, Some(ui), false));
        let err = r.unwrap_err();
        assert!(matches!(err, EncryptionCommandError::TouchIdSetup(_)));
        let msg = err.to_string();
        assert!(msg.contains("brew install age-plugin-se"));
        assert!(msg.contains("passphrase setup is complete"));
        let p = public_key_of(&f);
        assert!(store::identity_path_in(&f.config, &p).exists());
        assert!(!store::se_wrapped_path_in(&f.config, &p).exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_enable_rolls_back_identity_when_config_write_fails() {
        use std::os::unix::fs::PermissionsExt;
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        // Make the config read-only so the final write fails.
        let path = f.project.join(CONFIG_FILE);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        if fs::OpenOptions::new().write(true).open(&path).is_ok() {
            return; // running as root: permissions are not enforced
        }
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        let r = enable_encryption_with(&f.project, false, &ctx(&f, Some(ui), true));
        assert!(r.is_err());
        let dir = store::identities_dir_in(&f.config);
        let left = fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
        assert_eq!(left, 0, "identity files must be rolled back");
    }

    // === migrate ===

    fn legacy_setup(f: &Fixture, secret: &str) -> (String, String) {
        let kp = crate::crypto::keys::generate_key_pair();
        crate::crypto::keys::save_private_key(&f.project.join(KEYS_FILE), &kp.private_key).unwrap();
        let enc = encrypt_for(&kp.public_key, secret);
        write_config(
            f,
            &format!(
                "version = \"1.0\"\n\n[encryption]\npublic_key = \"{}\"\n\n[environments.dev]\ndescription = \"d\"\nSECRET = \"{}\"\n",
                kp.public_key, enc
            ),
        );
        (kp.public_key, enc)
    }

    #[test]
    fn test_migrate_wraps_key_and_removes_keys_file() {
        let f = fixture();
        let (p, enc) = legacy_setup(&f, "s3cret");
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        let c = ctx(&f, Some(ui), true);
        migrate_encryption_with(&f.project, &c).unwrap();

        assert!(store::identity_path_in(&f.config, &p).exists());
        assert!(!f.project.join(KEYS_FILE).exists());
        assert!(read_config(&f).contains(&enc), "values must be untouched");

        // Values still decrypt through the gate with the new passphrase.
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(&f, Some(ui), true);
        let purpose = DecryptPurpose::new("test", None);
        let out = crate::crypto::decrypt_single_value_with(&enc, &f.project, &purpose, &c).unwrap();
        assert_eq!(out, "s3cret");
    }

    #[test]
    fn test_migrate_refuses_when_wrapped_identity_already_exists() {
        let f = fixture();
        let (p, _) = legacy_setup(&f, "x");
        store::ensure_identities_dir_in(&f.config).unwrap();
        let existing = store::identity_path_in(&f.config, &p);
        fs::write(&existing, b"precious existing wrap").unwrap();
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        let r = migrate_encryption_with(&f.project, &ctx(&f, Some(ui.clone()), true));
        assert!(matches!(
            r,
            Err(EncryptionCommandError::IdentityAlreadyExists(_))
        ));
        assert_eq!(fs::read(&existing).unwrap(), b"precious existing wrap");
        assert!(f.project.join(KEYS_FILE).exists());
        assert_eq!(ui.prompt_count(), 0, "must refuse before prompting");
    }

    #[test]
    fn test_migrate_requires_keys_file() {
        let f = fixture();
        let _ = enabled(&f);
        let ui = MockUi::new(vec![]);
        let r = migrate_encryption_with(&f.project, &ctx(&f, Some(ui), true));
        assert!(matches!(r, Err(EncryptionCommandError::KeysFileNotFound)));
    }

    #[test]
    fn test_migrate_requires_encryption_section() {
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        let kp = crate::crypto::keys::generate_key_pair();
        crate::crypto::keys::save_private_key(&f.project.join(KEYS_FILE), &kp.private_key).unwrap();
        let ui = MockUi::new(vec![]);
        let r = migrate_encryption_with(&f.project, &ctx(&f, Some(ui), true));
        assert!(matches!(r, Err(EncryptionCommandError::NotEnabled)));
    }

    #[test]
    fn test_migrate_rejects_mismatching_legacy_key() {
        let f = fixture();
        let _ = legacy_setup(&f, "x");
        let other = crate::crypto::keys::generate_key_pair();
        fs::remove_file(f.project.join(KEYS_FILE)).unwrap();
        crate::crypto::keys::save_private_key(&f.project.join(KEYS_FILE), &other.private_key)
            .unwrap();
        let ui = MockUi::new(vec![Some("pw"), Some("pw")]);
        let r = migrate_encryption_with(&f.project, &ctx(&f, Some(ui), true));
        assert!(matches!(
            r,
            Err(EncryptionCommandError::PublicKeyMismatch { .. })
        ));
        assert!(f.project.join(KEYS_FILE).exists());
    }

    #[test]
    fn test_migrate_requires_terminal_and_keeps_keys_file() {
        let f = fixture();
        let _ = legacy_setup(&f, "x");
        let r = migrate_encryption_with(&f.project, &ctx(&f, None, true));
        assert!(matches!(
            r,
            Err(EncryptionCommandError::Crypto(CryptoError::NoTerminal))
        ));
        assert!(f.project.join(KEYS_FILE).exists());
    }

    // === add-touch-id ===

    #[test]
    fn test_add_touch_id_writes_se_files() {
        let f = fixture();
        let p = enabled(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        add_touch_id_with(&f.project, &ctx(&f, Some(ui), true)).unwrap();
        assert!(store::se_identity_path_in(&f.config, &p).exists());
        assert!(store::se_wrapped_path_in(&f.config, &p).exists());
    }

    #[test]
    fn test_add_touch_id_errors_when_already_present() {
        let f = fixture();
        let p = enabled(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        add_touch_id_with(&f.project, &ctx(&f, Some(ui), true)).unwrap();
        let ui = MockUi::new(vec![Some("pw")]);
        let r = add_touch_id_with(&f.project, &ctx(&f, Some(ui), true));
        assert!(r.is_err());
        assert!(store::se_wrapped_path_in(&f.config, &p).exists());
    }

    #[test]
    fn test_add_touch_id_requires_encryption_and_identity() {
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        let ui = MockUi::new(vec![]);
        let r = add_touch_id_with(&f.project, &ctx(&f, Some(ui), true));
        assert!(matches!(r, Err(EncryptionCommandError::NotEnabled)));

        let kp = crate::crypto::keys::generate_key_pair();
        write_config(
            &f,
            &format!(
                "version = \"1.0\"\n\n[encryption]\npublic_key = \"{}\"\n",
                kp.public_key
            ),
        );
        let ui = MockUi::new(vec![]);
        let r = add_touch_id_with(&f.project, &ctx(&f, Some(ui), true));
        assert!(matches!(
            r,
            Err(EncryptionCommandError::IdentityNotFound(_))
        ));
    }

    #[test]
    fn test_add_touch_id_without_plugin_fails_with_hint() {
        let f = fixture();
        let _ = enabled(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        let r = add_touch_id_with(&f.project, &ctx(&f, Some(ui), false));
        assert!(r
            .unwrap_err()
            .to_string()
            .contains("brew install age-plugin-se"));
    }

    // === disable ===

    #[test]
    fn test_disable_decrypts_values_and_removes_identity_files() {
        let f = fixture();
        let p = enabled(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        add_touch_id_with(&f.project, &ctx(&f, Some(ui), true)).unwrap();

        let enc1 = encrypt_for(&p, "secret1");
        let enc2 = encrypt_for(&p, "secret2");
        let content = read_config(&f);
        let content = content.replace(
            "description = \"Development\"\n",
            &format!("description = \"Development\"\nAPI_KEY = \"{}\"\n", enc1),
        );
        write_config(
            &f,
            &format!("{}\n[common]\nSHARED = \"{}\"\n", content, enc2),
        );

        // Touch ID is tried first; the mock backend unwraps without prompting.
        let ui = MockUi::new(vec![Some("pw")]);
        let result = disable_encryption_internal(&f.project, &ctx(&f, Some(ui), true)).unwrap();
        assert_eq!(result.decrypted_count, 2);
        assert_eq!(result.removed_files.len(), 3);

        let after = read_config(&f);
        assert!(!after.contains("[encryption]"));
        assert!(after.contains("API_KEY = \"secret1\""));
        assert!(after.contains("SHARED = \"secret2\""));
        assert!(!store::identity_path_in(&f.config, &p).exists());
        assert!(!store::se_wrapped_path_in(&f.config, &p).exists());
        assert!(!store::se_identity_path_in(&f.config, &p).exists());
    }

    #[test]
    fn test_disable_with_confirmation_declined_changes_nothing() {
        struct Decline;
        impl UnlockUi for Decline {
            fn show(&self, _: &str) {}
            fn request_passphrase(&self, _: &str) -> Option<SecretString> {
                None
            }
            fn confirm(&self, _: &str) -> Option<bool> {
                Some(false)
            }
        }
        let f = fixture();
        let p = enabled(&f);
        let mut c = ctx(&f, None, true);
        c.ui = Some(Arc::new(Decline));
        disable_encryption_with(&f.project, &c).unwrap();
        assert!(read_config(&f).contains("[encryption]"));
        assert!(store::identity_path_in(&f.config, &p).exists());
    }

    #[test]
    fn test_disable_removes_legacy_keys_file() {
        let f = fixture();
        let _ = legacy_setup(&f, "legacy-secret");
        let c = ctx(&f, Some(MockUi::new(vec![])), true);
        let result = disable_encryption_internal(&f.project, &c).unwrap();
        assert_eq!(result.decrypted_count, 1);
        assert!(!f.project.join(KEYS_FILE).exists());
        assert!(read_config(&f).contains("SECRET = \"legacy-secret\""));
    }

    #[test]
    fn test_disable_error_sanitizes_variable_name() {
        let f = fixture();
        let _ = enabled(&f);
        let content = read_config(&f).replace(
            "description = \"Development\"\n",
            "description = \"Development\"\n\"BAD\\u001b[2K\" = \"encrypted:not-valid\"\n",
        );
        write_config(&f, &content);
        let ui = MockUi::new(vec![Some("pw")]);
        let err = disable_encryption_internal(&f.project, &ctx(&f, Some(ui), true)).unwrap_err();
        assert!(!err.to_string().contains('\x1b'));
        assert!(err.to_string().contains("BAD\\x1b[2K"));
    }

    #[test]
    fn test_disable_not_enabled() {
        let f = fixture();
        write_config(&f, BASE_CONFIG);
        let c = ctx(&f, Some(MockUi::new(vec![])), true);
        let r = disable_encryption_internal(&f.project, &c);
        assert!(matches!(r, Err(EncryptionCommandError::NotEnabled)));
    }

    #[test]
    fn test_disable_without_encrypted_values_needs_no_unlock() {
        let f = fixture();
        let p = enabled(&f);
        let c = ctx(&f, Some(MockUi::new(vec![])), true);
        let result = disable_encryption_internal(&f.project, &c).unwrap();
        assert_eq!(result.decrypted_count, 0);
        assert!(!read_config(&f).contains("[encryption]"));
        assert!(!store::identity_path_in(&f.config, &p).exists());
    }

    #[test]
    fn test_disable_fails_on_malformed_value_and_keeps_everything() {
        let f = fixture();
        let p = enabled(&f);
        let content = read_config(&f);
        let content = content.replace(
            "description = \"Development\"\n",
            "description = \"Development\"\nBAD = \"encrypted:not-valid\"\n",
        );
        write_config(&f, &content);
        let before = read_config(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        let r = disable_encryption_internal(&f.project, &ctx(&f, Some(ui), true));
        assert!(matches!(
            r,
            Err(EncryptionCommandError::DecryptionFailed { .. })
        ));
        assert_eq!(read_config(&f), before);
        assert!(store::identity_path_in(&f.config, &p).exists());
    }
}
