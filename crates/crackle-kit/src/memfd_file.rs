use std::{
    ffi::CString,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom, Write},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd},
    path::{Path, PathBuf},
};

use anyhow::Error;
use nix::sys::memfd::{MFdFlags, memfd_create};

pub struct MemFdFile {
    file: File,
    path: PathBuf,
}

impl MemFdFile {
    /// Create a new (empty) memfd file with the given name and flags.
    ///
    /// This function creates an anonymous in‑memory file. It returns a `MemFdFile`
    /// which wraps the underlying file descriptor.
    pub fn new(name: &str, flags: MFdFlags) -> Result<Self, Error> {
        // Create the memfd file descriptor.
        let fd = memfd_create(name, flags)?;

        let file = unsafe { File::from_raw_fd(fd.into_raw_fd()) };
        let path = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
        Ok(Self { file, path })
    }

    /// Create a new memfd file by copying the content of an existing file (e.g. a BAM file).
    ///
    /// The content from `source_path` is read and written into the memfd.
    /// After creation, you can use the `path()` method to retrieve its in‑memory path.
    pub fn from_path(
        source_path: impl AsRef<Path>,
        memfd_name: &str,
        flags: MFdFlags,
    ) -> Result<Self, Error> {
        // Open the source file (for example, a BAM file stored on disk).
        let mut source_file = File::open(source_path)?;
        let mut memfd_file = Self::new(memfd_name, flags)?;

        // Copy all contents from the source file into the memfd file.
        std::io::copy(&mut source_file, &mut memfd_file.file)?;
        // Seek back to the beginning so that the file pointer is reset.
        memfd_file.file.seek(SeekFrom::Start(0))?;
        Ok(memfd_file)
    }

    /// Return the file path (/proc/self/fd/<fd>) for this memfd.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Write data to the memfd file.
    pub fn write_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.file.write_all(data)?;
        Ok(())
    }

    pub fn truncate(&mut self) -> Result<(), Error> {
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;

        Ok(())
    }

    /// Read all the data from the memfd file.
    pub fn read_data(&mut self) -> Result<Vec<u8>, Error> {
        self.file.seek(SeekFrom::Start(0))?;
        let mut buffer = Vec::new();
        self.file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }
}
