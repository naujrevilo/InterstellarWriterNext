//! Secret storage backed by the operating-system credential store.
//!
//! The GitHub personal access token never lives in the frontend. It is
//! written to the OS credential store (Windows Credential Manager,
//! macOS Keychain, Secret Service on Linux) and read back by the backend
//! only when a sync needs it. The frontend only ever learns *whether* a
//! token is configured — never its value.

use std::sync::Arc;

use thiserror::Error;

/// Key under which the GitHub personal access token is stored.
pub const GITHUB_TOKEN_KEY: &str = "github-token";

/// Service name used for every credential this application writes.
const SERVICE_NAME: &str = "interstellar-writer-next";

/// Failures that the secret store can produce.
///
/// No variant ever carries the secret itself: `Store` only holds the
/// credential store's own error message.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecretError {
    /// The caller tried to store an empty secret.
    #[error("the secret must not be empty")]
    Empty,
    /// No credential is stored under the requested key.
    #[error("no credential stored for '{0}'")]
    NotFound(String),
    /// The credential store itself failed.
    #[error("credential store error: {0}")]
    Store(String),
}

/// A credential store the application can write secrets to.
///
/// The trait exists so the unit tests can run against an in-memory
/// double instead of the real operating-system credential store.
pub trait SecretStore: Send + Sync {
    /// Store `secret` under `key`, replacing any previous value.
    fn set_secret(&self, key: &str, secret: &str) -> Result<(), SecretError>;
    /// Read the secret stored under `key`.
    fn get_secret(&self, key: &str) -> Result<String, SecretError>;
    /// Remove the secret stored under `key`.
    fn clear_secret(&self, key: &str) -> Result<(), SecretError>;
}

/// The real credential store of the host operating system.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsSecretStore;

impl OsSecretStore {
    fn entry(key: &str) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(SERVICE_NAME, key).map_err(|err| SecretError::Store(err.to_string()))
    }
}

/// Translate a credential-store error into our own error model.
///
/// A missing entry is not a failure of the store: it means "no token
/// configured yet", which the caller reports differently from a broken
/// credential store.
fn map_store_error(key: &str, err: keyring::Error) -> SecretError {
    match err {
        keyring::Error::NoEntry => SecretError::NotFound(key.to_string()),
        other => SecretError::Store(other.to_string()),
    }
}

impl SecretStore for OsSecretStore {
    fn set_secret(&self, key: &str, secret: &str) -> Result<(), SecretError> {
        let entry = Self::entry(key)?;
        entry
            .set_password(secret)
            .map_err(|err| SecretError::Store(err.to_string()))
    }

    fn get_secret(&self, key: &str) -> Result<String, SecretError> {
        let entry = Self::entry(key)?;
        entry.get_password().map_err(|err| map_store_error(key, err))
    }

    fn clear_secret(&self, key: &str) -> Result<(), SecretError> {
        let entry = Self::entry(key)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            // Clearing a credential that is not there is a no-op, not a
            // failure: the desired end state already holds.
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(SecretError::Store(err.to_string())),
        }
    }
}

/// Application-level rules on top of a credential store.
pub struct SecretService {
    store: Arc<dyn SecretStore>,
}

impl SecretService {
    /// Build a service on top of the host operating-system credential store.
    pub fn os() -> Self {
        Self::new(Arc::new(OsSecretStore))
    }

    /// Build a service on top of the given store.
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self { store }
    }

    /// Store the GitHub personal access token.
    ///
    /// An empty token is rejected: it would silently disable the sync
    /// while looking like a successful save.
    pub fn save_github_token(&self, token: &str) -> Result<(), SecretError> {
        let token = token.trim();
        if token.is_empty() {
            return Err(SecretError::Empty);
        }
        self.store.set_secret(GITHUB_TOKEN_KEY, token)
    }

    /// Remove the stored GitHub personal access token.
    pub fn clear_github_token(&self) -> Result<(), SecretError> {
        self.store.clear_secret(GITHUB_TOKEN_KEY)
    }

    /// Read the stored GitHub personal access token.
    pub fn github_token(&self) -> Result<String, SecretError> {
        self.store.get_secret(GITHUB_TOKEN_KEY)
    }

    /// Whether a GitHub token is configured — a boolean, never the secret.
    pub fn github_token_configured(&self) -> bool {
        matches!(self.github_token(), Ok(token) if !token.trim().is_empty())
    }
}

