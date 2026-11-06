use std::path::Path;

use serde::Serialize;
use tauri::State;

use crate::commands::workspace::WorkspaceState;
use crate::services::git::{commit_and_push, GitError, SyncResult};
use crate::services::secrets::{SecretError, SecretService, SecretsState};
use crate::services::security::validate_repo_path;

/// Reason reported when no GitHub token is stored in the credential store.
const NO_TOKEN_REASON: &str = "No GitHub token configured. Open Settings to add one.";

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("git: {0}")]
    Git(#[from] GitError),
}

impl serde::Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Tauri-facing wrapper around `services::git::commit_and_push`.
///
/// The GitHub token is not sent by the frontend: the backend reads it
/// from the operating-system credential store and uses it internally.
///
/// Returns a structured result so the frontend can distinguish a
/// successful push from a no-op (nothing to commit) and from a real
/// failure (no token, 401, 403, conflict, etc.). The Rust side just
/// reports; the UI decides how to render each state.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncOutcome {
    /// Commit + push completed. `message` is the localized confirmation.
    Synced { message: String },
    /// Repo was clean — nothing to commit. Still considered a
    /// "successful" call, but we report it so the UI can show
    /// "Up to date" instead of "Synced".
    NoChanges,
    /// Push is blocked because of a 401/403. The UI should bounce
    /// the user to the settings dialog to fix the token.
    AuthFailed { reason: String },
    /// 403 specifically — token is valid but lacks the `repo` scope.
    ScopeMissing { reason: String },
    /// Local file differs from remote; user has to resolve in a
    /// terminal. The UI surfaces the message verbatim.
    Conflict { reason: String },
    /// Anything else (offline, not a repo, safe.directory, etc.).
    Failed { reason: String },
}

#[tauri::command]
pub fn sync_to_github(
    state: State<'_, WorkspaceState>,
    secrets: State<'_, SecretsState>,
    repo_path: String,
    commit_message: String,
) -> SyncOutcome {
    let root = match state.root() {
        Ok(root) => root,
        Err(err) => {
            return SyncOutcome::Failed {
                reason: err.to_string(),
            }
        }
    };

    sync_to_github_within_root(&root, Path::new(&repo_path), &commit_message, secrets.service())
}

