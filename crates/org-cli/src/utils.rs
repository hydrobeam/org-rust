use std::{
    borrow::Cow,
    path::{Component, Path, PathBuf},
};

use crate::types::CliError;

// a fs::canonicalize that doesnt care for existince. used for error handling
// yanked straight from:
// https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub fn mkdir_recursively(path: &Path) -> Result<(), CliError> {
    std::fs::create_dir_all(path).map_err(|e| {
        CliError::from(e)
            .with_path(path)
            .with_cause(&format!("failed to create directory {}", path.display()))
    })
}

pub fn relative_path_from<'a, 'b>(
    src: &'a Path,
    added: &'b Path,
) -> Result<Cow<'b, Path>, CliError> {
    if added.is_relative() {
        Ok(src
            .parent()
            .ok_or(
                CliError::new()
                    .with_path(src)
                    .with_cause("no parent directory found"),
            )?
            .join(added)
            .canonicalize()
            .map_err(|e| {
                CliError::from(e)
                    .with_path(&src.parent().unwrap().join(added))
                    .with_cause(&format!(
                        "Failed to locate path from: {}",
                        src.display()
                    ))
            })?
            .into())
    } else {
        Ok(added.into())
    }
}
