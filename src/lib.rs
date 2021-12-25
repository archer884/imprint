use std::{
    cmp,
    fmt::Display,
    fs, io,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
};

use blake3::{Hash, Hasher};

/// Sample size for head and tail segments.
///
/// This sample is 512kb in length, which should be more than sufficient.
const SAMPLE_SIZE: i64 = 0x80000;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Imprint {
    head: Hash,
    tail: Option<Hash>,
}

impl Imprint {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        use std::fs::File;

        let len = read_len(path.as_ref())?;
        let mut reader = File::open(path)?;
        let mut buffer = vec![0; SAMPLE_SIZE as usize].into_boxed_slice();

        let head = hash_from_start(&mut reader, &mut buffer[..get_head_length(len) as usize])?;
        let tail = get_tail_length(len)
            .map(|len| hash_from_end(&mut reader, &mut buffer[..len as usize], len))
            .transpose()?;

        Ok(Imprint { head, tail })
    }
}

impl Display for Imprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.head.fmt(f)
    }
}

fn read_len(path: &Path) -> io::Result<u64> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Path does not reference a file",
        ));
    }
    Ok(metadata.len())
}

fn hash_from_start(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<Hash> {
    reader.read_exact(buffer)?;
    Ok(hash(buffer))
}

fn hash_from_end(
    reader: &mut (impl Read + Seek),
    buffer: &mut [u8],
    offset: i64,
) -> io::Result<Hash> {
    reader.seek(SeekFrom::End(-offset))?;
    reader.read_exact(buffer)?;
    Ok(hash(buffer))
}

fn hash(s: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    let mut cursor = Cursor::new(s);
    io::copy(&mut cursor, &mut hasher).unwrap();
    hasher.finalize()
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