/// State shared with the Tauri command layer.
#[derive(Clone)]
pub struct SecretsState {
    service: Arc<SecretService>,
}

impl SecretsState {
    /// State backed by the host operating-system credential store.
    pub fn new() -> Self {
        Self {
            service: Arc::new(SecretService::os()),
        }
    }

    /// State backed by an arbitrary store (used by the unit tests).
    pub fn with_store(store: Arc<dyn SecretStore>) -> Self {
        Self {
            service: Arc::new(SecretService::new(store)),
        }
    }

    /// The secret service behind this state.
    pub fn service(&self) -> &SecretService {
        &self.service
    }
}

impl Default for SecretsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Test double for the credential store.
///
/// It keeps every secret in memory, so the unit tests never touch the
/// real operating-system credential store.
#[cfg(test)]
pub mod fake {
    use super::{SecretError, SecretStore};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct InMemorySecretStore {
        entries: Mutex<HashMap<String, String>>,
        failure: Mutex<Option<String>>,
    }

    impl InMemorySecretStore {
        pub fn new() -> Self {
            Self::default()
        }

        /// Make every operation fail with a credential-store error.
        pub fn fail_with(&self, reason: &str) {
            *self.failure.lock().unwrap() = Some(reason.to_string());
        }

        /// Snapshot of the stored entries, for assertions.
        pub fn entries(&self) -> HashMap<String, String> {
            self.entries.lock().unwrap().clone()
        }
    }

    impl SecretStore for InMemorySecretStore {
        fn set_secret(&self, key: &str, secret: &str) -> Result<(), SecretError> {
            if let Some(reason) = self.failure.lock().unwrap().clone() {
                return Err(SecretError::Store(reason));
            }
            self.entries
                .lock()
                .unwrap()
                .insert(key.to_string(), secret.to_string());
            Ok(())
        }

        fn get_secret(&self, key: &str) -> Result<String, SecretError> {
            if let Some(reason) = self.failure.lock().unwrap().clone() {
                return Err(SecretError::Store(reason));
            }
            self.entries
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| SecretError::NotFound(key.to_string()))
        }

