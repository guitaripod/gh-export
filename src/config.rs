use crate::error::{GhExportError, Result};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub github_token: Option<String>,
    pub output_directory: PathBuf,
    pub parallel_downloads: usize,
    pub include_archived: bool,
    pub exclude_forks: bool,
    pub shallow_clone: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github_token: None,
            output_directory: PathBuf::from("./github-backup"),
            parallel_downloads: 4,
            include_archived: false,
            exclude_forks: false,
            shallow_clone: false,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| GhExportError::Config(format!("Failed to serialize config: {e}")))?;

        fs::write(&config_path, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&config_path, permissions)?;
        }

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let config_dir = config_dir().ok_or_else(|| {
            GhExportError::Config("Could not determine config directory".to_string())
        })?;
        Ok(config_dir.join("gh-export").join("config.toml"))
    }

    pub fn validate(&self) -> Result<()> {
        if self.parallel_downloads == 0 {
            return Err(GhExportError::Config(
                "Parallel downloads must be at least 1".to_string(),
            ));
        }

        if self.parallel_downloads > 10 {
            return Err(GhExportError::Config(
                "Parallel downloads should not exceed 10 to avoid rate limiting".to_string(),
            ));
        }

        Ok(())
    }

    pub fn ensure_output_directory(&self) -> Result<()> {
        if !self.output_directory.exists() {
            fs::create_dir_all(&self.output_directory)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub last_export: chrono::DateTime<chrono::Utc>,
    pub total_repos: usize,
    pub successful_exports: usize,
    pub failed_exports: Vec<String>,
    pub export_duration_seconds: u64,
}

impl ExportMetadata {
    pub fn save(&self, output_dir: &Path) -> Result<()> {
        let metadata_path = output_dir.join(".gh-export-metadata.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(metadata_path, content)?;
        Ok(())
    }

    pub fn load(output_dir: &Path) -> Result<Option<Self>> {
        let metadata_path = output_dir.join(".gh-export-metadata.json");
        if metadata_path.exists() {
            let content = fs::read_to_string(metadata_path)?;
            let metadata: ExportMetadata = serde_json::from_str(&content)?;
            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }
}
