use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A recovered swap file: the original source path and the recovered text content.
pub struct RecoveredFile {
    pub source_path: PathBuf,
    pub content: String,
}

/// Per-buffer swap state.
struct SwapEntry {
    /// Absolute path of the source file.
    source_path: PathBuf,
    /// BLAKE3 hash of the file content when loaded from or last saved to disk.
    disk_hash: [u8; 32],
    /// BLAKE3 hash of the buffer content at the last swap file write.
    last_swap_hash: [u8; 32],
    /// Path to the swap file on disk.
    swap_path: PathBuf,
    /// Path to the metadata file (stores original source path).
    meta_path: PathBuf,
}

/// Manages swap files and external change detection for all open buffers.
pub struct SwapManager {
    swap_dir: PathBuf,
    entries: HashMap<usize, SwapEntry>,
}

impl SwapManager {
    pub fn new() -> Self {
        let swap_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("astrum")
            .join("swap");
        std::fs::create_dir_all(&swap_dir).ok();
        Self {
            swap_dir,
            entries: HashMap::new(),
        }
    }

    /// Register a buffer for swap file tracking. Called when a file is opened or saved.
    pub fn register(&mut self, buf_id: usize, source_path: &Path, content_hash: [u8; 32]) {
        let abs_path = std::fs::canonicalize(source_path)
            .unwrap_or_else(|_| source_path.to_path_buf());
        let path_hash = blake3::hash(abs_path.to_string_lossy().as_bytes());
        let hex = path_hash.to_hex();
        let swap_path = self.swap_dir.join(format!("{}.swp", &hex[..32]));
        let meta_path = self.swap_dir.join(format!("{}.meta", &hex[..32]));

        self.entries.insert(buf_id, SwapEntry {
            source_path: abs_path,
            disk_hash: content_hash,
            last_swap_hash: content_hash,
            swap_path,
            meta_path,
        });
    }

    /// Unregister a buffer and delete its swap file. Called on buffer close or clean exit.
    pub fn unregister(&mut self, buf_id: usize) {
        if let Some(entry) = self.entries.remove(&buf_id) {
            std::fs::remove_file(&entry.swap_path).ok();
            std::fs::remove_file(&entry.meta_path).ok();
        }
    }

    /// Check if the buffer content has changed since the last swap file write.
    pub fn needs_swap_write(&self, buf_id: usize, current_hash: [u8; 32]) -> bool {
        self.entries
            .get(&buf_id)
            .map_or(false, |e| current_hash != e.last_swap_hash)
    }

    /// Check if the file on disk has changed since we loaded/saved it.
    pub fn disk_changed(&self, buf_id: usize, current_disk_hash: [u8; 32]) -> bool {
        self.entries
            .get(&buf_id)
            .map_or(false, |e| current_disk_hash != e.disk_hash)
    }

    /// Update the disk hash after a successful save.
    pub fn update_disk_hash(&mut self, buf_id: usize, new_hash: [u8; 32]) {
        if let Some(entry) = self.entries.get_mut(&buf_id) {
            entry.disk_hash = new_hash;
        }
    }

    /// Update the swap hash after a swap file write completes.
    pub fn update_swap_hash(&mut self, buf_id: usize, new_hash: [u8; 32]) {
        if let Some(entry) = self.entries.get_mut(&buf_id) {
            entry.last_swap_hash = new_hash;
        }
    }

    /// Get the swap file path and source path for a buffer (used by the flush task).
    pub fn swap_info(&self, buf_id: usize) -> Option<(&Path, &Path)> {
        self.entries
            .get(&buf_id)
            .map(|e| (e.swap_path.as_path(), e.source_path.as_path()))
    }

    /// Get the meta file path for a buffer.
    pub fn meta_path(&self, buf_id: usize) -> Option<&Path> {
        self.entries.get(&buf_id).map(|e| e.meta_path.as_path())
    }

    /// Write a swap file and its metadata. Called from a background task.
    pub fn write_swap_file(
        swap_path: &Path,
        meta_path: &Path,
        source_path: &Path,
        content: &str,
    ) {
        std::fs::write(swap_path, content).ok();
        std::fs::write(meta_path, source_path.to_string_lossy().as_bytes()).ok();
    }

    /// Scan the swap directory for stale swap files (crash recovery).
    /// Returns pairs of (source_path, swap_path) for files that weren't cleaned up.
    pub fn find_stale_swap_files(&self) -> Vec<RecoveredFile> {
        let mut recovered = Vec::new();
        let entries = match std::fs::read_dir(&self.swap_dir) {
            Ok(e) => e,
            Err(_) => return recovered,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("meta") {
                continue;
            }

            let source_path_str = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let source_path = PathBuf::from(source_path_str.trim());

            // Derive the swap file path from the meta path.
            let swap_path = path.with_extension("swp");
            let swap_content = match std::fs::read_to_string(&swap_path) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Only offer recovery if the source file exists and the swap content
            // differs from what's on disk.
            if source_path.exists() {
                let disk_content = std::fs::read_to_string(&source_path).unwrap_or_default();
                if swap_content != disk_content {
                    recovered.push(RecoveredFile {
                        source_path,
                        content: swap_content,
                    });
                } else {
                    // Swap matches disk — stale, clean it up.
                    std::fs::remove_file(&swap_path).ok();
                    std::fs::remove_file(&path).ok();
                }
            } else {
                // Source file doesn't exist but swap does — offer recovery.
                recovered.push(RecoveredFile {
                    source_path,
                    content: swap_content,
                });
            }
        }

        recovered
    }

    /// Clean up all swap files (called on normal exit).
    pub fn cleanup_all(&mut self) {
        let buf_ids: Vec<usize> = self.entries.keys().copied().collect();
        for buf_id in buf_ids {
            self.unregister(buf_id);
        }
    }

    /// Get all registered buffer IDs.
    pub fn registered_buffer_ids(&self) -> Vec<usize> {
        self.entries.keys().copied().collect()
    }
}

/// Hash file content on disk. Returns None if the file can't be read.
pub fn hash_file(path: &Path) -> Option<[u8; 32]> {
    let content = std::fs::read(path).ok()?;
    Some(*blake3::hash(&content).as_bytes())
}

