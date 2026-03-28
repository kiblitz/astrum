use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Workspace directory
// ---------------------------------------------------------------------------

pub fn workspace_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("astrum")
        .join("workspace")
}

// ---------------------------------------------------------------------------
// Workspace — list of cloned repos (pane content)
// ---------------------------------------------------------------------------

pub struct Workspace {
    pub workspace_dir: PathBuf,
    pub repos: Vec<WorkspaceRepo>,
    pub selected: usize,
    pub scroll_offset: usize,
}

pub struct WorkspaceRepo {
    pub name: String,   // e.g. "kiblitz/astrum"
    pub path: PathBuf,  // full path on disk
}

impl Workspace {
    /// Scan workspace directory for cloned repos.
    pub fn open() -> Result<Self> {
        let dir = workspace_dir();
        std::fs::create_dir_all(&dir)?;

        let mut repos = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join(".git").exists() {
                    // Try to get the remote origin to derive "user/repo" name.
                    let name = git_remote_name(&path)
                        .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                    repos.push(WorkspaceRepo { name, path });
                }
            }
        }
        repos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(Workspace {
            workspace_dir: dir,
            repos,
            selected: 0,
            scroll_offset: 0,
        })
    }

    /// The set of "user/repo" names that are already cloned.
    pub fn cloned_set(&self) -> HashSet<String> {
        self.repos.iter().map(|r| r.name.clone()).collect()
    }
}

// ---------------------------------------------------------------------------
// GitRepo — single repo overview (pane content, placeholder for now)
// ---------------------------------------------------------------------------

pub struct GitRepo {
    pub repo_path: PathBuf,
    pub name: String,
    pub branch: String,
    pub remote_url: Option<String>,
}

impl GitRepo {
    pub fn open(repo_path: PathBuf) -> Result<Self> {
        let name = git_remote_name(&repo_path)
            .unwrap_or_else(|| {
                repo_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            });
        let branch = git_branch(&repo_path).unwrap_or_else(|_| "unknown".to_string());
        let remote_url = git_remote_url(&repo_path).ok();

        Ok(GitRepo {
            repo_path,
            name,
            branch,
            remote_url,
        })
    }
}

// ---------------------------------------------------------------------------
// GitStatus — uncommitted files (pane content)
// ---------------------------------------------------------------------------

pub struct GitStatus {
    pub repo_path: PathBuf,
    pub files: Vec<GitStatusFile>,
    pub selected: usize,
    pub scroll_offset: usize,
}

pub struct GitStatusFile {
    pub status: String, // "M ", " M", "??", "A ", etc.
    pub path: String,
}

