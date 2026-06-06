use super::helpers::compact_text;
use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ProjectLocation {
    Unsaved,
    Local(PathBuf),
}

impl Default for ProjectLocation {
    fn default() -> Self {
        Self::Unsaved
    }
}

impl ProjectLocation {
    pub(super) fn local(path: impl Into<PathBuf>) -> Self {
        Self::Local(path.into())
    }

    pub(super) fn path(&self) -> Option<&Path> {
        match self {
            Self::Unsaved => None,
            Self::Local(path) => Some(path),
        }
    }

    pub(super) fn display_name(&self) -> String {
        match self {
            Self::Unsaved => "Not saved to disk".to_string(),
            Self::Local(path) => compact_text(&path.display().to_string(), 36),
        }
    }
}