fn sync_to_github_within_root(
    root: &Path,
    repo_path: &Path,
    commit_message: &str,
    secrets: &SecretService,
) -> SyncOutcome {
    let repo_path = match validate_repo_path(root, repo_path) {
        Ok(repo_path) => repo_path,
        Err(err) => {
            return SyncOutcome::Failed {
                reason: err.to_string(),
            }
        }
    };

    // The token is read from the credential store only after the path
    // boundary is checked, so an outside path keeps reporting the path
    // failure it reported before.
    let token = match secrets.github_token() {
        Ok(token) if !token.trim().is_empty() => token,
        Ok(_) | Err(SecretError::NotFound(_)) | Err(SecretError::Empty) => {
            return SyncOutcome::AuthFailed {
                reason: NO_TOKEN_REASON.to_string(),
            }
        }
        Err(err) => {
            return SyncOutcome::Failed {
                reason: format!(
                    "could not read the GitHub token from the system credential store: {err}"
                ),
            }
        }
    };

    let result: SyncResult = commit_and_push(&repo_path, &token, commit_message);
    match result {
        Ok(message) => SyncOutcome::Synced { message },
        Err(GitError::NoChanges) => SyncOutcome::NoChanges,
        Err(GitError::PushFailed(reason)) if reason.contains("401") => {
            SyncOutcome::AuthFailed { reason }
        }
        Err(GitError::PushFailed(reason)) if reason.contains("403") => {
            SyncOutcome::ScopeMissing { reason }
        }
        Err(GitError::MergeConflict(reason)) => SyncOutcome::Conflict { reason },
        Err(err) => SyncOutcome::Failed {
            reason: err.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::secrets::fake::InMemorySecretStore;
    use crate::services::secrets::{SecretStore, GITHUB_TOKEN_KEY};
    use git2::Status;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn init_repo_with_initial_commit(path: &Path) -> git2::Repository {
        let repo = git2::Repository::init(path).unwrap();
        fs::write(path.join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_all(["."], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let signature = git2::Signature::now("Test", "test@example.com").unwrap();
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "initial",
                &tree,
                &[],
            )
            .unwrap();
        }
        repo
    }

    /// A service backed by the in-memory store with a token already saved.
    fn service_with_token(token: &str) -> SecretService {
        let store = Arc::new(InMemorySecretStore::new());
        store.set_secret(GITHUB_TOKEN_KEY, token).unwrap();
        SecretService::new(store)
    }

    /// A service backed by the in-memory store with nothing stored.
    fn service_without_token() -> SecretService {
        SecretService::new(Arc::new(InMemorySecretStore::new()))
    }

    #[test]
    fn rejects_an_outside_repo_path_without_mutating_the_repository() {
        let workspace = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let repo = init_repo_with_initial_commit(outside.path());
        let head_before = repo.head().unwrap().target().unwrap();

        fs::write(outside.path().join("draft.md"), "uncommitted").unwrap();

        let outcome = sync_to_github_within_root(
            workspace.path(),
            outside.path(),
            "message",
            &service_with_token("token"),
        );

        match outcome {
            SyncOutcome::Failed { reason } => {
                assert_eq!(reason, "path outside workspace root")
            }
            other => panic!("expected a path validation failure, got {other:?}"),
        }

        let repo_after = git2::Repository::open(outside.path()).unwrap();
        assert_eq!(repo_after.head().unwrap().target().unwrap(), head_before);
        assert!(repo_after
            .statuses(None)
            .unwrap()
            .iter()
            .any(|entry| entry.status() == Status::WT_NEW));
    }

    #[test]
    fn keeps_the_no_changes_outcome_for_the_workspace_repository() {
        let workspace = tempdir().unwrap();
        init_repo_with_initial_commit(workspace.path());

        let outcome = sync_to_github_within_root(
            workspace.path(),
            workspace.path(),
            "message",
            &service_with_token("token"),
        );

        assert!(matches!(outcome, SyncOutcome::NoChanges));
    }

    #[test]
    fn reports_auth_failed_when_no_token_is_stored() {
        let workspace = tempdir().unwrap();
        init_repo_with_initial_commit(workspace.path());

        let outcome = sync_to_github_within_root(
            workspace.path(),
            workspace.path(),
            "message",
            &service_without_token(),
        );

        match outcome {
            SyncOutcome::AuthFailed { reason } => assert_eq!(reason, NO_TOKEN_REASON),
            other => panic!("expected an auth failure, got {other:?}"),
        }
    }

    #[test]
    fn reports_failed_when_the_credential_store_cannot_be_read() {
        let workspace = tempdir().unwrap();
        init_repo_with_initial_commit(workspace.path());

        let store = Arc::new(InMemorySecretStore::new());
        store.fail_with("the credential store is locked");
        let secrets = SecretService::new(store);

        let outcome = sync_to_github_within_root(
            workspace.path(),
            workspace.path(),
            "message",
            &secrets,
        );

        match outcome {
            SyncOutcome::Failed { reason } => {
                assert!(reason.contains("credential store"), "reason: {reason}");
                assert!(!reason.contains("ghp_secret"), "reason: {reason}");
            }
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn the_outside_path_check_comes_before_the_token_lookup() {
        let workspace = tempdir().unwrap();
        let outside = tempdir().unwrap();
        init_repo_with_initial_commit(outside.path());

        let outcome = sync_to_github_within_root(
            workspace.path(),
            outside.path(),
            "message",
            &service_without_token(),
        );

        match outcome {
            SyncOutcome::Failed { reason } => {
                assert_eq!(reason, "path outside workspace root")
            }
            other => panic!("expected a path validation failure, got {other:?}"),
        }
    }
}
