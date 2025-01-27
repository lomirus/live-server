use std::path::Path;

use ignore::gitignore::Gitignore;

/// Check if the target file (`target_path`) is ignored in the direcotry (`dir_path`).
pub(crate) fn is_ignored(
    dir_path: &Path,
    target_path: &Path,
    is_dir: bool,
) -> Result<bool, ignore::Error> {
    let gitignore_path = dir_path.join(".gitignore");
    let (gitignore, err) = Gitignore::new(gitignore_path);
    if let Some(err) = err {
        return Err(err);
    }
    Ok(gitignore.matched(target_path, is_dir).is_ignore())
}
