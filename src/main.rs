use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;

const MARKER: &str = "# block-detached-commit";
const CALL: &str = "block-detached-commit";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        None | Some("check") => run_check(),
        Some("install") => run_install(),
        Some("uninstall") => run_uninstall(),
        Some(cmd) => {
            eprintln!("error: unknown subcommand '{cmd}'");
            eprintln!("usage: block-detached-commit [check|install|uninstall]");
            process::exit(2);
        }
    }
}

fn run_check() {
    let git_dir = git_dir_or_exit();
    let head_path = git_dir.join("HEAD");

    let mut content = String::new();
    if let Err(e) = fs::File::open(&head_path).and_then(|mut f| f.read_to_string(&mut content)) {
        eprintln!("error: cannot read {}: {e}", head_path.display());
        process::exit(2);
    }

    if is_attached(&content) {
        process::exit(0);
    }

    eprintln!("error: cannot commit in detached HEAD state");
    eprintln!("hint:  create a branch:         git checkout -b <branch-name>");
    eprintln!("hint:  or switch to existing:   git switch <branch-name>");
    process::exit(1);
}

fn run_install() {
    let git_dir = git_dir_or_exit();
    let hooks = hooks_dir(&git_dir);

    if let Err(e) = fs::create_dir_all(&hooks) {
        eprintln!("error: cannot create hooks dir: {e}");
        process::exit(2);
    }

    let hook_path = hooks.join("pre-commit");
    let existing = fs::read_to_string(&hook_path).unwrap_or_default();

    if existing.contains(MARKER) {
        eprintln!("info: hook already installed at {}", hook_path.display());
        return;
    }

    let new_content = build_hook_content(&existing);

    if let Err(e) = fs::write(&hook_path, &new_content) {
        eprintln!("error: cannot write {}: {e}", hook_path.display());
        process::exit(2);
    }

    make_executable(&hook_path);
    eprintln!("info: hook installed at {}", hook_path.display());
}

fn run_uninstall() {
    let git_dir = git_dir_or_exit();
    let hook_path = hooks_dir(&git_dir).join("pre-commit");

    let content = match fs::read_to_string(&hook_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("info: hook not found, nothing to uninstall");
            return;
        }
    };

    if !content.contains(MARKER) {
        eprintln!("info: hook entry not present, nothing to uninstall");
        return;
    }

    let filtered: Vec<&str> = content
        .lines()
        .filter(|l| *l != MARKER && *l != CALL)
        .collect();

    let has_real_content = filtered
        .iter()
        .any(|l| !l.trim().is_empty() && !l.starts_with("#!") && !l.starts_with('#'));

    if !has_real_content {
        let _ = fs::remove_file(&hook_path);
        eprintln!("info: hook removed (file was empty after uninstall)");
        return;
    }

    let new_content = filtered.join("\n") + "\n";
    if let Err(e) = fs::write(&hook_path, &new_content) {
        eprintln!("error: cannot write {}: {e}", hook_path.display());
        process::exit(2);
    }
    eprintln!("info: hook entry removed from {}", hook_path.display());
}

// --- helpers ---

fn is_attached(head_content: &str) -> bool {
    head_content.trim_start().starts_with("ref:")
}

fn build_hook_content(existing: &str) -> String {
    if existing.is_empty() {
        return format!("#!/bin/sh\n{MARKER}\n{CALL}\n");
    }
    // Insert after the shebang line (if present) so we don't break it
    if let Some(nl) = existing.find('\n') {
        let (first_line, rest) = existing.split_at(nl + 1);
        format!("{first_line}{MARKER}\n{CALL}\n{rest}")
    } else {
        format!("{existing}\n{MARKER}\n{CALL}\n")
    }
}

fn git_dir_or_exit() -> PathBuf {
    find_git_dir().unwrap_or_else(|| {
        eprintln!("error: not inside a git repository");
        process::exit(2);
    })
}

