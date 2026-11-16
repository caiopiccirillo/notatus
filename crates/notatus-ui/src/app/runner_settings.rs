//! Which model runner each project uses, kept in the user's own settings.
//!
//! A runner is a program the app executes, so it must never come from a
//! project file: a project downloaded or shared by someone else could name
//! any command. The choice lives in a settings file in the user's config
//! folder instead, keyed by project ID, and a project starts with no runner
//! until its user picks one.

use notatus_core::ProjectId;
use notatus_infer::ExternalModelSpec;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "model-runners.json";

pub(super) struct RunnerSettings {
    /// Where choices are saved; `None` keeps them for this session only.
    path: Option<PathBuf>,
    runners: BTreeMap<ProjectId, ExternalModelSpec>,
}

impl RunnerSettings {
    /// The user's settings, or session-only settings when there is no config
    /// folder. Tests never touch the real one.
    pub(super) fn for_user() -> Self {
        let path = if cfg!(test) {
            None
        } else {
            config_dir().map(|dir| dir.join("notatus").join(FILE_NAME))
        };
        Self::at(path)
    }

    /// Settings saved at `path`, reading what is already there. A missing or
    /// unreadable file starts empty.
    pub(super) fn at(path: Option<PathBuf>) -> Self {
        let runners = path
            .as_deref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self { path, runners }
    }

    pub(super) fn get(&self, project: ProjectId) -> Option<&ExternalModelSpec> {
        self.runners.get(&project)
    }

    /// Records `spec` for `project` and saves the settings.
    pub(super) fn set(
        &mut self,
        project: ProjectId,
        spec: ExternalModelSpec,
    ) -> std::io::Result<()> {
        self.runners.insert(project, spec);
        match &self.path {
            Some(path) => save(path, &self.runners),
            None => Ok(()),
        }
    }
}

/// Writes the settings through a temporary file, so a crash cannot leave
/// them half written.
fn save(path: &Path, runners: &BTreeMap<ProjectId, ExternalModelSpec>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(runners).map_err(std::io::Error::other)?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, json)?;
    std::fs::rename(&temp, path)
}

/// The platform's folder for per-user settings.
fn config_dir() -> Option<PathBuf> {
    let env = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
    if cfg!(target_os = "windows") {
        env("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|home| PathBuf::from(home).join(".config")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(program: &str) -> ExternalModelSpec {
        ExternalModelSpec {
            program: PathBuf::from(program),
            args: Vec::new(),
            env: Default::default(),
        }
    }

    #[test]
    fn choices_are_saved_per_project_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/model-runners.json");
        let (first, second) = (ProjectId::new(), ProjectId::new());

        let mut settings = RunnerSettings::at(Some(path.clone()));
        settings.set(first, spec("/opt/yolo")).unwrap();
        settings.set(second, spec("/opt/sam")).unwrap();

        let reloaded = RunnerSettings::at(Some(path));
        assert_eq!(reloaded.get(first), Some(&spec("/opt/yolo")));
        assert_eq!(reloaded.get(second), Some(&spec("/opt/sam")));
        assert_eq!(reloaded.get(ProjectId::new()), None);
    }

    #[test]
    fn an_unreadable_file_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model-runners.json");
        std::fs::write(&path, "not json").unwrap();

        assert_eq!(RunnerSettings::at(Some(path)).runners.len(), 0);
    }

    #[test]
    fn tests_never_use_the_real_settings() {
        assert_eq!(RunnerSettings::for_user().path, None);
    }
}
