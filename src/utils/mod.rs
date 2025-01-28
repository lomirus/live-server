use std::path::Path;

use ignore::gitignore::Gitignore;

/// Remove the prefix path, like `/home/mirus/live-server/src/main.rs` -> `src/main.rs`
pub(crate) fn strip_prefix<'a>(path: &'a Path, prefix: &Path) -> &'a Path {
    path.strip_prefix(prefix).unwrap()
}

/// Check if the target file (`target_path`) is ignored or hidden in the direcotry (`dir_path`).
pub(crate) fn is_ignored(dir_path: &Path, target_path: &Path) -> Result<bool, ignore::Error> {
    let is_hidden = strip_prefix(target_path, dir_path)
        .components()
        .any(|component| {
            if let std::path::Component::Normal(name) = component {
                let name_str = name.to_string_lossy();
                name_str.starts_with('.')
            } else {
                false
            }
        });
    if is_hidden {
        return Ok(true);
    }
    let gitignore_path = dir_path.join(".gitignore");
    let (gitignore, err) = Gitignore::new(gitignore_path);
    if let Some(err) = err {
        return Err(err);
    }
    Ok(gitignore
        .matched_path_or_any_parents(target_path, target_path.is_dir())
        .is_ignore())
}
