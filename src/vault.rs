use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs;
use chrono::Local;

/// Windows reserved names that cannot be used as filenames or path components.
/// Any attempt to create files with these names can cause system errors or lockups on Windows.
const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
];

/// Represents an Obsidian Vault and handles direct, secure text-based file operations.
#[derive(Debug, Clone)]
pub struct ObsidianVault {
    root_dir: PathBuf,
}

impl ObsidianVault {
    /// Initializes a new ObsidianVault instance.
    ///
    /// The root path is canonicalized immediately to guarantee correctness and safety 
    /// against directory traversal attacks.
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let root_dir = std::fs::canonicalize(root)?;
        if !root_dir.is_dir() {
            return Err("The provided vault root path is not a valid directory.".into());
        }
        Ok(Self { root_dir })
    }

    /// Resolves a note title into a safe, absolute target `PathBuf` within the vault boundaries.
    ///
    /// **Safety Guardrails:**
    /// - Rejects parent directory traversal components (`..`).
    /// - Rejects absolute paths or drive letters in titles.
    /// - Blocks Windows reserved device names (e.g. `CON`, `NUL`, `PRN`).
    /// - Guarantees the resolved path is structurally restricted within the canonicalized vault root.
    /// - Enforces the `.md` file extension.
    fn resolve_path(&self, title: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        if title.is_empty() {
            return Err("Note title cannot be empty.".into());
        }

        let path = Path::new(title);

        // Security Validation: Scan components for traversal vectors and reserved names.
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    return Err("Directory traversal attempt detected (..) in title.".into());
                }
                std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                    return Err("Absolute paths or drive letters are prohibited in titles.".into());
                }
                std::path::Component::Normal(name) => {
                    if let Some(name_str) = name.to_str() {
                        let name_upper = name_str.to_uppercase();
                        let stem = Path::new(&name_upper)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(&name_upper);
                        if RESERVED_NAMES.contains(&stem) {
                            return Err(format!("Windows reserved device name '{}' is prohibited.", name_str).into());
                        }
                    }
                }
                _ => {}
            }
        }

        // Join with the canonicalized root directory.
        let mut full_path = self.root_dir.clone();
        full_path.push(path);

        // Enforce the `.md` extension.
        if full_path.extension().map_or(true, |ext| ext != "md") {
            let mut file_name = full_path
                .file_name()
                .ok_or_else(|| "Invalid file name in title.")?
                .to_os_string();
            file_name.push(".md");
            full_path.set_file_name(file_name);
        }

        // Double-check: ensure the resolved path remains strictly within the vault root.
        if !full_path.starts_with(&self.root_dir) {
            return Err("Resolved path escapes the vault root directory.".into());
        }

        Ok(full_path)
    }

    /// Finds an existing `.md` file in the vault by its title.
    ///
    /// It first checks if the path can be resolved directly. If not, it recursively
    /// searches the entire vault for a matching file name (case-insensitively, handling
    /// both `My Note` and `My Note.md` inputs).
    async fn find_note_path(&self, title: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
        // 1. Direct path check (fast path).
        let direct_path = self.resolve_path(title)?;
        if fs::metadata(&direct_path).await.is_ok() {
            return Ok(direct_path);
        }

        // 2. Recursive fallback search.
        let root = self.root_dir.clone();
        
        // Strip .md if the user included it in the search string for clean matching.
        let title_clean = if title.to_lowercase().ends_with(".md") {
            &title[..title.len() - 3]
        } else {
            title
        };
        let target_filename = format!("{}.md", title_clean.to_lowercase());

        // Spawn a blocking task to run WalkDir off the async thread pool.
        let found_path = tokio::task::spawn_blocking(move || {
            use walkdir::WalkDir;
            for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.to_lowercase() == target_filename {
                            return Some(entry.into_path());
                        }
                    }
                }
            }
            None
        })
        .await?;

        match found_path {
            Some(path) => {
                if path.starts_with(&self.root_dir) {
                    Ok(path)
                } else {
                    Err("Found note path escapes vault root.".into())
                }
            }
            None => Err(format!("Note '{}' not found in the vault.", title).into()),
        }
    }

    /// Creates a new `.md` note.
    ///
    /// Automatically builds parent directories if they do not exist, formats and injects
    /// a YAML frontmatter block at the top containing the current date and tags,
    /// and writes the content.
    ///
    /// Returns an error if the note already exists to protect data from accidental overwriting.
    pub async fn create_note(
        &self,
        title: &str,
        content: &str,
        tags: Vec<String>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = self.resolve_path(title)?;

        // Ensure subdirectories exist.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Open with create_new to prevent accidental data loss.
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(format!("Note '{}' already exists in the vault. Use append_to_note to update it.", title).into());
            }
            Err(e) => return Err(e.into()),
        };

        // Format the YAML frontmatter.
        let date_str = Local::now().format("%Y-%m-%d").to_string();
        let mut frontmatter = String::new();
        frontmatter.push_str("---\n");
        frontmatter.push_str(&format!("date: {}\n", date_str));
        
        if !tags.is_empty() {
            frontmatter.push_str("tags:\n");
            for tag in &tags {
                let sanitized = tag.replace('"', "\\\"");
                frontmatter.push_str(&format!("  - \"{}\"\n", sanitized));
            }
        } else {
            frontmatter.push_str("tags: []\n");
        }
        frontmatter.push_str("---\n\n");

        let full_payload = format!("{}{}", frontmatter, content);
        tokio::io::AsyncWriteExt::write_all(&mut file, full_payload.as_bytes()).await?;

        Ok(())
    }

    /// Appends new content to the bottom of an existing note.
    ///
    /// Precedes the content with a timestamped Markdown header (e.g. `## Update: [Time]`).
    pub async fn append_to_note(
        &self,
        title: &str,
        content: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let path = self.find_note_path(title).await?;

        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&path)
            .await?;

        let time_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let update_block = format!("\n\n## Update: {}\n\n{}", time_str, content);

        tokio::io::AsyncWriteExt::write_all(&mut file, update_block.as_bytes()).await?;

        Ok(())
    }

    /// Reads the full text contents of a note in the vault by title.
    pub async fn read_note(&self, title: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let path = self.find_note_path(title).await?;
        let content = fs::read_to_string(path).await?;
        Ok(content)
    }

    /// Recursively searches the vault directory for `.md` files containing the query.
    ///
    /// Performs a case-insensitive search and returns a list of relative paths
    /// (e.g. `"Folder/Note.md"`) of all matching notes to preserve uniqueness.
    pub async fn search_vault(&self, query: &str) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let root = self.root_dir.clone();
        let query_lower = query.to_lowercase();

        let matches = tokio::task::spawn_blocking(move || {
            use walkdir::WalkDir;
            let mut results = Vec::new();

            for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "md") {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            if content.to_lowercase().contains(&query_lower) {
                                if let Ok(rel_path) = path.strip_prefix(&root) {
                                    if let Some(rel_str) = rel_path.to_str() {
                                        results.push(rel_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            results
        })
        .await?;

        Ok(matches)
    }
}
