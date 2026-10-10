//! The unlock gate: the single chokepoint every decryption goes through.
//!
//! Threat model: an AI agent runs as the same OS user in the project
//! directory and must not be able to decrypt secrets. Such a process has no
//! controlling terminal and no access to the human's passphrase or biometrics,
//! so unlocking requires (1) an openable `/dev/tty` and (2) a passphrase or
//! Touch ID. CI is the explicit exception (`CI=true` + `STAND_PRIVATE_KEY`).

use std::path::Path;
use std::sync::Arc;

use age::secrecy::{SecretSlice, SecretString};
use age::x25519;

use super::store::{self, UnwrapError};
use super::touch_id::{RealSecureEnclave, SecureEnclaveBackend};
use super::tty::TtyUi;
use super::ui::{sanitize_for_terminal, DecryptPurpose, UnlockUi};
use super::{keys, CryptoError};

/// Maximum number of passphrase attempts.
const MAX_PASSPHRASE_ATTEMPTS: usize = 3;

const LEGACY_WARNING: &str = "warning: using legacy .stand.keys in the project directory; any process, including AI agents, can read it. Run `stand encrypt migrate`.";

/// Snapshot of everything the gate reads from the environment.
#[derive(Debug, Clone)]
pub struct UnlockEnv {
    /// `CI`
    pub ci: Option<String>,
    /// `STAND_KEY_SOURCE`
    pub key_source: Option<String>,
    /// `STAND_PRIVATE_KEY`
    pub private_key: Option<String>,
    /// `STAND_UNLOCK` (`passphrase` skips Touch ID)
    pub unlock_mode: Option<String>,
    /// `STAND_FORCE_NON_TTY` is set: behave as if `/dev/tty` cannot be opened.
    pub force_non_tty: bool,
    /// Stand config directory holding `identities/`.
    pub config_dir: std::path::PathBuf,
    /// Upper bound for the scrypt work factor accepted on load (`None` = age default).
    pub max_work_factor: Option<u8>,
}

impl UnlockEnv {
    /// Reads the real process environment.
    pub fn from_process() -> Result<Self, CryptoError> {
        let var = |name: &str| std::env::var_os(name).map(|v| v.to_string_lossy().into_owned());
        Ok(Self {
            ci: var("CI"),
            key_source: var("STAND_KEY_SOURCE"),
            private_key: var("STAND_PRIVATE_KEY"),
            unlock_mode: var("STAND_UNLOCK"),
            force_non_tty: std::env::var_os("STAND_FORCE_NON_TTY").is_some(),
            config_dir: store::config_dir()?,
            max_work_factor: None,
        })
    }

    fn env_key_opted_in(&self) -> bool {
        let ci = self
            .ci
            .as_deref()
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);
        let source = self
            .key_source
            .as_deref()
            .map(|v| v.eq_ignore_ascii_case("env"))
            .unwrap_or(false);
        ci || source
    }
}

/// Everything the gate needs, injectable for tests.
pub struct UnlockContext {
    pub env: UnlockEnv,
    /// `None` means no terminal is available.
    pub ui: Option<Arc<dyn UnlockUi>>,
    pub secure_enclave: Box<dyn SecureEnclaveBackend>,
    /// Receives warnings (production prints them to stderr).
    pub warn: Arc<dyn Fn(&str) + Send + Sync>,
}

impl UnlockContext {
    /// Production context: real environment, `/dev/tty` and the real Secure Enclave.
    pub fn production() -> Result<Self, CryptoError> {
        let env = UnlockEnv::from_process()?;
        let ui: Option<Arc<dyn UnlockUi>> = if env.force_non_tty {
            None
        } else {
            TtyUi::new().ok().map(|t| Arc::new(t) as Arc<dyn UnlockUi>)
        };
        Ok(Self {
            env,
            ui,
            secure_enclave: Box::new(RealSecureEnclave),
            warn: Arc::new(|m| eprintln!("{}", m)),
        })
    }
}

/// Proof that an unwrapped identity was bound to the project it was unlocked for.
///
/// Only the unlock gate can create one. Destructive operations (deleting the
/// identity files) require it, so they cannot be reached for a key that was
/// not verified in this run.
#[derive(Debug)]
pub struct VerifiedProject {
    public_key: String,
}

impl VerifiedProject {
    /// The public key of the identity that was verified.
    pub fn public_key(&self) -> &str {
        &self.public_key
    }

    #[cfg(test)]
    pub(crate) fn for_test(public_key: &str) -> Self {
        Self {
            public_key: public_key.to_string(),
        }
    }
}

