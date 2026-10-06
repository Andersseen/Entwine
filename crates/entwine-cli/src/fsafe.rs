//! Conservative file creation for scaffolding: never overwrite, never follow symlinks.
use std::{
    fs, io,
    io::Write,
    path::{Component, Path},
};

pub(crate) enum Written {
    Created,
    Exists,
}

/// Create `relative` under `root` only if it does not exist. Every directory below `root`
/// must be a real directory; a symbolic link anywhere on the path is refused.
pub(crate) fn create_new(root: &Path, relative: &str, contents: &str) -> io::Result<Written> {
    let mut path = root.to_path_buf();
    let parts: Vec<_> = Path::new(relative).components().collect();
    if parts.is_empty() || parts.iter().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unsafe path: {relative}"),
        ));
    }
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        let last = index + 1 == parts.len();
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(io::Error::other(format!(
                    "Refusing to write through symbolic link: {}",
                    path.display()
                )))
            }
            Ok(meta) if last => {
                return if meta.is_file() {
                    Ok(Written::Exists)
                } else {
                    Err(io::Error::other(format!(
                        "{} exists and is not a file",
                        path.display()
                    )))
                }
            }
            Ok(meta) if !meta.is_dir() => {
                return Err(io::Error::other(format!(
                    "{} exists and is not a directory",
                    path.display()
                )))
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if !last {
                    fs::create_dir(&path)?;
                }
            }
            Err(e) => return Err(e),
        }
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(contents.as_bytes())?;
            Ok(Written::Created)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(Written::Exists),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_overwrites_and_rejects_unsafe_paths() {
        let temp = tempfile::tempdir().unwrap();
        assert!(matches!(
            create_new(temp.path(), "a/b.md", "one").unwrap(),
            Written::Created
        ));
        assert!(matches!(
            create_new(temp.path(), "a/b.md", "two").unwrap(),
            Written::Exists
        ));
        assert_eq!(
            fs::read_to_string(temp.path().join("a/b.md")).unwrap(),
            "one"
        );
        for bad in ["../x", "/x", "", "a/../../x"] {
            assert!(create_new(temp.path(), bad, "x").is_err(), "{bad}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinked_directories() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("docs")).unwrap();
        assert!(create_new(temp.path(), "docs/x.md", "x").is_err());
        assert!(!outside.path().join("x.md").exists());
    }
}
