//! Media paths relative to the project folder.
//!
//! In memory, local media paths are absolute, so everything that opens an
//! image and every undo snapshot can use them as they are. On disk, media
//! inside the project folder is stored relative to it, so the folder can be
//! moved or shared with its images. Media elsewhere stays absolute.

use notatus_core::{AssetLocation, AssetRecord};
use std::path::{Component, Path, PathBuf};

/// `assets` with every local path inside `root` made relative to it, using
/// `/` between folders on every platform.
pub(crate) fn for_disk(assets: &[AssetRecord], root: &Path) -> Vec<AssetRecord> {
    let root = absolute(root);
    assets
        .iter()
        .map(|asset| {
            let mut asset = asset.clone();
            if let AssetLocation::LocalPath { path } = &mut asset.location
                && let Some(relative) = relative_inside(Path::new(path), &root)
            {
                *path = relative;
            }
            asset
        })
        .collect()
}

/// Resolves relative local paths read from disk against the project folder.
pub(crate) fn from_disk(assets: &mut [AssetRecord], root: &Path) {
    let root = absolute(root);
    for asset in assets {
        if let AssetLocation::LocalPath { path } = &mut asset.location
            && Path::new(path.as_str()).is_relative()
        {
            *path = root.join(path.as_str()).to_string_lossy().into_owned();
        }
    }
}

/// `path` relative to `root` with `/` separators, when it lies inside it.
fn relative_inside(path: &Path, root: &Path) -> Option<String> {
    if !path.is_absolute() {
        return None;
    }
    let relative = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            // A path such as root/../x only looks inside the folder.
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(path: &str) -> AssetRecord {
        AssetRecord::new_image(AssetLocation::local(path), 10, 10).unwrap()
    }

    fn local_path(asset: &AssetRecord) -> &str {
        match &asset.location {
            AssetLocation::LocalPath { path } => path,
            AssetLocation::S3Object { .. } => panic!("expected a local path"),
        }
    }

    #[test]
    fn stores_media_inside_the_project_relative_to_it() {
        let root = Path::new("/data/project");
        let assets = [
            asset("/data/project/images/a.jpg"),
            asset("/data/other/b.jpg"),
            asset("/data/project/../escape.jpg"),
            asset("/data/projectile/c.jpg"),
        ];

        let stored = for_disk(&assets, root);

        assert_eq!(local_path(&stored[0]), "images/a.jpg");
        assert_eq!(local_path(&stored[1]), "/data/other/b.jpg");
        assert_eq!(local_path(&stored[2]), "/data/project/../escape.jpg");
        assert_eq!(local_path(&stored[3]), "/data/projectile/c.jpg");
    }

    #[test]
    fn resolves_relative_paths_against_the_project() {
        let mut assets = [asset("images/a.jpg"), asset("/data/other/b.jpg")];

        from_disk(&mut assets, Path::new("/moved/project"));

        assert_eq!(local_path(&assets[0]), "/moved/project/images/a.jpg");
        assert_eq!(local_path(&assets[1]), "/data/other/b.jpg");
    }
}