/// How the project binding of an unlocked identity was established.
#[derive(Debug)]
pub enum Binding {
    /// The `# project:` line matched the current project directory.
    Verified(VerifiedProject),
    /// The binding did not match (or was missing) and the human explicitly
    /// confirmed using the key anyway.
    Confirmed,
    /// Not a wrapped identity (CI environment key or legacy `.stand.keys`).
    NotApplicable,
}

/// What the gate does when the project binding does not match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingPolicy {
    /// Read-only decryption: the human may confirm a single run (changes nothing on disk).
    ConfirmMismatch,
    /// Operations that write or delete key material: no prompt, mismatch is an error.
    RequireExact,
}

/// Result of a successful unlock.
pub struct Unlocked {
    pub identity: x25519::Identity,
    pub binding: Binding,
    /// The exact plaintext that was wrapped (including its `# project:` line).
    /// Only re-wrapping (add-touch-id) may use it; see [`Unlocked::take_wrapped_plaintext`].
    wrapped_plaintext: Option<SecretSlice<u8>>,
}

impl Unlocked {
    /// Hands out the original unwrapped identity file bytes (once), so a
    /// re-wrap preserves the recorded project binding instead of regenerating it.
    pub(crate) fn take_wrapped_plaintext(&mut self) -> Option<SecretSlice<u8>> {
        self.wrapped_plaintext.take()
    }

    /// The proof, only when the binding matched cleanly.
    pub fn verified(&self) -> Option<&VerifiedProject> {
        match &self.binding {
            Binding::Verified(v) => Some(v),
            _ => None,
        }
    }
}

/// Production entry point: unlocks the project identity for `public_key`.
pub fn unlock_project_identity(
    project_dir: &Path,
    public_key: &str,
    purpose: &DecryptPurpose,
) -> Result<x25519::Identity, CryptoError> {
    let ctx = UnlockContext::production()?;
    unlock_project_identity_with(project_dir, Some(public_key), purpose, &ctx)
}

/// Testable gate. `public_key` is `None` when the project config has no
/// `[encryption].public_key`; only the env-key and legacy paths can then work.
pub fn unlock_project_identity_with(
    project_dir: &Path,
    public_key: Option<&str>,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
) -> Result<x25519::Identity, CryptoError> {
    unlock_project_identity_verified_with(project_dir, public_key, purpose, ctx).map(|u| u.identity)
}

/// [`unlock_project_identity_with`] that also reports the project [`Binding`].
pub fn unlock_project_identity_verified_with(
    project_dir: &Path,
    public_key: Option<&str>,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
) -> Result<Unlocked, CryptoError> {
    unlock_inner(
        project_dir,
        public_key,
        purpose,
        ctx,
        BindingPolicy::ConfirmMismatch,
    )
}

/// Like [`unlock_project_identity_verified_with`] for operations that write or
/// delete key material (add-touch-id, re-wrapping). A binding mismatch is
/// refused with [`CryptoError::ProjectMismatch`] without asking the human:
/// a one-time confirmation must never become a permanent re-binding.
pub fn unlock_project_identity_exact_with(
    project_dir: &Path,
    public_key: Option<&str>,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
) -> Result<Unlocked, CryptoError> {
    unlock_inner(
        project_dir,
        public_key,
        purpose,
        ctx,
        BindingPolicy::RequireExact,
    )
}

