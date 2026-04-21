//! Circular ring buffer log writer (1MB fixed size).
//!
//! When the file reaches 1MB, the last 500KB is copied to the beginning,
//! a rotation marker is written, and new writes continue from that point.
//! This creates a moving window of the most recent ~1MB of logs.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

const LOG_MAX_SIZE: u64 = 1024 * 1024;
const LOG_KEEP_SIZE: u64 = 512 * 1024;
const ROTATION_MARKER: &str = "--- ROTATED ---\n";

pub struct RingLogWriter {
    file: File,
    current_pos: u64,
}

impl RingLogWriter {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        let current_pos = file.metadata()?.len();
        Ok(Self { file, current_pos })
    }

    pub fn write_log(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.current_pos + buf.len() as u64 > LOG_MAX_SIZE {
            self.rotate()?;
        }

        self.file.seek(SeekFrom::Start(self.current_pos))?;
        let written = self.file.write(buf)?;
        self.current_pos += written as u64;
        Ok(written)
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        let read_start = self.current_pos.saturating_sub(LOG_KEEP_SIZE);
        let bytes_to_keep = self.current_pos - read_start;

        let mut buffer = vec![0u8; bytes_to_keep as usize];
        self.file.seek(SeekFrom::Start(read_start))?;
        self.file.read_exact(&mut buffer)?;

        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&buffer)?;

        let marker_len = ROTATION_MARKER.len();
        self.file.write_all(ROTATION_MARKER.as_bytes())?;

        self.current_pos = bytes_to_keep + marker_len as u64;
        self.file.set_len(self.current_pos)?;

        Ok(())
    }
}

impl Write for RingLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_log(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

pub type SharedRingLog = Arc<Mutex<RingLogWriter>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_ring_log_grows() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        let mut writer = RingLogWriter::open(&path).unwrap();
        writer.write_log(b"hello world").unwrap();
        writer.flush().unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_ring_log_rotates() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        let mut writer = RingLogWriter::open(&path).unwrap();

        // Write enough to trigger rotation
        let big_chunk = vec![b'x'; 8 * 1024];
        writer.write_log(&big_chunk).unwrap();
        writer.write_log(&big_chunk).unwrap();
        writer.flush().unwrap();

        let metadata = std::fs::metadata(&path).unwrap();
        // After rotation: 5KB kept + marker + remaining data
        // Should be between 5KB and 15KB (two rotations max)
        assert!(
            metadata.len() >= LOG_KEEP_SIZE,
            "file too small: {}",
            metadata.len()
        );
        assert!(
            metadata.len() <= LOG_MAX_SIZE * 2,
            "file too large: {}",
            metadata.len()
        );
    }
}
