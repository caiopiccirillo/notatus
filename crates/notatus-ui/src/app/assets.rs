//! Icons and other assets the app embeds.
//!
//! `gpui_kit_assets::Assets` embeds only the ~100 icons gpui-component uses.
//! Any other Lucide icon renders blank until it is registered here; the test
//! below fails when the UI names one that is not.

use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

gpui_kit_assets::icon_assets!(
    AppIcons,
    [
        Crosshair,
        Download,
        FilePlus,
        Focus,
        ImagePlus,
        ListTree,
        MousePointer2,
        Pentagon,
        RotateCcw,
        Save,
        SaveAll,
        Tag,
        VectorSquare
    ]
);

/// The app's icons first, then gpui-component's.
pub(crate) struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = AppIcons.load(path)? {
            return Ok(Some(bytes));
        }
        gpui_kit_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        let mut paths = gpui_kit_assets::Assets.list(path)?;
        paths.extend(AppIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Every icon the UI source names through the `Lucide` alias or the
    /// `gpui_kit_assets::IconName` path, found by reading the files.
    fn icon_names_in_source() -> Vec<String> {
        let mut names = Vec::new();
        let mut pending = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                if path.extension().is_none_or(|extension| extension != "rs") {
                    continue;
                }
                let source = std::fs::read_to_string(&path).unwrap();
                for prefix in ["Lucide::", "gpui_kit_assets::IconName::"] {
                    for (start, _) in source.match_indices(prefix) {
                        let name: String = source[start + prefix.len()..]
                            .chars()
                            .take_while(|ch| ch.is_ascii_alphanumeric())
                            .collect();
                        // Icon names are CamelCase; this skips constants
                        // such as `IconName::ALL`.
                        if name.starts_with(|ch: char| ch.is_ascii_uppercase())
                            && name.chars().any(|ch| ch.is_ascii_lowercase())
                        {
                            names.push(name);
                        }
                    }
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }

    #[test]
    fn every_icon_the_ui_names_is_embedded() {
        let names = icon_names_in_source();
        assert!(!names.is_empty(), "the source scan found no icons");

        let missing: Vec<_> = names
            .iter()
            .filter(|name| {
                let icon = gpui_kit_assets::IconName::ALL
                    .iter()
                    .find(|icon| format!("{icon:?}") == **name)
                    .unwrap_or_else(|| panic!("{name} is not a Lucide icon"));
                AppAssets.load(icon.path().as_ref()).unwrap().is_none()
            })
            .collect();

        assert!(
            missing.is_empty(),
            "add these to AppIcons in app/assets.rs: {missing:?}"
        );
    }
}