        fn clear_secret(&self, key: &str) -> Result<(), SecretError> {
            if let Some(reason) = self.failure.lock().unwrap().clone() {
                return Err(SecretError::Store(reason));
            }
            self.entries.lock().unwrap().remove(key);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::InMemorySecretStore;
    use super::*;
    use std::sync::Arc;

    fn service() -> (SecretService, Arc<InMemorySecretStore>) {
        let store = Arc::new(InMemorySecretStore::new());
        (SecretService::new(store.clone()), store)
    }

    #[test]
    fn stores_reads_back_and_clears_the_github_token() {
        let (secrets, store) = service();

        secrets.save_github_token("ghp_secret").unwrap();
        assert_eq!(secrets.github_token().unwrap(), "ghp_secret");
        assert_eq!(
            store.entries().get(GITHUB_TOKEN_KEY).map(String::as_str),
            Some("ghp_secret")
        );

        secrets.clear_github_token().unwrap();
        assert!(store.entries().is_empty());
        assert!(matches!(
            secrets.github_token(),
            Err(SecretError::NotFound(_))
        ));
    }

    #[test]
    fn reports_a_missing_credential() {
        let (secrets, _) = service();

        assert!(matches!(
            secrets.github_token(),
            Err(SecretError::NotFound(_))
        ));
        assert!(!secrets.github_token_configured());
    }

    #[test]
    fn rejects_an_empty_token() {
        let (secrets, store) = service();

        assert!(matches!(
            secrets.save_github_token(""),
            Err(SecretError::Empty)
        ));
        assert!(matches!(
            secrets.save_github_token("   "),
            Err(SecretError::Empty)
        ));
        assert!(store.entries().is_empty());
    }

    #[test]
    fn reports_the_configured_state_without_exposing_the_secret() {
        let (secrets, _) = service();

        assert!(!secrets.github_token_configured());
        secrets.save_github_token("ghp_secret").unwrap();
        assert!(secrets.github_token_configured());
    }

    #[test]
    fn maps_a_missing_credential_entry_to_not_found() {
        // Pure mapping check — it never opens the real credential store.
        assert_eq!(
            map_store_error(GITHUB_TOKEN_KEY, keyring::Error::NoEntry),
            SecretError::NotFound(GITHUB_TOKEN_KEY.to_string())
        );
        assert!(matches!(
            map_store_error(GITHUB_TOKEN_KEY, keyring::Error::NoDefaultStore),
            SecretError::Store(_)
        ));
    }

    #[test]
    fn a_store_failure_never_carries_the_secret() {
        let (secrets, store) = service();
        store.fail_with("the credential store is locked");

        let err = secrets.save_github_token("ghp_secret").unwrap_err();
        assert!(matches!(err, SecretError::Store(_)));
        assert!(!err.to_string().contains("ghp_secret"));

        let err = secrets.github_token().unwrap_err();
        assert!(matches!(err, SecretError::Store(_)));
        assert!(!err.to_string().contains("ghp_secret"));
    }
}

/// Probe against the real operating-system credential store.
///
/// This test is `#[ignore]`d on purpose: the normal suite and the hosted CI
/// must never touch the credential store of the machine that runs them. It is
/// an environment probe, not a unit test, and it is executed explicitly on a
/// single host with:
///
/// ```text
/// cargo test --manifest-path src-tauri/Cargo.toml --locked -- --ignored
/// ```
///
/// The probe never uses the real `github-token` key: it writes under a unique
/// temporary account of the application service name and always deletes it,
/// even when an assertion panics.
#[cfg(test)]
mod os_store_probe {
    use super::{OsSecretStore, SecretError, SecretStore, GITHUB_TOKEN_KEY};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Distinguishes probes of concurrent executions.
    static PROBE_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Removes the probe credential when the test ends, even on panic.
    struct ProbeCleanup {
        key: String,
    }

    impl Drop for ProbeCleanup {
        fn drop(&mut self) {
            // Best effort: the probe must never survive the test.
            let _ = OsSecretStore.clear_secret(&self.key);
        }
    }

    /// Unique account key for this probe. It never collides with the real
    /// GitHub token key and never repeats across executions.
    fn probe_key() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!(
            "probe-{}-{}-{}",
            std::process::id(),
            nanos,
            PROBE_COUNTER.fetch_add(1, Ordering::Relaxed)
        )
    }

    #[test]
    #[ignore = "touches the real OS credential store; run explicitly on one host"]
    fn os_credential_store_round_trip() {
        let key = probe_key();
        assert_ne!(
            key, GITHUB_TOKEN_KEY,
            "el probe nunca debe usar la clave real del token"
        );

        // The guard deletes the probe even if an assertion below panics.
        let _cleanup = ProbeCleanup { key: key.clone() };
        let store = OsSecretStore;

        // A non-secret probe value. It is never printed.
        let probe = "probe-not-a-secret";

        store
            .set_secret(&key, probe)
            .expect("el almacén de credenciales del sistema operativo rechazó la escritura");

        let read_back = store
            .get_secret(&key)
            .expect("el almacén de credenciales del sistema operativo rechazó la lectura");
        assert!(
            read_back == probe,
            "el valor leído no coincide con el valor escrito"
        );

        store
            .clear_secret(&key)
            .expect("no se pudo eliminar la credencial de prueba");

        assert!(
            matches!(store.get_secret(&key), Err(SecretError::NotFound(_))),
            "la credencial de prueba sigue presente después de eliminarla"
        );
    }
}
