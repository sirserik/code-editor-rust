use git2::{Repository, StatusOptions, StatusShow};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub branch: String,
    pub files: Vec<GitFileStatus>,
    pub is_repo: bool,
}

#[derive(Debug, Clone)]
pub struct GitFileStatus {
    pub path: String,
    pub status: FileStatus,
    pub staged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

impl FileStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            FileStatus::Modified => "M",
            FileStatus::Added => "A",
            FileStatus::Deleted => "D",
            FileStatus::Renamed => "R",
            FileStatus::Untracked => "?",
        }
    }
}

struct CachedStatus {
    status: GitStatus,
    timestamp: Instant,
}

pub struct GitManager {
    cache: HashMap<String, CachedStatus>,
}

const CACHE_TTL_MS: u64 = 2000;

impl GitManager {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn get_status(&mut self, repo_path: &str) -> GitStatus {
        // Check cache
        if let Some(cached) = self.cache.get(repo_path) {
            if cached.timestamp.elapsed().as_millis() < CACHE_TTL_MS as u128 {
                return cached.status.clone();
            }
        }

        let status = self.fetch_status(repo_path);
        self.cache.insert(
            repo_path.to_string(),
            CachedStatus {
                status: status.clone(),
                timestamp: Instant::now(),
            },
        );
        status
    }

    pub fn invalidate_cache(&mut self, repo_path: &str) {
        self.cache.remove(repo_path);
    }

