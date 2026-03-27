use std::{
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use tracing::{debug, trace, warn};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("File was not modified")]
    NotModified,
    #[error(transparent)]
    Other(#[from] std::io::Error),
}
pub(crate) struct BackupFile {
    dest: PathBuf,
    working_path: PathBuf,
    working_metadata: std::fs::Metadata,
    finished: bool,
}

impl BackupFile {
    pub(crate) fn new(path: PathBuf, initial_content: Option<String>) -> anyhow::Result<Self> {
        let mut working_path = path.to_path_buf();
        loop {
            working_path.add_extension("bak");
            if !working_path.exists() {
                break;
            }
        }
        trace!(?working_path, "found working path");

        // Copy existing config file to temporary file for editing
        trace!("attempting to copy existing file to working path");
        std::fs::copy(&path, &working_path)
            .map(|_| ())
            .or_else(|e| -> anyhow::Result<()> {
                if e.kind() == ErrorKind::NotFound {
                    debug!("No existing file to copy. Creating new working file.");
                    let mut f = std::fs::File::create_new(&working_path)?;
                    if let Some(content) = initial_content {
                        f.write_all(&content.into_bytes())?;
                        debug!("Wrote initial content to working file");
                    }
                    Ok(())
                } else {
                    warn!("copying existing file to working file failed: {e}");
                    Err(e.into())
                }
            })?;
        let working_metadata = working_path.metadata()?;
        Ok(Self {
            dest: path,
            working_path,
            working_metadata,
            finished: false,
        })
    }

    pub(crate) fn finish(mut self) -> Result<PathBuf, Error> {
        let metadata = self.working_path.metadata()?;
        if metadata.modified()? > self.working_metadata.modified()? {
            std::fs::rename(&self.working_path, &self.dest)?;
            self.finished = true;
            let dest: PathBuf = std::mem::take(&mut self.dest);
            Ok(dest)
        } else {
            Err(Error::NotModified)
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.working_path
    }
}

impl Drop for BackupFile {
    fn drop(&mut self) {
        if !self.finished
            && let Err(e) = std::fs::remove_file(&self.working_path)
        {
            tracing::warn!("Failed to clean up backup file: {e}");
        }
    }
}
