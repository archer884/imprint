use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::{cmp, fs, io};
use std::hash::{Hash, Hasher};
use std::convert::TryInto;

/// Sample size for head and tail segments.
///
/// This sample is 512kb in length, which should be more than sufficient.
const SAMPLE_SIZE: i64 = 0x80000;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Imprint {
    meta: Metadata,
    head: Box<[u8]>,
    tail: Option<Box<[u8]>>,
}

impl Imprint {
    pub fn path(&self) -> &Path {
        &self.meta.path
    }

    pub fn len(&self) -> u64 {
        self.meta.length
    }
}

/// Represents file metadata relevant to an Imprint.
///
/// This struct represents a file path and its length. Other metadata items are ignored.
#[derive(Clone, Debug, Eq)]
pub struct Metadata {
    path: PathBuf,
    length: u64,
}

impl Metadata {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> u64 {
        self.length
    }

    pub fn from_path(path: impl AsRef<Path> + Into<PathBuf>) -> io::Result<Self> {
        let metadata = fs::metadata(path.as_ref())?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Path does not reference a file",
            ));
        }

        Ok(Metadata {
            path: path.into(),
            length: metadata.len(),
        })
    }
}

impl TryInto<Imprint> for Metadata {
    type Error = io::Error;

    fn try_into(self) -> io::Result<Imprint> {
        use std::fs::File;

        let mut reader = File::open(&self.path)?;
        let mut buffer = vec![0; SAMPLE_SIZE as usize].into_boxed_slice();

        let head = hash_from_start(
            &mut reader,
            &mut buffer[..get_head_length(self.length) as usize],
        )?;

        let tail = get_tail_length(self.length)
            .map(|len| hash_from_end(&mut reader, &mut buffer[..len as usize], len))
            .transpose()?;

        Ok(Imprint {
            meta: self,
            head,
            tail,
        })
    }
}

// Metadata for a given file is considered equivalent if both files have the same length.
impl PartialEq for Metadata {
    fn eq(&self, other: &Self) -> bool {
        self.length == other.length
    }
}

// Because we have a custom PartialEq implementation, we need a custom Hash implementation.
impl Hash for Metadata {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.length.hash(h);
    }
}

fn hash_from_start(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<Box<[u8]>> {
    reader.read_exact(buffer)?;
    Ok(hash(buffer))
}

fn hash_from_end(
    reader: &mut (impl Read + Seek),
    buffer: &mut [u8],
    offset: i64,
) -> io::Result<Box<[u8]>> {
    reader.seek(SeekFrom::End(-offset))?;
    reader.read_exact(buffer)?;
    Ok(hash(buffer))
}

fn hash(s: &[u8]) -> Box<[u8]> {
    use sha2::{digest::Digest, Sha256};

    // I can't stand GenericArray.
    let mut hasher = Sha256::new();
    hasher.input(s);
    hasher
        .result()
        .into_iter()
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn get_head_length(len: u64) -> i64 {
    if len > i64::max_value() as u64 {
        SAMPLE_SIZE
    } else {
        cmp::min(len as i64, SAMPLE_SIZE)
    }
}

fn get_tail_length(len: u64) -> Option<i64> {
    if len > i64::max_value() as u64 {
        return Some(SAMPLE_SIZE);
    }

    match len as i64 - SAMPLE_SIZE {
        len if len <= 0 => None,
        len => Some(cmp::min(len, SAMPLE_SIZE)),
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read, Seek, SeekFrom};

    // This test demonstrates the correct use of SeekFrom. It does not relate to any library
    // function; it is just here as a reference.
    #[test]
    fn seek() -> io::Result<()> {
        let message = "In the beginning, God created the heaven and the earth.";
        let mut reader = Cursor::new(&message);
        let mut buf = String::new();

        reader.seek(SeekFrom::End(-6))?;
        reader.read_to_string(&mut buf)?;

        assert_eq!("earth.", buf);
        Ok(())
    }
}
