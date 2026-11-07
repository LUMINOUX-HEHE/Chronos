use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Write bytes to a sibling temporary file and replace the destination by rename.
///
/// The temporary file is created with create_new to avoid following or clobbering
/// an existing path. It is flushed to the OS before replacement. A failed write
/// or rename leaves the previous destination untouched and attempts to remove
/// the temporary file.
pub fn write_file_atomic(path: impl AsRef<Path>, bytes: &[u8]) -> io::Result<()> {
    let destination = path.as_ref();
    let file_name = destination.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "destination must name a file")
    })?;
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    for _ in 0..128 {
        let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = OsString::from(".");
        temporary_name.push(file_name);
        temporary_name.push(format!(".chronos-tmp-{}-{sequence}", std::process::id()));
        let temporary_path = parent.join(temporary_name);

        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };

        let write_result = file.write_all(bytes).and_then(|()| file.sync_all());
        drop(file);

        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }

        if let Err(error) = fs::rename(&temporary_path, destination) {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }

        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique CHRONOS temporary file",
    ))
}
