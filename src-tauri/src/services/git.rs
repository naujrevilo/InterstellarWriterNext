use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("not a git repository: {0}")]
    NotRepo(String),
    #[error("safe.directory: run `git config --global --add safe.directory '{0}'`")]
    SafeDirectory(String),
    #[error("no changes to commit")]
    NoChanges,
    #[error("merge conflict: {0}")]
    MergeConflict(String),
    #[error("push failed: {0}")]
    PushFailed(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("git: {0}")]
    Git2(#[from] git2::Error),
}

pub type SyncResult = Result<String, GitError>;

/// Remove user information from a remote URL before including it in an
/// error message. A remote may contain credentials in its authority, and
/// those credentials must never be exposed through Git Sync diagnostics.
fn redact_remote_url(remote_url: &str) -> String {
    let Some(scheme_end) = remote_url.find("://") else {
        return remote_url.to_string();
    };
    let prefix = &remote_url[..scheme_end + 3];
    let remainder = &remote_url[scheme_end + 3..];
    let authority_end = remainder.find('/').unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];

    let Some(at) = authority.rfind('@') else {
        return remote_url.to_string();
    };

    format!("{prefix}{}", &remainder[at + 1..])
}

pub fn commit_and_push(repo_path: &Path, token: &str, commit_msg: &str) -> SyncResult {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(e) => {
            if e.code() == git2::ErrorCode::Owner {
                return Err(GitError::SafeDirectory(repo_path.display().to_string()));
            }
            return Err(GitError::NotRepo(repo_path.display().to_string()));
        }
    };

    // Stage all changes
    let mut index = repo.index()?;
    index.add_all(["."], git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    // Check if there are staged changes
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;

    // Try to get HEAD commit
    let head_commit = repo.head().and_then(|h| h.peel_to_commit());

    match head_commit {
        Ok(parent) => {
            // Check if tree is the same as HEAD (no changes)
            if tree_oid == parent.tree_id() {
                return Err(GitError::NoChanges);
            }

            let sig = repo.signature()?;
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                commit_msg,
                &tree,
                &[&parent],
            )?;
        }
        Err(_) => {
            // No HEAD yet — initial commit
            let sig = repo.signature()?;
            repo.commit(Some("HEAD"), &sig, &sig, commit_msg, &tree, &[])?;
        }
    }

    // Identify branch
    let branch = repo.head()?.shorthand().unwrap_or("main").to_string();

    // Fetch
    let mut remote = repo.find_remote("origin")?;
    let remote_url = redact_remote_url(remote.url().unwrap_or("unknown"));

    let mut fetch_options = git2::FetchOptions::new();
    let mut callbacks = git2::RemoteCallbacks::new();
    let token_for_fetch = token.to_string();
    callbacks.credentials(move |_url, username_from_url, _allowed_types| {
        let user = username_from_url.unwrap_or("x-access-token");
        git2::Cred::userpass_plaintext(user, &token_for_fetch)
    });
    fetch_options.remote_callbacks(callbacks);

    if let Err(e) = remote.fetch(&[&branch], Some(&mut fetch_options), None) {
        if e.to_string().contains("403") {
            return Err(GitError::PushFailed(format!(
                "403 fetch denied for {}. Check token permissions.",
                remote_url
            )));
        }
        // Ignore other fetch errors (offline, etc.)
    }

    // Merge (fast-forward or normal)
    if let Ok(fetch_head) = repo.find_reference("FETCH_HEAD") {
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
        let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

        if analysis.is_fast_forward() {
            let refname = format!("refs/heads/{}", branch);
            if let Ok(mut reference) = repo.find_reference(&refname) {
                reference.set_target(fetch_commit.id(), "Fast-forward")?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default().force(),
                ))?;
            }
        } else if analysis.is_normal() {
            repo.merge(&[&fetch_commit], None, None)?;
            if repo.index()?.has_conflicts() {
                repo.cleanup_state()?;
                return Err(GitError::MergeConflict(
                    "Resolve conflicts in terminal then sync again".to_string(),
                ));
            }
            let sig = repo.signature()?;
            let tree_id = repo.index()?.write_tree()?;
            let tree = repo.find_tree(tree_id)?;
            let head_commit = repo.head()?.peel_to_commit()?;
            let merge_commit = repo.find_commit(fetch_commit.id())?;
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Merge remote-tracking branch",
                &tree,
                &[&head_commit, &merge_commit],
            )?;
            repo.cleanup_state()?;
        }
    }

    // Push
    let mut callbacks = git2::RemoteCallbacks::new();
    let token_for_push = token.to_string();
    callbacks.credentials(move |_url, username_from_url, _allowed_types| {
        let user = username_from_url.unwrap_or("x-access-token");
        git2::Cred::userpass_plaintext(user, &token_for_push)
    });

    let mut push_options = git2::PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let ref_spec = format!("refs/heads/{}:refs/heads/{}", branch, branch);
    if let Err(e) = remote.push(&[&ref_spec], Some(&mut push_options)) {
        let err_msg = e.to_string();
        if err_msg.contains("403") {
            return Err(GitError::PushFailed(format!(
                "403 push denied for {}. Check token 'repo' scope.",
                remote_url
            )));
        } else if err_msg.contains("401") {
            return Err(GitError::PushFailed(
                "401 unauthorized. Token invalid or expired.".to_string(),
            ));
        }
        return Err(GitError::PushFailed(err_msg));
    }

    Ok("✅ Sincronizado correctamente".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn redacts_credentials_from_remote_urls() {
        assert_eq!(
            redact_remote_url("https://user:secret@example.com/repo.git"),
            "https://example.com/repo.git"
        );
        assert_eq!(
            redact_remote_url("ssh://git@example.com/repo.git"),
            "ssh://example.com/repo.git"
        );
        assert_eq!(
            redact_remote_url("https://example.com/repo.git"),
            "https://example.com/repo.git"
        );
    }

    #[test]
    fn not_a_repo_returns_error() {
        let dir = tempdir().unwrap();
        let result = commit_and_push(dir.path(), "token", "msg");
        assert!(matches!(result, Err(GitError::NotRepo(_))));
    }

    #[test]
    fn no_changes_returns_error() {
        let dir = tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        // Create initial commit so HEAD exists
        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();

        // Now try to sync with no changes
        let result = commit_and_push(dir.path(), "token", "msg");
        assert!(matches!(result, Err(GitError::NoChanges)));
    }

    /// Local, deterministic proof of the push path.
    ///
    /// The test builds a bare remote on the local filesystem, a normal
    /// repository with a repository-local identity, and then calls the real
    /// service. No network, no GitHub and no global Git configuration are
    /// involved: the remote branch must receive the commit produced by
    /// `commit_and_push`.
    #[test]
    fn commit_and_push_reaches_a_local_bare_remote() {
        let scratch = tempdir().unwrap();
        let remote_path = scratch.path().join("remote.git");
        let work_path = scratch.path().join("workspace");

        // Local bare remote: the destination of the push.
        git2::Repository::init_bare(&remote_path).unwrap();
        fs::create_dir_all(&work_path).unwrap();

        // Normal repository with a deterministic branch name.
        let mut init_options = git2::RepositoryInitOptions::new();
        init_options.initial_head("refs/heads/main");
        let repo = git2::Repository::init_opts(&work_path, &init_options).unwrap();

        // Repository-local identity: the test must not depend on the global
        // Git configuration of the machine that runs it.
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Interstellar Test").unwrap();
            config
                .set_str("user.email", "test@interstellar.invalid")
                .unwrap();
        }

        // Initial commit, published so the remote branch already exists.
        fs::write(work_path.join("notes.md"), "# Notes\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_all(["."], git2::IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = repo.signature().unwrap();
        let initial_commit = repo
            .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        let remote_url = local_remote_url(&remote_path);
        let mut remote = repo.remote("origin", &remote_url).unwrap();
        remote
            .push(&["refs/heads/main:refs/heads/main"], None)
            .unwrap();

        // Modify the worktree: the service must commit and push this change.
        fs::write(
            work_path.join("notes.md"),
            "# Notes\n\nCambio local.\n",
        )
        .unwrap();

        let result = commit_and_push(&work_path, "local-bare-remote-token", "Actualizar notas");
        assert!(
            result.is_ok(),
            "el servicio falló contra el remoto local: {:?}",
            result.err()
        );

        // The commit must have reached the remote branch.
        let remote_repo = git2::Repository::open_bare(&remote_path).unwrap();
        let remote_ref = remote_repo.find_reference("refs/heads/main").unwrap();
        let remote_commit = remote_ref.peel_to_commit().unwrap();

        assert_eq!(
            remote_commit.message().unwrap().trim(),
            "Actualizar notas",
            "el remoto no recibió el mensaje del servicio"
        );
        assert_eq!(
            remote_commit.parent_id(0).unwrap(),
            initial_commit,
            "el commit remoto no es hijo del commit inicial"
        );

        let tree = remote_commit.tree().unwrap();
        let entry = tree.get_path(Path::new("notes.md")).unwrap();
        let blob = remote_repo.find_blob(entry.id()).unwrap();
        assert!(
            String::from_utf8_lossy(blob.content()).contains("Cambio local."),
            "el remoto no recibió el contenido modificado"
        );
    }

    /// Render a local path as a remote URL that libgit2 accepts on every
    /// platform (forward slashes, no `file://` scheme).
    fn local_remote_url(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}
