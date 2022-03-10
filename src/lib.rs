use std::{
    fmt::Display,
    fs,
    io::{self, BufReader},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use blake3::{Hash, Hasher};

/// Sample size for head and tail segments.
///
/// This sample is 512kb in length, which should be more than sufficient.
const SAMPLE_SIZE: u64 = 0x80000;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Imprint {
    head: Hash,
    tail: Option<Hash>,
}

impl Imprint {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        use std::fs::File;

        let path = path.as_ref();
        let meta = fs::metadata(path)?;
        if !meta.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "received a directory when expecting a file",
            ));
        }

        let len = meta.len();
        let mut reader =
            File::open(path).map(|f| BufReader::with_capacity(SAMPLE_SIZE as usize, f))?;
        let mut buffer = vec![0; SAMPLE_SIZE as usize].into_boxed_slice();

        Ok(Imprint {
            head: hash_head(&mut reader, &mut buffer, len)?,
            tail: hash_tail(&mut reader, &mut buffer, len)?,
        })
    }
}

impl Display for Imprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.head.fmt(f)
    }
}

fn hash_head(reader: &mut impl Read, buf: &mut [u8], len: u64) -> io::Result<Hash> {
    let len = len.min(SAMPLE_SIZE) as usize;
    let buf = &mut buf[..len];
    reader.read_exact(buf)?;
    Ok(Hasher::new().update(buf).finalize())
}

fn hash_tail(
    reader: &mut (impl Read + Seek),
    buf: &mut [u8],
    len: u64,
) -> io::Result<Option<Hash>> {
    let tail_len = len.saturating_sub(SAMPLE_SIZE);
    if tail_len == 0 {
        return Ok(None);
    }

    let len = tail_len.min(SAMPLE_SIZE) as usize;
    let buf = &mut buf[..len];
    reader.seek(SeekFrom::End(-(len as i64)))?;
    reader.read_exact(buf)?;
    Ok(Some(Hasher::new().update(buf).finalize()))
}
