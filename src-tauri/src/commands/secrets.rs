use tauri::State;

pub use crate::services::secrets::SecretsState;

/// Store the GitHub personal access token in the operating-system
/// credential store.
///
/// The token travels from the frontend to the backend exactly once, on
/// the way into the credential store. It is never written to
/// localStorage and never returned by any command.
#[tauri::command]
pub fn save_github_token(state: State<'_, SecretsState>, token: String) -> Result<(), String> {
    save_token(state.inner(), &token)
}

/// Remove the GitHub personal access token from the operating-system
/// credential store.
#[tauri::command]
pub fn clear_github_token(state: State<'_, SecretsState>) -> Result<(), String> {
    clear_token(state.inner())
}

/// Whether a GitHub token is configured.
///
/// The answer is a boolean: the secret itself never leaves the backend.
#[tauri::command]
pub fn github_token_configured(state: State<'_, SecretsState>) -> bool {
    token_configured(state.inner())
}

fn save_token(state: &SecretsState, token: &str) -> Result<(), String> {
    state
        .service()
        .save_github_token(token)
        .map_err(|err| err.to_string())
}

fn clear_token(state: &SecretsState) -> Result<(), String> {
    state.service().clear_github_token().map_err(|err| err.to_string())
}

fn token_configured(state: &SecretsState) -> bool {
    state.service().github_token_configured()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::secrets::fake::InMemorySecretStore;
    use crate::services::secrets::GITHUB_TOKEN_KEY;
    use std::sync::Arc;

    fn state() -> (SecretsState, Arc<InMemorySecretStore>) {
        let store = Arc::new(InMemorySecretStore::new());
        (SecretsState::with_store(store.clone()), store)
    }

    #[test]
    fn saves_clears_and_reports_the_configured_state() {
        let (state, store) = state();

        assert!(!token_configured(&state));

        save_token(&state, "ghp_secret").unwrap();
        assert!(token_configured(&state));
        assert_eq!(
            store.entries().get(GITHUB_TOKEN_KEY),
            Some(&"ghp_secret".to_string())
        );

        clear_token(&state).unwrap();
        assert!(!token_configured(&state));
        assert!(store.entries().is_empty());
    }

    #[test]
    fn rejects_an_empty_token_without_storing_anything() {
        let (state, store) = state();

        let err = save_token(&state, "   ").unwrap_err();
        assert_eq!(err, "the secret must not be empty");
        assert!(store.entries().is_empty());
        assert!(!token_configured(&state));
    }

    #[test]
    fn a_store_failure_is_reported_without_the_secret() {
        let (state, store) = state();
        store.fail_with("the credential store is locked");

        let err = save_token(&state, "ghp_secret").unwrap_err();
        assert!(err.contains("the credential store is locked"));
        assert!(!err.contains("ghp_secret"));
    }
}