    fn fetch_status(&self, repo_path: &str) -> GitStatus {
        let repo = match Repository::discover(repo_path) {
            Ok(r) => r,
            Err(_) => {
                return GitStatus {
                    branch: String::new(),
                    files: Vec::new(),
                    is_repo: false,
                }
            }
        };

        let branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()))
            .unwrap_or_else(|| "HEAD".to_string());

        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .show(StatusShow::IndexAndWorkdir);

        let statuses = match repo.statuses(Some(&mut opts)) {
            Ok(s) => s,
            Err(_) => {
                return GitStatus {
                    branch,
                    files: Vec::new(),
                    is_repo: true,
                }
            }
        };

        let mut files = Vec::new();

        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();

            // Index (staged) status — one entry per file
            if status.is_index_new() {
                files.push(GitFileStatus { path: path.clone(), status: FileStatus::Added, staged: true });
            } else if status.is_index_modified() {
                files.push(GitFileStatus { path: path.clone(), status: FileStatus::Modified, staged: true });
            } else if status.is_index_deleted() {
                files.push(GitFileStatus { path: path.clone(), status: FileStatus::Deleted, staged: true });
            } else if status.is_index_renamed() {
                files.push(GitFileStatus { path: path.clone(), status: FileStatus::Renamed, staged: true });
            }

            // Working tree (unstaged) status — one entry per file
            if status.is_wt_modified() {
                files.push(GitFileStatus { path: path.clone(), status: FileStatus::Modified, staged: false });
            } else if status.is_wt_new() {
                files.push(GitFileStatus { path: path.clone(), status: FileStatus::Untracked, staged: false });
            } else if status.is_wt_deleted() {
                files.push(GitFileStatus { path: path.clone(), status: FileStatus::Deleted, staged: false });
            }
        }

        GitStatus {
            branch,
            files,
            is_repo: true,
        }
    }

    pub fn stage_file(&mut self, repo_path: &str, file_path: &str) -> Result<(), String> {
        let repo = Repository::discover(repo_path).map_err(|e| e.to_string())?;
        let mut index = repo.index().map_err(|e| e.to_string())?;
        index
            .add_path(std::path::Path::new(file_path))
            .map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;
        self.invalidate_cache(repo_path);
        Ok(())
    }

    pub fn unstage_file(&mut self, repo_path: &str, file_path: &str) -> Result<(), String> {
        let repo = Repository::discover(repo_path).map_err(|e| e.to_string())?;
        let head = repo.head().map_err(|e| e.to_string())?;
        let target = head.target().ok_or("No HEAD target")?;
        let commit = repo.find_commit(target).map_err(|e| e.to_string())?;
        repo.reset_default(Some(commit.as_object()), [file_path])
            .map_err(|e| e.to_string())?;
        self.invalidate_cache(repo_path);
        Ok(())
    }

    pub fn stage_all(&mut self, repo_path: &str) -> Result<(), String> {
        let repo = Repository::discover(repo_path).map_err(|e| e.to_string())?;
        let mut index = repo.index().map_err(|e| e.to_string())?;
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;
        self.invalidate_cache(repo_path);
        Ok(())
    }

    pub fn commit(&mut self, repo_path: &str, message: &str) -> Result<(), String> {
        let repo = Repository::discover(repo_path).map_err(|e| e.to_string())?;
        let sig = repo.signature().map_err(|e| e.to_string())?;
        let mut index = repo.index().map_err(|e| e.to_string())?;
        let tree_id = index.write_tree().map_err(|e| e.to_string())?;
        let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;

        let parent = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|t| repo.find_commit(t).ok());

        let parents: Vec<&git2::Commit> = parent.iter().collect();

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .map_err(|e| e.to_string())?;

        self.invalidate_cache(repo_path);
        Ok(())
    }

    pub fn blame_line(&self, repo_path: &str, file_path: &str, line: usize) -> Option<String> {
        let repo = Repository::discover(repo_path).ok()?;
        let blame = repo.blame_file(std::path::Path::new(file_path), None).ok()?;
        let hunk = blame.get_line(line + 1)?; // git blame is 1-indexed
        let sig = hunk.final_signature();
        let name = sig.name().unwrap_or("unknown");
        let commit_id = hunk.final_commit_id();
        let short_id = &commit_id.to_string()[..7];
        // Get commit time
        let commit = repo.find_commit(commit_id).ok()?;
        let time = commit.time();
        let secs = time.seconds();
        // Format as relative time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let diff = now - secs;
        let age = if diff < 60 { "just now".to_string() }
            else if diff < 3600 { format!("{}m ago", diff / 60) }
            else if diff < 86400 { format!("{}h ago", diff / 3600) }
            else if diff < 2592000 { format!("{}d ago", diff / 86400) }
            else if diff < 31536000 { format!("{}mo ago", diff / 2592000) }
            else { format!("{}y ago", diff / 31536000) };
        Some(format!("{} {} • {}", short_id, name, age))
    }

    pub fn discard_file(&mut self, repo_path: &str, file_path: &str) -> Result<(), String> {
        let repo = Repository::discover(repo_path).map_err(|e| e.to_string())?;
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.path(file_path).force();
        repo.checkout_head(Some(&mut checkout))
            .map_err(|e| e.to_string())?;
        self.invalidate_cache(repo_path);
        Ok(())
    }

    /// Run a git CLI subcommand in the repo. We shell out to the system `git` for network
    /// ops (pull/push/fetch) because libgit2 auth (SSH keys, credential helper, 2FA tokens)
    /// is a maze; the user's `git` config "just works" with whatever they've already set up.
    pub fn run_cli(repo_path: &str, args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new("git")
            .arg("-C").arg(repo_path)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to run git: {}", e))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }

    /// List local + remote branches. Returns sorted: current first, then local, then remote.
    pub fn list_branches(repo_path: &str) -> Vec<BranchInfo> {
        let repo = match Repository::discover(repo_path) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        let head_name = repo.head().ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));
        // Local
        if let Ok(iter) = repo.branches(Some(git2::BranchType::Local)) {
            for b in iter.flatten() {
                let (branch, _) = b;
                if let Ok(Some(name)) = branch.name() {
                    let is_current = head_name.as_deref() == Some(name);
                    out.push(BranchInfo { name: name.to_string(), is_remote: false, is_current });
                }
            }
        }
        // Remote
        if let Ok(iter) = repo.branches(Some(git2::BranchType::Remote)) {
            for b in iter.flatten() {
                let (branch, _) = b;
                if let Ok(Some(name)) = branch.name() {
                    // skip the HEAD symref like "origin/HEAD"
                    if name.ends_with("/HEAD") { continue; }
                    out.push(BranchInfo { name: name.to_string(), is_remote: true, is_current: false });
                }
            }
        }
        out.sort_by(|a, b| {
            b.is_current.cmp(&a.is_current)
                .then_with(|| a.is_remote.cmp(&b.is_remote))
                .then_with(|| a.name.cmp(&b.name))
        });
        out
    }

    /// Checkout an existing branch. For remote branches like `origin/feat`, creates a local
    /// tracking branch `feat` automatically (matches `git checkout` behaviour).
    pub fn checkout_branch(repo_path: &str, branch: &str) -> Result<(), String> {
        // If user picked a remote branch (e.g. "origin/foo/bar"), strip the remote prefix
        // for the local name (-> "foo/bar"). `git checkout <name>` then creates the local
        // tracking branch automatically when one doesn't exist.
        let is_remote_branch = Repository::discover(repo_path)
            .ok()
            .map(|repo| repo.find_branch(branch, git2::BranchType::Remote).is_ok())
            .unwrap_or(false);
        let local_name: &str = if is_remote_branch {
            match branch.find('/') {
                Some(idx) => &branch[idx + 1..],
                None => branch,
            }
        } else {
            branch
        };
        Self::run_cli(repo_path, &["checkout", local_name]).map(|_| ())
    }

    pub fn create_branch(repo_path: &str, name: &str) -> Result<(), String> {
        Self::run_cli(repo_path, &["checkout", "-b", name]).map(|_| ())
    }
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub is_current: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_manager_new() {
        let gm = GitManager::new();
        assert!(gm.cache.is_empty());
    }

    #[test]
    fn git_status_non_repo() {
        let mut gm = GitManager::new();
        let status = gm.get_status("/tmp/definitely_not_a_repo_12345");
        assert!(!status.is_repo);
        assert!(status.files.is_empty());
    }

    #[test]
    fn git_status_caching() {
        let mut gm = GitManager::new();
        let _ = gm.get_status("/tmp");
        let _ = gm.get_status("/tmp");
        assert!(gm.cache.len() <= 1);
    }

    #[test]
    fn invalidate_cache_works() {
        let mut gm = GitManager::new();
        let _ = gm.get_status("/tmp");
        gm.invalidate_cache("/tmp");
        assert!(!gm.cache.contains_key("/tmp"));
    }

    #[test]
    fn file_status_symbols() {
        assert_eq!(FileStatus::Modified.symbol(), "M");
        assert_eq!(FileStatus::Added.symbol(), "A");
        assert_eq!(FileStatus::Deleted.symbol(), "D");
        assert_eq!(FileStatus::Renamed.symbol(), "R");
        assert_eq!(FileStatus::Untracked.symbol(), "?");
    }
}