fn unlock_inner(
    project_dir: &Path,
    public_key: Option<&str>,
    purpose: &DecryptPurpose,
    ctx: &UnlockContext,
    policy: BindingPolicy,
) -> Result<Unlocked, CryptoError> {
    let env = &ctx.env;
    let not_applicable = |identity| Unlocked {
        identity,
        binding: Binding::NotApplicable,
        wrapped_plaintext: None,
    };

    // 1. Environment key, only in CI or when explicitly requested.
    if let Some(raw) = &env.private_key {
        if !env.env_key_opted_in() {
            return Err(CryptoError::EnvKeyNotAllowed);
        }
        if raw.trim().is_empty() {
            return Err(CryptoError::InvalidPrivateKey(
                "STAND_PRIVATE_KEY environment variable is empty".to_string(),
            ));
        }
        return keys::parse_private_key(raw.trim()).map(not_applicable);
    }

    // 2. Legacy plaintext .stand.keys (transitional, no TTY required).
    let any_wrapped = match public_key {
        Some(p) if store::validate_public_key(p).is_ok() => {
            store::identity_path_in(&env.config_dir, p).exists()
                || store::se_wrapped_path_in(&env.config_dir, p).exists()
        }
        _ => false,
    };
    let legacy_path = project_dir.join(".stand.keys");
    if !any_wrapped && legacy_path.exists() {
        (ctx.warn)(LEGACY_WARNING);
        let key = keys::load_private_key(&legacy_path)?;
        return keys::parse_private_key(&key).map(not_applicable);
    }
    let public_key = public_key.ok_or(CryptoError::NoEncryptionKey)?;
    store::validate_public_key(public_key)?;

    // 3. A terminal is required from here on.
    let ui = match (&ctx.ui, env.force_non_tty) {
        (Some(ui), false) => ui.clone(),
        _ => return Err(CryptoError::NoTerminal),
    };

    // 4. Tell the human what is about to happen before touching any secret.
    ui.show(&purpose.banner());

    let se_wrapped = store::se_wrapped_path_in(&env.config_dir, public_key);
    let se_identity = store::se_identity_path_in(&env.config_dir, public_key);
    let passphrase_file = store::identity_path_in(&env.config_dir, public_key);

    // 5. Touch ID.
    let touch_id_wanted = !env
        .unlock_mode
        .as_deref()
        .map(|m| m.eq_ignore_ascii_case("passphrase"))
        .unwrap_or(false);
    if touch_id_wanted
        && se_wrapped.exists()
        && se_identity.exists()
        && ctx.secure_enclave.is_available()
    {
        if let Some(path) = ctx.secure_enclave.plugin_path() {
            ui.show(&format!(
                "Touch ID will run the age plugin at {}. Cancel if that is not the plugin you installed.",
                sanitize_for_terminal(&path.display().to_string())
            ));
        }
        match try_touch_id(ctx, &se_identity, &se_wrapped, ui.clone()) {
            Ok(plain) => {
                return finish_wrapped(plain, public_key, project_dir, ui.as_ref(), policy)
            }
            Err(reason) => ui.show(&format!(
                "Touch ID unlock failed: {}. Falling back to passphrase.",
                sanitize_for_terminal(&reason)
            )),
        }
    }

    // 6. Passphrase.
    if passphrase_file.exists() {
        let ciphertext = std::fs::read(&passphrase_file)?;
        let plain = unlock_with_passphrase(&ciphertext, ui.as_ref(), env.max_work_factor)?;
        return finish_wrapped(plain, public_key, project_dir, ui.as_ref(), policy);
    }

    // 8. Nothing usable.
    Err(CryptoError::NoPrivateKey {
        hint: format!(
            "expected {}; run `stand encrypt migrate` to create it",
            sanitize_for_terminal(&passphrase_file.display().to_string())
        ),
    })
}

/// An unwrapped identity file: the parsed identity, its recorded project and the raw bytes.
struct UnwrappedIdentity {
    identity: x25519::Identity,
    bound: Option<String>,
    plaintext: SecretSlice<u8>,
}

impl UnwrappedIdentity {
    fn from_plaintext(plaintext: Vec<u8>) -> Result<Self, CryptoError> {
        Ok(Self {
            identity: store::parse_identity_file_contents(&plaintext)?,
            bound: store::parse_project_binding(&plaintext),
            plaintext: SecretSlice::from(plaintext),
        })
    }
}

fn try_touch_id(
    ctx: &UnlockContext,
    se_identity: &Path,
    se_wrapped: &Path,
    ui: Arc<dyn UnlockUi>,
) -> Result<UnwrappedIdentity, String> {
    let identity_file = std::fs::read_to_string(se_identity).map_err(|e| e.to_string())?;
    let ciphertext = std::fs::read(se_wrapped).map_err(|e| e.to_string())?;
    let plaintext = ctx
        .secure_enclave
        .unwrap(&identity_file, &ciphertext, ui)
        .map_err(|e| e.to_string())?;
    UnwrappedIdentity::from_plaintext(plaintext).map_err(|e| e.to_string())
}

