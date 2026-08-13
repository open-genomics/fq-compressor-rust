// =============================================================================
// fqc-rust - Ordinary-file output transactions
// =============================================================================
// Writers never truncate the final path until the full payload has been flushed
// and closed. Temps live in the destination directory so rename stays on one
// filesystem. Stdout is out of scope (callers must not use this type for "-").
// =============================================================================

use crate::archive::writer::FqcWriter;
use crate::error::{FqcError, Result};
use std::fs::File;
use std::path::{Path, PathBuf};
use tempfile::{Builder, NamedTempFile, TempPath};

/// In-flight write to a temporary sibling of `final_path`.
///
/// Drop without [`commit`](Self::commit) deletes the temporary file and leaves
/// the final path untouched (including when `--force` would have replaced it).
pub struct OutputTransaction {
    final_path: PathBuf,
    temp_path: Option<TempPath>,
    file: Option<File>,
}

impl OutputTransaction {
    /// Begin a transaction for an ordinary filesystem path.
    ///
    /// Fails before creating a temp when the final path exists and `force` is
    /// false. Refuses the stdout sentinel `"-"`.
    pub fn begin(final_path: impl AsRef<Path>, force: bool) -> Result<Self> {
        let final_path = final_path.as_ref().to_path_buf();
        if final_path.as_os_str() == "-" {
            return Err(FqcError::InvalidArgument(
                "output transaction cannot target stdout".to_string(),
            ));
        }
        if final_path.exists() && !force {
            return Err(FqcError::InvalidArgument(format!(
                "Output file already exists: {} (use -f to overwrite)",
                final_path.display()
            )));
        }

        let parent = final_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));

        let named: NamedTempFile = Builder::new()
            .prefix(".fqc-tmp-")
            .suffix(".partial")
            .tempfile_in(parent)
            .map_err(FqcError::Io)?;
        let (file, temp_path) = named.into_parts();

        Ok(Self {
            final_path,
            temp_path: Some(temp_path),
            file: Some(file),
        })
    }

    /// Take the exclusive write handle. Call once; subsequent calls error.
    pub fn take_file(&mut self) -> Result<File> {
        self.file
            .take()
            .ok_or_else(|| FqcError::Io(std::io::Error::other("output transaction file already taken")))
    }

    pub fn final_path(&self) -> &Path {
        &self.final_path
    }

    /// Persist the temporary file onto the final path after the writer was
    /// flushed and dropped. Consumes the transaction.
    pub fn commit(mut self) -> Result<()> {
        if let Some(file) = self.file.take() {
            file.sync_all().map_err(FqcError::Io)?;
            drop(file);
        }
        let temp = self.temp_path.take().ok_or_else(|| {
            FqcError::Io(std::io::Error::other(
                "output transaction already committed or missing temp path",
            ))
        })?;
        temp.persist(&self.final_path).map_err(|e| FqcError::Io(e.error))?;
        Ok(())
    }
}

impl Drop for OutputTransaction {
    fn drop(&mut self) {
        // TempPath deletes the file on drop when persist was not called.
        self.file.take();
        self.temp_path.take();
    }
}

/// Open an [`FqcWriter`] backed by a same-directory temporary file.
pub fn begin_fqc_writer(path: impl AsRef<Path>, force: bool) -> Result<(FqcWriter, OutputTransaction)> {
    let mut tx = OutputTransaction::begin(path, force)?;
    let writer = FqcWriter::from_file(tx.take_file()?)?;
    Ok((writer, tx))
}

/// Commit R1 then R2 after both writers have been flushed and dropped.
///
/// POSIX cannot rename two paths atomically. If the second commit fails after
/// the first succeeded, R1 may already have been replaced.
pub fn commit_split(r1: OutputTransaction, r2: OutputTransaction) -> Result<()> {
    r1.commit()?;
    r2.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn missing_target_stays_absent_when_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("out.fqc");
        {
            let mut tx = OutputTransaction::begin(&final_path, false).unwrap();
            let mut f = tx.take_file().unwrap();
            f.write_all(b"partial").unwrap();
            // drop without commit
        }
        assert!(!final_path.exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn force_keeps_old_content_on_abort() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("out.fqc");
        std::fs::write(&final_path, b"original").unwrap();
        {
            let mut tx = OutputTransaction::begin(&final_path, true).unwrap();
            let mut f = tx.take_file().unwrap();
            f.write_all(b"partial-new").unwrap();
        }
        assert_eq!(std::fs::read(&final_path).unwrap(), b"original");
    }

    #[test]
    fn refuse_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("out.fqc");
        std::fs::write(&final_path, b"original").unwrap();
        match OutputTransaction::begin(&final_path, false) {
            Err(FqcError::InvalidArgument(_)) => {}
            Ok(_) => panic!("expected InvalidArgument, got Ok"),
            Err(e) => panic!("expected InvalidArgument, got {e}"),
        }
        assert_eq!(std::fs::read(&final_path).unwrap(), b"original");
    }

    #[test]
    fn successful_commit_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("out.fqc");
        std::fs::write(&final_path, b"old").unwrap();
        {
            let mut tx = OutputTransaction::begin(&final_path, true).unwrap();
            let mut f = tx.take_file().unwrap();
            f.write_all(b"new-content").unwrap();
            f.sync_all().unwrap();
            drop(f);
            tx.commit().unwrap();
        }
        assert_eq!(std::fs::read(&final_path).unwrap(), b"new-content");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn rejects_stdout_sentinel() {
        match OutputTransaction::begin("-", false) {
            Err(FqcError::InvalidArgument(_)) => {}
            Ok(_) => panic!("expected InvalidArgument, got Ok"),
            Err(e) => panic!("expected InvalidArgument, got {e}"),
        }
    }
}