impl GitStatus {
    pub fn open(repo_path: PathBuf) -> Result<Self> {
        let files = git_status_files(&repo_path)?;
        Ok(GitStatus {
            repo_path,
            files,
            selected: 0,
            scroll_offset: 0,
        })
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.files = git_status_files(&self.repo_path)?;
        if self.selected >= self.files.len() {
            self.selected = self.files.len().saturating_sub(1);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// CloneRepoState — popup for searching/cloning GitHub repos
// ---------------------------------------------------------------------------

pub struct CloneRepoState {
    pub query: String,
    pub cache: RepoCache,
    pub filtered: Vec<String>, // "user/repo" strings matching query
    pub selected: usize,
    pub cloned: HashSet<String>,
    pub loading: Option<String>, // user currently being fetched
    pub error: Option<String>,
}

impl CloneRepoState {
    pub fn new(cloned: HashSet<String>) -> Self {
        let cache = RepoCache::load();
        let mut state = CloneRepoState {
            query: String::new(),
            cache,
            filtered: Vec::new(),
            selected: 0,
            cloned,
            loading: None,
            error: None,
        };
        state.refilter();
        state
    }

    pub fn refilter(&mut self) {
        let query_lower = self.query.to_lowercase();
        self.filtered = self
            .cache
            .all_repos()
            .into_iter()
            .filter(|r| !self.cloned.contains(r))
            .filter(|r| query_lower.is_empty() || r.to_lowercase().contains(&query_lower))
            .collect();
        self.filtered.sort();
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }

    pub fn selected_repo(&self) -> Option<&str> {
        self.filtered.get(self.selected).map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// RepoCache — persisted cache of GitHub user repos
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RepoCache {
    pub users: HashMap<String, Vec<String>>,
}

impl RepoCache {
    fn cache_path() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("astrum")
            .join("github_repos.json")
    }

    pub fn load() -> Self {
        let path = Self::cache_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            RepoCache::default()
        }
    }

    pub fn save(&self) {
        let path = Self::cache_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, data);
        }
    }

    pub fn add_user(&mut self, user: String, repos: Vec<String>) {
        self.users.insert(user, repos);
        self.save();
    }

    /// All "user/repo" strings in the cache.
    pub fn all_repos(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (user, repos) in &self.users {
            for repo in repos {
                out.push(format!("{user}/{repo}"));
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Git helpers (synchronous, run in blocking context)
// ---------------------------------------------------------------------------

fn git_remote_name(repo_path: &Path) -> Option<String> {
    let url = git_remote_url(repo_path).ok()?;
    parse_github_repo_from_url(&url)
}

fn git_remote_url(repo_path: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output()
        .context("failed to run git")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_branch(repo_path: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("failed to run git")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_status_files(repo_path: &Path) -> Result<Vec<GitStatusFile>> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .context("failed to run git status")?;
    let text = String::from_utf8_lossy(&output.stdout);
    let files = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let status = line.get(..2).unwrap_or("??").to_string();
            let path = line.get(3..).unwrap_or("").to_string();
            GitStatusFile { status, path }
        })
        .collect();
    Ok(files)
}

/// Parse "user/repo" from a GitHub URL (HTTPS or SSH).
fn parse_github_repo_from_url(url: &str) -> Option<String> {
    // SSH: git@github.com:user/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let repo = rest.trim_end_matches(".git");
        return Some(repo.to_string());
    }
    // HTTPS: https://github.com/user/repo.git
    if let Some(rest) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
    {
        let repo = rest.trim_end_matches(".git");
        return Some(repo.to_string());
    }
    None
}

// ---------------------------------------------------------------------------
// Async operations (for editor event loop)
// ---------------------------------------------------------------------------

/// Fetch all repos for a GitHub user via `gh api`.
pub async fn fetch_user_repos(user: &str) -> Result<Vec<String>> {
    let output = tokio::process::Command::new("gh")
        .args([
            "api",
            &format!("users/{user}/repos"),
            "--paginate",
            "--jq",
            ".[].name",
        ])
        .output()
        .await
        .context("failed to run gh api — is the GitHub CLI installed and authenticated?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh api failed: {stderr}");
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let repos: Vec<String> = text.lines().filter(|l| !l.is_empty()).map(String::from).collect();
    Ok(repos)
}

/// Clone a GitHub repo into the workspace directory.
pub async fn git_clone(repo: &str) -> Result<PathBuf> {
    let dest = workspace_dir();
    std::fs::create_dir_all(&dest)?;

    let url = format!("https://github.com/{repo}.git");
    let output = tokio::process::Command::new("git")
        .args(["clone", &url])
        .current_dir(&dest)
        .output()
        .await
        .context("failed to run git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git clone failed: {stderr}");
    }

    // The cloned directory name is the repo name (after the slash).
    let repo_name = repo.split('/').last().unwrap_or(repo);
    Ok(dest.join(repo_name))
}

/// Stage all files in a repo.
pub async fn git_add_all(repo_path: &Path) -> Result<()> {
    let output = tokio::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .await
        .context("failed to run git add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git add failed: {stderr}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Determining workspace context
// ---------------------------------------------------------------------------

/// If the given path is inside the workspace directory, return the repo root.
pub fn workspace_repo_for_path(path: &Path) -> Option<PathBuf> {
    let ws = workspace_dir();
    let canonical_ws = std::fs::canonicalize(&ws).unwrap_or(ws.clone());
    let canonical_path = std::fs::canonicalize(path).unwrap_or(path.to_path_buf());

    if canonical_path.starts_with(&canonical_ws) {
        // The repo root is the first directory under workspace/
        let relative = canonical_path.strip_prefix(&canonical_ws).ok()?;
        let repo_dir_name = relative.components().next()?;
        let repo_path = canonical_ws.join(repo_dir_name);
        if repo_path.join(".git").exists() {
            return Some(repo_path);
        }
    }
    None
}