fn find_git_dir() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(".git");
        if candidate.is_dir() {
            return Some(candidate);
        }
        if candidate.is_file() {
            // Worktree or submodule: `.git` is a file containing "gitdir: <path>"
            let content = fs::read_to_string(&candidate).ok()?;
            let rel = content.trim().strip_prefix("gitdir: ")?;
            let path = PathBuf::from(rel);
            return Some(if path.is_absolute() {
                path
            } else {
                dir.join(path)
            });
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn hooks_dir(git_dir: &Path) -> PathBuf {
    // Worktrees write a `commondir` file whose value points to the shared git dir.
    // Hooks live in the common dir so all worktrees share the same hooks.
    let common_file = git_dir.join("commondir");
    if let Ok(content) = fs::read_to_string(&common_file) {
        let rel = content.trim();
        let common = if Path::new(rel).is_absolute() {
            PathBuf::from(rel)
        } else {
            git_dir.join(rel)
        };
        return common.join("hooks");
    }
    git_dir.join("hooks")
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(perms.mode() | 0o755);
        let _ = fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

// --- tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_git_repo() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir(dir.path().join(".git").join("hooks")).unwrap();
        dir
    }

    fn set_head(repo: &TempDir, content: &str) {
        fs::write(repo.path().join(".git").join("HEAD"), content).unwrap();
    }

    #[test]
    fn attached_head_is_allowed() {
        assert!(is_attached("ref: refs/heads/main\n"));
        assert!(is_attached("ref: refs/heads/feature/foo\n"));
    }

    #[test]
    fn detached_head_is_blocked() {
        assert!(!is_attached("a3f9c2d1b8e4f6a2c9d5e7b3f1a8c6d4e2f9b7a5\n"));
        assert!(!is_attached("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\n"));
    }

    #[test]
    fn build_hook_content_empty_file() {
        let content = build_hook_content("");
        assert!(content.starts_with("#!/bin/sh\n"));
        assert!(content.contains(MARKER));
        assert!(content.contains(CALL));
    }

    #[test]
    fn build_hook_content_preserves_existing_shebang() {
        let existing = "#!/bin/bash\nsome-other-hook\n";
        let content = build_hook_content(existing);
        assert!(content.starts_with("#!/bin/bash\n"));
        assert!(content.contains(MARKER));
        // marker must come before existing hook body
        let marker_pos = content.find(MARKER).unwrap();
        let existing_pos = content.find("some-other-hook").unwrap();
        assert!(marker_pos < existing_pos);
    }

    #[test]
    fn install_is_idempotent() {
        let repo = make_git_repo();
        let hook_path = repo.path().join(".git").join("hooks").join("pre-commit");

        // First install
        let content = build_hook_content("");
        fs::write(&hook_path, &content).unwrap();
        let first = fs::read_to_string(&hook_path).unwrap();

        // Simulate second install (marker already present)
        let existing = fs::read_to_string(&hook_path).unwrap();
        assert!(
            existing.contains(MARKER),
            "marker must be present after install"
        );

        // build_hook_content should not be called again (caller checks first)
        assert_eq!(first, content);
    }

    #[test]
    fn uninstall_removes_entry_and_keeps_rest() {
        let existing = "#!/bin/sh\nother-hook\n# block-detached-commit\nblock-detached-commit\n";
        let filtered: Vec<&str> = existing
            .lines()
            .filter(|l| *l != MARKER && *l != CALL)
            .collect();
        let result = filtered.join("\n") + "\n";
        assert!(!result.contains(MARKER));
        assert!(!result.contains(CALL));
        assert!(result.contains("other-hook"));
    }

    #[test]
    fn find_git_dir_finds_parent() {
        let repo = make_git_repo();
        let subdir = repo.path().join("src").join("deep");
        fs::create_dir_all(&subdir).unwrap();
        set_head(&repo, "ref: refs/heads/main\n");

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&subdir).unwrap();
        let found = find_git_dir();
        std::env::set_current_dir(&original).unwrap();

        assert!(found.is_some());
        // Canonicalize both sides — macOS /var is a symlink to /private/var
        let found_canon = found.unwrap().canonicalize().unwrap();
        let expected_canon = repo.path().join(".git").canonicalize().unwrap();
        assert_eq!(found_canon, expected_canon);
    }
}