fn unlock_with_passphrase(
    ciphertext: &[u8],
    ui: &dyn UnlockUi,
    max_work_factor: Option<u8>,
) -> Result<UnwrappedIdentity, CryptoError> {
    for attempt in 1..=MAX_PASSPHRASE_ATTEMPTS {
        let passphrase: SecretString = ui
            .request_passphrase("Enter passphrase to unlock the stand key:")
            .ok_or_else(|| CryptoError::UnlockFailed("passphrase entry cancelled".to_string()))?;
        match store::unwrap_with_passphrase(ciphertext, &passphrase, max_work_factor) {
            Ok(plain) => return UnwrappedIdentity::from_plaintext(plain),
            Err(UnwrapError::WrongPassphrase) => {
                if attempt < MAX_PASSPHRASE_ATTEMPTS {
                    ui.show("Incorrect passphrase. Try again.");
                }
            }
            Err(UnwrapError::Other(reason)) => {
                return Err(CryptoError::UnlockFailed(sanitize_for_terminal(&reason)))
            }
        }
    }
    Err(CryptoError::UnlockFailed(format!(
        "incorrect passphrase ({} attempts)",
        MAX_PASSPHRASE_ATTEMPTS
    )))
}

/// Checks the identity against the public key and its project binding.
///
/// A mismatching or missing `# project:` line is shown to the human, who must
/// explicitly confirm (default No) for read-only decryption; with
/// [`BindingPolicy::RequireExact`] it is refused without asking. The agent-editable `[encryption].public_key`
/// selects the identity, so this stops a rewritten config from reusing a key
/// that belongs to another project.
fn finish_wrapped(
    plain: UnwrappedIdentity,
    public_key: &str,
    project_dir: &Path,
    ui: &dyn UnlockUi,
    policy: BindingPolicy,
) -> Result<Unlocked, CryptoError> {
    let UnwrappedIdentity {
        identity,
        bound,
        plaintext,
    } = plain;
    let identity = check_identity(identity, public_key)?;
    let current = store::canonical_project_dir(project_dir)?;
    let matches = bound.as_deref().is_some_and(|b| {
        b == current
            || std::fs::canonicalize(b)
                .ok()
                .and_then(|p| p.to_str().map(|s| s == current))
                .unwrap_or(false)
    });
    if matches {
        return Ok(Unlocked {
            identity,
            binding: Binding::Verified(VerifiedProject {
                public_key: public_key.to_string(),
            }),
            wrapped_plaintext: Some(plaintext),
        });
    }

    let bound_shown = bound
        .as_deref()
        .map(sanitize_for_terminal)
        .unwrap_or_else(|| "(none recorded)".to_string());
    let current_shown = sanitize_for_terminal(&current);
    if policy == BindingPolicy::RequireExact {
        return Err(CryptoError::ProjectMismatch {
            bound: bound_shown,
            current: current_shown,
        });
    }
    ui.show(&format!(
        "WARNING: this key was created for project {} but you are unlocking it for {}. \
         A changed .stand.toml may be trying to use a key from another project.",
        bound_shown, current_shown
    ));
    let confirmed = ui
        .confirm("Unlock this key for the current project anyway?")
        .unwrap_or(false);
    if confirmed {
        Ok(Unlocked {
            identity,
            binding: Binding::Confirmed,
            wrapped_plaintext: None,
        })
    } else {
        Err(CryptoError::ProjectMismatch {
            bound: bound_shown,
            current: current_shown,
        })
    }
}

