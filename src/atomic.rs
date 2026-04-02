use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Writes to a temporary file, then atomically renames to the target path on commit.
/// If dropped without committing, the temporary file is cleaned up.
pub struct AtomicWriter {
    temp_path: PathBuf,
    final_path: PathBuf,
    writer: BufWriter<File>,
    committed: bool,
}

impl AtomicWriter {
    pub fn new(final_path: impl Into<PathBuf>) -> io::Result<Self> {
        let final_path = final_path.into();
        let temp_path = final_path.with_extension("tmp");

        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = File::create(&temp_path)?;
        let writer = BufWriter::new(file);

        Ok(Self {
            temp_path,
            final_path,
            writer,
            committed: false,
        })
    }

    pub fn writer(&mut self) -> &mut BufWriter<File> {
        &mut self.writer
    }

    pub fn commit(mut self) -> io::Result<()> {
        self.writer.flush()?;
        fs::rename(&self.temp_path, &self.final_path)?;
        self.committed = true;
        Ok(())
    }
}

impl Write for AtomicWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl Drop for AtomicWriter {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

/// Atomically write the entire contents to a file.
pub fn atomic_write(path: impl AsRef<Path>, contents: &[u8]) -> io::Result<()> {
    let mut w = AtomicWriter::new(path.as_ref())?;
    w.write_all(contents)?;
    w.commit()
}
