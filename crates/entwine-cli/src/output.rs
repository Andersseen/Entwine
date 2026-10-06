use entwine_engine::{render, Compilation};
use std::{fs, io, path::Path};

/// Stage a full output tree, preserving the last successful build on failure.
pub(crate) fn publish(project: &Path, compilation: &Compilation) -> io::Result<()> {
    let destination = project.join("dist");
    if destination.symlink_metadata().is_ok() {
        if destination.symlink_metadata()?.file_type().is_symlink() || !destination.is_dir() {
            return Err(io::Error::other(
                "Refusing to replace dist: expected a regular directory",
            ));
        }
        let marker = destination.join(".entwine-output");
        if marker
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
            || fs::read_to_string(marker).ok().as_deref() != Some("Entwine 0.1\n")
        {
            return Err(io::Error::other("dist/ exists and is not Entwine output. Move it before building; Entwine will not delete unrelated files."));
        }
    }
    let stage = tempfile::Builder::new()
        .prefix(".entwine-stage-")
        .tempdir_in(project)?;
    for file in render(compilation) {
        let path = stage.path().join(&file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, file.contents)?;
    }
    fs::write(stage.path().join(".entwine-output"), "Entwine 0.1\n")?;
    let backup = tempfile::Builder::new()
        .prefix(".entwine-backup-")
        .tempdir_in(project)?;
    let previous = backup.path().join("previous");
    let existed = destination.exists();
    if existed {
        fs::rename(&destination, &previous)?;
    }
    if let Err(error) = fs::rename(stage.path(), &destination) {
        if existed {
            if let Err(restore) = fs::rename(&previous, &destination) {
                let kept = backup.keep();
                return Err(io::Error::other(format!("Publication failed: {error}; restoration failed: {restore}. Previous output preserved at {}", kept.display())));
            }
        }
        return Err(error);
    }
    Ok(())
}
