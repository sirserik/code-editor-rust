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
    pub fn symbol(&self) -> &str {
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

    pub fn is_repo(path: &str) -> bool {
        Repository::discover(path).is_ok()
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

    pub fn diff_file(&self, repo_path: &str, file_path: &str) -> Result<String, String> {
        let repo = Repository::discover(repo_path).map_err(|e| e.to_string())?;
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(file_path);

        let diff = repo
            .diff_index_to_workdir(None, Some(&mut opts))
            .map_err(|e| e.to_string())?;

        let mut result = String::new();
        diff.print(git2::DiffFormat::Patch, |_, _, line| {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            result.push_str(prefix);
            result.push_str(&String::from_utf8_lossy(line.content()));
            true
        })
        .map_err(|e| e.to_string())?;

        Ok(result)
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
}