fn check_identity(
    identity: x25519::Identity,
    public_key: &str,
) -> Result<x25519::Identity, CryptoError> {
    if identity.to_public().to_string() == public_key {
        Ok(identity)
    } else {
        Err(CryptoError::IdentityMismatch {
            expected: public_key.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::testing::{MockSe, MockUi};
    use age::secrecy::ExposeSecret;
    use std::sync::Mutex;
    use tempfile::{tempdir, TempDir};

    const WF: u8 = 8;

    struct Fixture {
        _tmp: TempDir,
        project: std::path::PathBuf,
        config: std::path::PathBuf,
        identity: x25519::Identity,
        public_key: String,
    }

    fn fixture() -> Fixture {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        let config = tmp.path().join("config");
        std::fs::create_dir_all(&project).unwrap();
        let identity = x25519::Identity::generate();
        let public_key = identity.to_public().to_string();
        Fixture {
            _tmp: tmp,
            project,
            config,
            identity,
            public_key,
        }
    }

    fn write_pass_wrap(f: &Fixture, pw: &str) {
        store::write_passphrase_identity_in(
            &f.config,
            &f.public_key,
            &f.identity,
            &SecretString::from(pw.to_string()),
            Some(WF),
            &f.project,
        )
        .unwrap();
    }

    fn env_for(f: &Fixture) -> UnlockEnv {
        UnlockEnv {
            ci: None,
            key_source: None,
            private_key: None,
            unlock_mode: None,
            force_non_tty: false,
            config_dir: f.config.clone(),
            max_work_factor: Some(WF),
        }
    }

    fn ctx(
        env: UnlockEnv,
        ui: Option<Arc<MockUi>>,
        se: MockSe,
        warnings: Arc<Mutex<Vec<String>>>,
    ) -> UnlockContext {
        UnlockContext {
            env,
            ui: ui.map(|u| u as Arc<dyn UnlockUi>),
            secure_enclave: Box::new(se),
            warn: Arc::new(move |m| warnings.lock().unwrap().push(m.to_string())),
        }
    }

    fn purpose() -> DecryptPurpose {
        let mut p = DecryptPurpose::new("stand exec dev -- true", Some("dev"));
        p.secret_count = 2;
        p
    }

    fn no_warnings() -> Arc<Mutex<Vec<String>>> {
        Arc::new(Mutex::new(vec![]))
    }

    #[test]
    fn test_passphrase_unlock_succeeds_and_shows_banner_first() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );

        let id =
            unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        assert_eq!(id.to_public().to_string(), f.public_key);
        let shown = ui.shown_messages();
        assert!(shown[0].contains("wants to decrypt 2 secret(s)"));
    }

    #[test]
    fn test_wrong_passphrase_fails_after_three_attempts() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        let ui = MockUi::new(vec![Some("a"), Some("b"), Some("c"), Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );

        let r = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::UnlockFailed(_))));
        assert_eq!(ui.prompt_count(), 3);
    }

    #[test]
    fn test_second_attempt_can_succeed() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        let ui = MockUi::new(vec![Some("bad"), Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui),
            MockSe::new(false, false),
            no_warnings(),
        );
        assert!(
            unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).is_ok()
        );
    }

    #[test]
    fn test_cancelled_passphrase_fails() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        let ui = MockUi::new(vec![None]);
        let c = ctx(
            env_for(&f),
            Some(ui),
            MockSe::new(false, false),
            no_warnings(),
        );
        let r = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::UnlockFailed(_))));
    }

    #[test]
    fn test_public_key_mismatch_is_rejected() {
        let f = fixture();
        // Store a different identity under the project's public key name.
        let other = x25519::Identity::generate();
        store::write_passphrase_identity_in(
            &f.config,
            &f.public_key,
            &other,
            &SecretString::from("pw".to_string()),
            Some(WF),
            &f.project,
        )
        .unwrap();
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui),
            MockSe::new(false, false),
            no_warnings(),
        );
        let r = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::IdentityMismatch { .. })));
    }

    #[test]
    fn test_no_ui_means_no_terminal() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        let c = ctx(env_for(&f), None, MockSe::new(false, false), no_warnings());
        let r = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::NoTerminal)));
    }

    #[test]
    fn test_force_non_tty_ignores_available_ui() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        let mut env = env_for(&f);
        env.force_non_tty = true;
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env,
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );
        let r = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::NoTerminal)));
        assert_eq!(ui.prompt_count(), 0);
    }

    #[test]
    fn test_missing_identity_files_report_expected_path_and_migrate_hint() {
        let f = fixture();
        let ui = MockUi::new(vec![]);
        let c = ctx(
            env_for(&f),
            Some(ui),
            MockSe::new(false, false),
            no_warnings(),
        );
        let err =
            match unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c) {
                Ok(_) => panic!("expected an error"),
                Err(e) => e,
            };
        assert!(matches!(err, CryptoError::NoPrivateKey { .. }));
        let msg = err.to_string();
        assert!(msg.contains(&format!("{}.age", f.public_key)));
        assert!(msg.contains("stand encrypt migrate"));
    }

    #[test]
    fn test_env_key_requires_opt_in() {
        let f = fixture();
        let mut env = env_for(&f);
        env.private_key = Some(f.identity.to_string().expose_secret().to_string());
        let c = ctx(env, None, MockSe::new(false, false), no_warnings());
        let r = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::EnvKeyNotAllowed)));
    }

    #[test]
    fn test_env_key_works_without_tty_with_ci_or_key_source() {
        let f = fixture();
        let key = f.identity.to_string().expose_secret().to_string();
        for (ci, source) in [
            (Some("true"), None),
            (Some("TRUE"), None),
            (Some("1"), None),
            (None, Some("env")),
        ] {
            let mut env = env_for(&f);
            env.private_key = Some(key.clone());
            env.ci = ci.map(str::to_string);
            env.key_source = source.map(str::to_string);
            let c = ctx(env, None, MockSe::new(false, false), no_warnings());
            let id = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c)
                .unwrap();
            assert_eq!(id.to_public().to_string(), f.public_key);
        }
    }

    #[test]
    fn test_ci_false_does_not_opt_in() {
        let f = fixture();
        let mut env = env_for(&f);
        env.private_key = Some(f.identity.to_string().expose_secret().to_string());
        env.ci = Some("false".to_string());
        let c = ctx(env, None, MockSe::new(false, false), no_warnings());
        let r = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::EnvKeyNotAllowed)));
    }

    #[test]
    fn test_legacy_keys_file_works_without_tty_and_warns() {
        let f = fixture();
        keys::save_private_key(
            &f.project.join(".stand.keys"),
            f.identity.to_string().expose_secret(),
        )
        .unwrap();
        let warnings = no_warnings();
        let c = ctx(
            env_for(&f),
            None,
            MockSe::new(false, false),
            warnings.clone(),
        );
        let id =
            unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        assert_eq!(id.to_public().to_string(), f.public_key);
        let w = warnings.lock().unwrap();
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("legacy .stand.keys"));
        assert!(w[0].contains("stand encrypt migrate"));
    }

    #[test]
    fn test_legacy_keys_ignored_once_wrapped_identity_exists() {
        let f = fixture();
        keys::save_private_key(
            &f.project.join(".stand.keys"),
            f.identity.to_string().expose_secret(),
        )
        .unwrap();
        write_pass_wrap(&f, "pw");
        // No UI: must demand a terminal instead of silently reading the legacy file.
        let c = ctx(env_for(&f), None, MockSe::new(false, false), no_warnings());
        let r = unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::NoTerminal)));
    }

    fn write_touch_id(f: &Fixture) {
        let se = MockSe::new(true, false);
        crate::crypto::enable::add_touch_id_in(
            &f.config,
            &se,
            &f.identity,
            &f.public_key,
            &f.project,
        )
        .unwrap();
    }

    #[test]
    fn test_touch_id_is_preferred_over_passphrase() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        write_touch_id(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        let se = MockSe::new(true, false);
        let c = ctx(env_for(&f), Some(ui.clone()), se, no_warnings());
        let id =
            unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        assert_eq!(id.to_public().to_string(), f.public_key);
        assert_eq!(ui.prompt_count(), 0, "passphrase must not be asked");
    }

    #[test]
    fn test_touch_id_failure_falls_back_to_passphrase() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        write_touch_id(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(true, true),
            no_warnings(),
        );
        let id =
            unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        assert_eq!(id.to_public().to_string(), f.public_key);
        let shown = ui.shown_messages().join("\n");
        assert!(shown.contains("Touch ID unlock failed"));
        assert!(shown.contains("Falling back to passphrase"));
        assert_eq!(ui.prompt_count(), 1);
    }

    #[test]
    fn test_touch_id_attempt_shows_resolved_plugin_path() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        write_touch_id(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(true, false),
            no_warnings(),
        );
        unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        let shown = ui.shown_messages();
        let banner_pos = shown
            .iter()
            .position(|m| m.contains("wants to decrypt"))
            .unwrap();
        let path_pos = shown
            .iter()
            .position(|m| m.contains("/mock/bin/age-plugin-se"))
            .expect("plugin path must be shown");
        let touch_pos = shown
            .iter()
            .position(|m| m.contains("Touch ID requested"))
            .unwrap();
        assert!(banner_pos < path_pos && path_pos < touch_pos);
    }

    #[test]
    fn test_plugin_path_not_shown_when_touch_id_is_not_attempted() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        write_touch_id(&f);
        let mut env = env_for(&f);
        env.unlock_mode = Some("passphrase".to_string());
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env,
            Some(ui.clone()),
            MockSe::new(true, false),
            no_warnings(),
        );
        unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        assert!(!ui.shown_messages().join("\n").contains("age-plugin-se"));
    }

    #[test]
    fn test_touch_id_failure_reason_is_sanitized() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        write_touch_id(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        let se = MockSe::new(true, true).with_fail_reason("bad\x1b[2K\rreason\n");
        let c = ctx(env_for(&f), Some(ui.clone()), se, no_warnings());
        unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        let shown = ui.shown_messages().join("\n");
        assert!(!shown.contains('\x1b'));
        assert!(!shown.contains('\r'));
        assert!(shown.contains("bad\\x1b[2K\\rreason\\n"));
    }

    #[test]
    fn test_missing_identity_hint_sanitizes_path() {
        let f = fixture();
        let mut env = env_for(&f);
        env.config_dir = f.config.join("we\x1bird");
        let c = ctx(
            env,
            Some(MockUi::new(vec![])),
            MockSe::new(false, false),
            no_warnings(),
        );
        let err =
            match unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c) {
                Ok(_) => panic!("expected an error"),
                Err(e) => e,
            };
        assert!(!err.to_string().contains('\x1b'));
    }

    #[test]
    fn test_stand_unlock_passphrase_skips_touch_id() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        write_touch_id(&f);
        let mut env = env_for(&f);
        env.unlock_mode = Some("passphrase".to_string());
        let ui = MockUi::new(vec![Some("pw")]);
        let se = Arc::new(MockSe::new(true, false));
        struct Shared(Arc<MockSe>);
        impl SecureEnclaveBackend for Shared {
            fn is_available(&self) -> bool {
                self.0.is_available()
            }
            fn keygen(&self) -> Result<crate::crypto::touch_id::SeKeyMaterial, CryptoError> {
                self.0.keygen()
            }
            fn wrap(&self, r: &str, p: &[u8]) -> Result<Vec<u8>, CryptoError> {
                self.0.wrap(r, p)
            }
            fn unwrap(
                &self,
                i: &str,
                c: &[u8],
                ui: Arc<dyn UnlockUi>,
            ) -> Result<Vec<u8>, CryptoError> {
                self.0.unwrap(i, c, ui)
            }
        }
        let c = UnlockContext {
            env,
            ui: Some(ui.clone() as Arc<dyn UnlockUi>),
            secure_enclave: Box::new(Shared(se.clone())),
            warn: Arc::new(|_| {}),
        };
        unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        assert_eq!(se.unwrap_count(), 0);
        assert_eq!(ui.prompt_count(), 1);
    }

    #[test]
    fn test_unavailable_touch_id_backend_uses_passphrase_silently() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        write_touch_id(&f);
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );
        unlock_project_identity_with(&f.project, Some(&f.public_key), &purpose(), &c).unwrap();
        assert!(!ui.shown_messages().join("\n").contains("Touch ID"));
    }

    // === project binding ===

    /// Writes `<P>.age` bound to a different project directory.
    fn write_pass_wrap_for_other_project(f: &Fixture, pw: &str) -> std::path::PathBuf {
        let other = f._tmp.path().join("other-project");
        std::fs::create_dir_all(&other).unwrap();
        store::write_passphrase_identity_in(
            &f.config,
            &f.public_key,
            &f.identity,
            &SecretString::from(pw.to_string()),
            Some(WF),
            &other,
        )
        .unwrap();
        std::fs::canonicalize(other).unwrap()
    }

    #[test]
    fn test_matching_binding_is_verified_without_confirmation() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );
        let u =
            unlock_project_identity_verified_with(&f.project, Some(&f.public_key), &purpose(), &c)
                .unwrap();
        assert_eq!(u.verified().unwrap().public_key(), f.public_key);
        assert_eq!(ui.confirm_count(), 0);
    }

    #[test]
    fn test_binding_mismatch_declined_refuses_and_names_both_paths() {
        let f = fixture();
        let other = write_pass_wrap_for_other_project(&f, "pw");
        let ui = MockUi::new(vec![Some("pw")]);
        ui.set_confirm(Some(false));
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );
        let r =
            unlock_project_identity_verified_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::ProjectMismatch { .. })));
        assert_eq!(ui.confirm_count(), 1);
        let shown = ui.shown_messages().join("\n");
        assert!(shown.contains(other.to_str().unwrap()));
        assert!(shown.contains(std::fs::canonicalize(&f.project).unwrap().to_str().unwrap()));
    }

    #[test]
    fn test_exact_unlock_refuses_mismatch_without_prompting_and_names_both_paths() {
        let f = fixture();
        let other = write_pass_wrap_for_other_project(&f, "pw");
        let ui = MockUi::new(vec![Some("pw")]);
        ui.set_confirm(Some(true));
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );
        let r = unlock_project_identity_exact_with(&f.project, Some(&f.public_key), &purpose(), &c);
        let msg = match r {
            Err(e @ CryptoError::ProjectMismatch { .. }) => e.to_string(),
            _ => panic!("expected ProjectMismatch"),
        };
        assert_eq!(ui.confirm_count(), 0);
        assert!(msg.contains(other.to_str().unwrap()), "{msg}");
        assert!(msg.contains(std::fs::canonicalize(&f.project).unwrap().to_str().unwrap()));
    }

    #[test]
    fn test_exact_unlock_with_matching_binding_carries_original_plaintext() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        let ui = MockUi::new(vec![Some("pw")]);
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );
        let mut u =
            unlock_project_identity_exact_with(&f.project, Some(&f.public_key), &purpose(), &c)
                .unwrap();
        assert!(u.verified().is_some());
        let bytes = std::fs::read(store::identity_path_in(&f.config, &f.public_key)).unwrap();
        let expected =
            store::unwrap_with_passphrase(&bytes, &SecretString::from("pw".to_string()), Some(WF))
                .unwrap();
        let got = u.take_wrapped_plaintext().unwrap();
        assert_eq!(got.expose_secret(), &expected[..]);
        assert!(u.take_wrapped_plaintext().is_none());
    }

    #[test]
    fn test_binding_mismatch_unanswerable_confirm_refuses() {
        let f = fixture();
        write_pass_wrap_for_other_project(&f, "pw");
        let ui = MockUi::new(vec![Some("pw")]);
        ui.set_confirm(None);
        let c = ctx(
            env_for(&f),
            Some(ui),
            MockSe::new(false, false),
            no_warnings(),
        );
        let r =
            unlock_project_identity_verified_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::ProjectMismatch { .. })));
    }

    #[test]
    fn test_binding_mismatch_confirmed_yields_confirmed_not_verified() {
        let f = fixture();
        write_pass_wrap_for_other_project(&f, "pw");
        let ui = MockUi::new(vec![Some("pw")]);
        ui.set_confirm(Some(true));
        let c = ctx(
            env_for(&f),
            Some(ui),
            MockSe::new(false, false),
            no_warnings(),
        );
        let u =
            unlock_project_identity_verified_with(&f.project, Some(&f.public_key), &purpose(), &c)
                .unwrap();
        assert!(matches!(u.binding, Binding::Confirmed));
        assert!(u.verified().is_none());
    }

    #[test]
    fn test_identity_without_project_line_counts_as_mismatch() {
        let f = fixture();
        let plain = format!(
            "# public key: {}\n{}\n",
            f.public_key,
            f.identity.to_string().expose_secret()
        );
        let wrapped = store::wrap_with_passphrase(
            plain.as_bytes(),
            &SecretString::from("pw".to_string()),
            Some(WF),
        )
        .unwrap();
        store::ensure_identities_dir_in(&f.config).unwrap();
        store::write_private_file(&store::identity_path_in(&f.config, &f.public_key), &wrapped)
            .unwrap();
        let ui = MockUi::new(vec![Some("pw")]);
        ui.set_confirm(Some(false));
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(false, false),
            no_warnings(),
        );
        let r =
            unlock_project_identity_verified_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::ProjectMismatch { .. })));
        assert!(ui.shown_messages().join("\n").contains("none recorded"));
    }

    #[test]
    fn test_touch_id_path_also_checks_binding_and_does_not_fall_back() {
        let f = fixture();
        write_pass_wrap(&f, "pw");
        // SE copy bound to another project.
        let other = f._tmp.path().join("other-project");
        std::fs::create_dir_all(&other).unwrap();
        let se = MockSe::new(true, false);
        crate::crypto::enable::add_touch_id_in(&f.config, &se, &f.identity, &f.public_key, &other)
            .unwrap();
        let ui = MockUi::new(vec![Some("pw")]);
        ui.set_confirm(Some(false));
        let c = ctx(
            env_for(&f),
            Some(ui.clone()),
            MockSe::new(true, false),
            no_warnings(),
        );
        let r =
            unlock_project_identity_verified_with(&f.project, Some(&f.public_key), &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::ProjectMismatch { .. })));
        assert_eq!(ui.prompt_count(), 0, "must not fall back to the passphrase");
    }

    #[test]
    fn test_env_key_and_legacy_paths_are_not_binding_checked() {
        let f = fixture();
        let mut env = env_for(&f);
        env.private_key = Some(f.identity.to_string().expose_secret().to_string());
        env.ci = Some("true".to_string());
        let c = ctx(env, None, MockSe::new(false, false), no_warnings());
        let u =
            unlock_project_identity_verified_with(&f.project, Some(&f.public_key), &purpose(), &c)
                .unwrap();
        assert!(matches!(u.binding, Binding::NotApplicable));
        assert!(u.verified().is_none());
    }

    #[test]
    fn test_no_public_key_and_no_legacy_file_is_no_encryption_key() {
        let f = fixture();
        let c = ctx(env_for(&f), None, MockSe::new(false, false), no_warnings());
        let r = unlock_project_identity_with(&f.project, None, &purpose(), &c);
        assert!(matches!(r, Err(CryptoError::NoEncryptionKey)));
    }
}
