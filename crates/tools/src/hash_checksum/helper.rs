use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};

const CHUNK_SIZE: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl HashAlgorithm {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Md5" => Some(Self::Md5),
            "Sha1" => Some(Self::Sha1),
            "Sha256" => Some(Self::Sha256),
            "Sha384" => Some(Self::Sha384),
            "Sha512" => Some(Self::Sha512),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Md5 => "Md5",
            Self::Sha1 => "Sha1",
            Self::Sha256 => "Sha256",
            Self::Sha384 => "Sha384",
            Self::Sha512 => "Sha512",
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HashError {
    #[error("无法读取文件")]
    FileRead,
    #[error("未知算法")]
    UnknownAlgorithm,
    #[error("已取消")]
    Cancelled,
}

/// Cooperative cancel flag. Checked between chunks, not mid-`read`.
#[derive(Clone, Debug)]
pub struct HashCancel {
    flag: Arc<AtomicBool>,
}

impl HashCancel {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

impl Default for HashCancel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HashProgress {
    pub bytes_read: u64,
    pub total_bytes: Option<u64>,
}

impl HashProgress {
    pub fn fraction(&self) -> Option<f32> {
        let total = self.total_bytes.filter(|&total| total > 0)?;
        Some((self.bytes_read as f64 / total as f64).min(1.0) as f32)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct HashTaskResult {
    pub generation: u64,
    pub outcome: Result<String, HashError>,
}

/// Active-task generation. `begin` invalidates in-flight results from older tasks.
#[derive(Debug, Default)]
pub struct HashSession {
    generation: u64,
}

impl HashSession {
    pub fn new() -> Self {
        Self { generation: 0 }
    }

    pub fn begin(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    pub fn accept(&self, result: HashTaskResult) -> Option<Result<String, HashError>> {
        if result.generation == self.generation {
            Some(result.outcome)
        } else {
            None
        }
    }
}

/// Production code wraps `Read`; tests inject gated or fixed-size chunk sources.
pub trait ByteSource {
    fn total_len(&self) -> Option<u64>;
    fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, HashError>;
}

struct ReadSource<R> {
    reader: R,
    total: Option<u64>,
}

impl<R: Read> ByteSource for ReadSource<R> {
    fn total_len(&self) -> Option<u64> {
        self.total
    }

    fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, HashError> {
        self.reader.read(buf).map_err(|_| HashError::FileRead)
    }
}

pub fn is_file_input(input: &str, as_file: bool) -> bool {
    as_file || Path::new(input).is_file()
}

/// Hash UTF-8 text, or file bytes when `as_file` or the path exists.
pub fn compute_hash(
    input: &str,
    algorithm: HashAlgorithm,
    hmac_key: Option<&str>,
    uppercase: bool,
    as_file: bool,
) -> Result<String, HashError> {
    hash_input(
        input,
        algorithm,
        hmac_key,
        uppercase,
        as_file,
        0,
        &HashCancel::new(),
        |_| {},
    )
    .outcome
}

pub fn hash_input(
    input: &str,
    algorithm: HashAlgorithm,
    hmac_key: Option<&str>,
    uppercase: bool,
    as_file: bool,
    generation: u64,
    cancel: &HashCancel,
    on_progress: impl FnMut(HashProgress),
) -> HashTaskResult {
    if is_file_input(input, as_file) {
        hash_path(
            Path::new(input),
            algorithm,
            hmac_key.map(str::as_bytes),
            uppercase,
            generation,
            cancel,
            on_progress,
        )
    } else {
        let bytes = input.as_bytes();
        hash_reader(
            std::io::Cursor::new(bytes),
            Some(bytes.len() as u64),
            algorithm,
            hmac_key.map(str::as_bytes),
            uppercase,
            generation,
            cancel,
            on_progress,
        )
    }
}

pub fn hash_path(
    path: &Path,
    algorithm: HashAlgorithm,
    hmac_key: Option<&[u8]>,
    uppercase: bool,
    generation: u64,
    cancel: &HashCancel,
    on_progress: impl FnMut(HashProgress),
) -> HashTaskResult {
    if cancel.is_cancelled() {
        return HashTaskResult {
            generation,
            outcome: Err(HashError::Cancelled),
        };
    }
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => {
            return HashTaskResult {
                generation,
                outcome: Err(HashError::FileRead),
            };
        }
    };
    let total = file.metadata().ok().map(|meta| meta.len());
    hash_reader(
        file,
        total,
        algorithm,
        hmac_key,
        uppercase,
        generation,
        cancel,
        on_progress,
    )
}

pub fn hash_reader<R: Read>(
    reader: R,
    total_bytes: Option<u64>,
    algorithm: HashAlgorithm,
    hmac_key: Option<&[u8]>,
    uppercase: bool,
    generation: u64,
    cancel: &HashCancel,
    on_progress: impl FnMut(HashProgress),
) -> HashTaskResult {
    hash_source(
        ReadSource {
            reader,
            total: total_bytes,
        },
        algorithm,
        hmac_key,
        uppercase,
        generation,
        cancel,
        on_progress,
    )
}

pub fn hash_source<S: ByteSource>(
    source: S,
    algorithm: HashAlgorithm,
    hmac_key: Option<&[u8]>,
    uppercase: bool,
    generation: u64,
    cancel: &HashCancel,
    on_progress: impl FnMut(HashProgress),
) -> HashTaskResult {
    match hmac_key {
        None => match algorithm {
            HashAlgorithm::Md5 => {
                stream_digest::<Md5>(source, uppercase, generation, cancel, on_progress)
            }
            HashAlgorithm::Sha1 => {
                stream_digest::<Sha1>(source, uppercase, generation, cancel, on_progress)
            }
            HashAlgorithm::Sha256 => {
                stream_digest::<Sha256>(source, uppercase, generation, cancel, on_progress)
            }
            HashAlgorithm::Sha384 => {
                stream_digest::<Sha384>(source, uppercase, generation, cancel, on_progress)
            }
            HashAlgorithm::Sha512 => {
                stream_digest::<Sha512>(source, uppercase, generation, cancel, on_progress)
            }
        },
        Some(key) => match algorithm {
            HashAlgorithm::Md5 => stream_mac(
                Hmac::<Md5>::new_from_slice(key).unwrap(),
                source,
                uppercase,
                generation,
                cancel,
                on_progress,
            ),
            HashAlgorithm::Sha1 => stream_mac(
                Hmac::<Sha1>::new_from_slice(key).unwrap(),
                source,
                uppercase,
                generation,
                cancel,
                on_progress,
            ),
            HashAlgorithm::Sha256 => stream_mac(
                Hmac::<Sha256>::new_from_slice(key).unwrap(),
                source,
                uppercase,
                generation,
                cancel,
                on_progress,
            ),
            HashAlgorithm::Sha384 => stream_mac(
                Hmac::<Sha384>::new_from_slice(key).unwrap(),
                source,
                uppercase,
                generation,
                cancel,
                on_progress,
            ),
            HashAlgorithm::Sha512 => stream_mac(
                Hmac::<Sha512>::new_from_slice(key).unwrap(),
                source,
                uppercase,
                generation,
                cancel,
                on_progress,
            ),
        },
    }
}

pub fn checksum_matches(actual: &str, expected: &str) -> bool {
    actual.eq_ignore_ascii_case(expected.trim())
}

/// Existing file → read text and trim; otherwise treat `expected` as a literal.
pub fn resolve_checksum_expected(expected: &str) -> String {
    let trimmed = expected.trim();
    let path = Path::new(trimmed);
    if path.is_file() {
        fs::read_to_string(path)
            .map(|contents| contents.trim().to_string())
            .unwrap_or_else(|_| trimmed.to_string())
    } else {
        trimmed.to_string()
    }
}

pub fn checksum_matches_input(actual: &str, expected: &str) -> bool {
    checksum_matches(actual, &resolve_checksum_expected(expected))
}

fn stream_digest<D: Digest>(
    mut source: impl ByteSource,
    uppercase: bool,
    generation: u64,
    cancel: &HashCancel,
    mut on_progress: impl FnMut(HashProgress),
) -> HashTaskResult {
    let mut hasher = D::new();
    match pump(&mut source, cancel, &mut on_progress, |chunk| {
        hasher.update(chunk);
    }) {
        Ok(()) => HashTaskResult {
            generation,
            outcome: Ok(format_hex(hasher.finalize().as_ref(), uppercase)),
        },
        Err(err) => HashTaskResult {
            generation,
            outcome: Err(err),
        },
    }
}

fn stream_mac(
    mut mac: impl Mac,
    mut source: impl ByteSource,
    uppercase: bool,
    generation: u64,
    cancel: &HashCancel,
    mut on_progress: impl FnMut(HashProgress),
) -> HashTaskResult {
    match pump(&mut source, cancel, &mut on_progress, |chunk| {
        mac.update(chunk);
    }) {
        Ok(()) => HashTaskResult {
            generation,
            outcome: Ok(format_hex(mac.finalize().into_bytes().as_ref(), uppercase)),
        },
        Err(err) => HashTaskResult {
            generation,
            outcome: Err(err),
        },
    }
}

fn pump(
    source: &mut impl ByteSource,
    cancel: &HashCancel,
    on_progress: &mut impl FnMut(HashProgress),
    mut update: impl FnMut(&[u8]),
) -> Result<(), HashError> {
    let total_bytes = source.total_len();
    let mut bytes_read = 0u64;
    let mut buf = [0u8; CHUNK_SIZE];
    loop {
        if cancel.is_cancelled() {
            return Err(HashError::Cancelled);
        }
        let n = source.read_chunk(&mut buf)?;
        if n == 0 {
            break;
        }
        update(&buf[..n]);
        bytes_read += n as u64;
        on_progress(HashProgress {
            bytes_read,
            total_bytes,
        });
    }
    if cancel.is_cancelled() {
        Err(HashError::Cancelled)
    } else {
        Ok(())
    }
}

fn format_hex(digest: &[u8], uppercase: bool) -> String {
    if uppercase {
        hex::encode_upper(digest)
    } else {
        hex::encode(digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Cursor, Read};
    use std::sync::atomic::AtomicUsize;
    use std::sync::mpsc;
    use std::time::{SystemTime, UNIX_EPOCH};

    const MD5_EMPTY: &str = "d41d8cd98f00b204e9800998ecf8427e";
    /// RFC 1321 MD5("abc")
    const MD5_ABC: &str = "900150983cd24fb0d6963f7d28e17f72";
    const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const HMAC_SHA256_ABC_KEY: &str =
        "9c196e32dc0175f86f4b1cb89289d6619de6bee699e4c378e68309ed97a1a6ab";
    /// Independent .NET MD5 of 200_000 ASCII 'a' bytes.
    const MD5_200K_A: &str = "561b1994f6baacd6e5eaf4baaa12849f";

    struct CountingSource {
        data: Vec<u8>,
        pos: usize,
        chunk: usize,
        reads: Arc<AtomicUsize>,
        dropped: Arc<AtomicBool>,
    }

    impl ByteSource for CountingSource {
        fn total_len(&self) -> Option<u64> {
            Some(self.data.len() as u64)
        }

        fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, HashError> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if self.pos >= self.data.len() {
                return Ok(0);
            }
            let n = (self.data.len() - self.pos).min(self.chunk).min(buf.len());
            buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
            self.pos += n;
            Ok(n)
        }
    }

    impl Drop for CountingSource {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    struct GatedSource {
        data: Vec<u8>,
        sent: bool,
        gate: mpsc::Receiver<()>,
    }

    impl ByteSource for GatedSource {
        fn total_len(&self) -> Option<u64> {
            Some(self.data.len() as u64)
        }

        fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, HashError> {
            if self.sent {
                return Ok(0);
            }
            let _ = self.gate.recv();
            let n = self.data.len().min(buf.len());
            buf[..n].copy_from_slice(&self.data[..n]);
            self.sent = true;
            Ok(n)
        }
    }

    struct FailingAfterFirst {
        sent: bool,
    }

    impl ByteSource for FailingAfterFirst {
        fn total_len(&self) -> Option<u64> {
            Some(4)
        }

        fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, HashError> {
            if self.sent {
                return Err(HashError::FileRead);
            }
            buf[0] = b'x';
            self.sent = true;
            Ok(1)
        }
    }

    struct FailRead;

    impl Read for FailRead {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("fail"))
        }
    }

    struct TempPath(std::path::PathBuf);

    impl TempPath {
        fn create(name: &str, bytes: &[u8]) -> Self {
            let path = std::env::temp_dir().join(format!(
                "devtoys-hash-{name}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::write(&path, bytes).unwrap();
            Self(path)
        }
    }

    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn counting(data: &[u8], chunk: usize) -> (CountingSource, Arc<AtomicUsize>, Arc<AtomicBool>) {
        let reads = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicBool::new(false));
        let source = CountingSource {
            data: data.to_vec(),
            pos: 0,
            chunk,
            reads: reads.clone(),
            dropped: dropped.clone(),
        };
        (source, reads, dropped)
    }

    #[test]
    fn md5_of_empty_string() {
        let got = compute_hash("", HashAlgorithm::Md5, None, false, false).unwrap();
        assert_eq!(got, MD5_EMPTY);
        assert_eq!(got.len(), 32);
        assert!(got.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(got, got.to_ascii_lowercase());
    }

    #[test]
    fn sha256_of_abc() {
        let got = compute_hash("abc", HashAlgorithm::Sha256, None, false, false).unwrap();
        assert_eq!(got, SHA256_ABC);
        assert_eq!(got.len(), 64);
    }

    #[test]
    fn hmac_sha256_of_abc_with_key() {
        let got = compute_hash("abc", HashAlgorithm::Sha256, Some("key"), false, false).unwrap();
        assert_eq!(got, HMAC_SHA256_ABC_KEY);
        assert_eq!(got.len(), 64);
        assert!(got.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')));
    }

    #[test]
    fn uppercase_hex() {
        let got = compute_hash("", HashAlgorithm::Md5, None, true, false).unwrap();
        assert_eq!(got, MD5_EMPTY.to_ascii_uppercase());
        assert_eq!(got, got.to_ascii_uppercase());
    }

    #[test]
    fn compare_is_case_insensitive() {
        assert!(checksum_matches(MD5_EMPTY, &MD5_EMPTY.to_ascii_uppercase()));
        assert!(!checksum_matches(MD5_EMPTY, "deadbeef"));
    }

    #[test]
    fn hashes_existing_file_bytes() {
        let path = std::env::temp_dir().join("devtoys-hash-checksum-empty.bin");
        std::fs::write(&path, b"").unwrap();
        let got = compute_hash(
            path.to_str().unwrap(),
            HashAlgorithm::Md5,
            None,
            false,
            false,
        )
        .unwrap();
        assert_eq!(got, MD5_EMPTY);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_with_as_file_errors_without_contents() {
        let secret = "this-must-not-appear-in-errors-xyz";
        let path = std::env::temp_dir().join("devtoys-hash-missing-no-such-file.bin");
        let _ = std::fs::remove_file(&path);
        let err = compute_hash(
            path.to_str().unwrap(),
            HashAlgorithm::Md5,
            None,
            false,
            true,
        )
        .unwrap_err();
        assert_eq!(err, HashError::FileRead);
        let message = err.to_string();
        assert!(!message.contains(secret));
        assert!(!message.contains("this-must-not-appear"));
        assert_eq!(message, "无法读取文件");
    }

    #[test]
    fn unknown_algorithm_parse() {
        assert!(HashAlgorithm::parse("md5").is_none());
        assert_eq!(HashAlgorithm::parse("Md5"), Some(HashAlgorithm::Md5));
    }

    #[test]
    fn chunked_reader_md5_matches_rfc_abc() {
        let (source, reads, dropped) = counting(b"abc", 1);
        let cancel = HashCancel::new();
        let result = hash_source(source, HashAlgorithm::Md5, None, false, 3, &cancel, |_| {});
        assert_eq!(result.generation, 3);
        assert_eq!(result.outcome, Ok(MD5_ABC.to_string()));
        assert!(reads.load(Ordering::SeqCst) >= 3);
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn chunked_hmac_matches_independent_vector() {
        let (source, _, _) = counting(b"abc", 1);
        let result = hash_source(
            source,
            HashAlgorithm::Sha256,
            Some(b"key"),
            false,
            1,
            &HashCancel::new(),
            |_| {},
        );
        assert_eq!(result.outcome, Ok(HMAC_SHA256_ABC_KEY.to_string()));
    }

    #[test]
    fn progress_reports_each_chunk() {
        let (source, _, _) = counting(b"abcdef", 2);
        let mut seen = Vec::new();
        let result = hash_source(
            source,
            HashAlgorithm::Md5,
            None,
            false,
            1,
            &HashCancel::new(),
            |progress| seen.push(progress),
        );
        assert!(result.outcome.is_ok());
        assert_eq!(
            seen,
            [
                HashProgress {
                    bytes_read: 2,
                    total_bytes: Some(6)
                },
                HashProgress {
                    bytes_read: 4,
                    total_bytes: Some(6)
                },
                HashProgress {
                    bytes_read: 6,
                    total_bytes: Some(6)
                },
            ]
        );
        assert_eq!(seen.last().unwrap().fraction(), Some(1.0));
    }

    #[test]
    fn cancel_after_first_chunk_stops_reads_and_drops_source() {
        let (source, reads, dropped) = counting(b"abcdef", 2);
        let cancel = HashCancel::new();
        let result = hash_source(
            source,
            HashAlgorithm::Md5,
            None,
            false,
            9,
            &cancel,
            |progress| {
                if progress.bytes_read >= 2 {
                    cancel.cancel();
                }
            },
        );
        assert_eq!(result.generation, 9);
        assert_eq!(result.outcome, Err(HashError::Cancelled));
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn pre_cancelled_task_does_not_read() {
        let (source, reads, dropped) = counting(b"abc", 1);
        let cancel = HashCancel::new();
        cancel.cancel();
        let result = hash_source(source, HashAlgorithm::Md5, None, false, 4, &cancel, |_| {
            unreachable!("progress should not fire")
        });
        assert_eq!(result.outcome, Err(HashError::Cancelled));
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[test]
    fn session_drops_stale_generation() {
        let mut session = HashSession::new();
        let old_gen = session.begin();
        let current_gen = session.begin();
        let cancel = HashCancel::new();
        let current = hash_reader(
            Cursor::new(b"abc"),
            Some(3),
            HashAlgorithm::Md5,
            None,
            false,
            current_gen,
            &cancel,
            |_| {},
        );
        assert_eq!(session.accept(current).unwrap().unwrap(), MD5_ABC);

        let stale = hash_reader(
            Cursor::new(b"zzz"),
            Some(3),
            HashAlgorithm::Md5,
            None,
            false,
            old_gen,
            &cancel,
            |_| {},
        );
        assert!(session.accept(stale).is_none());
    }

    #[test]
    fn gated_old_task_cannot_overwrite_newer_result() {
        let mut session = HashSession::new();
        let old_gen = session.begin();
        let (gate_tx, gate_rx) = mpsc::channel();
        let cancel_old = HashCancel::new();
        let old_handle = std::thread::spawn(move || {
            hash_source(
                GatedSource {
                    data: b"zzz".to_vec(),
                    sent: false,
                    gate: gate_rx,
                },
                HashAlgorithm::Md5,
                None,
                false,
                old_gen,
                &cancel_old,
                |_| {},
            )
        });

        let current_gen = session.begin();
        let current = hash_reader(
            Cursor::new(b"abc"),
            Some(3),
            HashAlgorithm::Md5,
            None,
            false,
            current_gen,
            &HashCancel::new(),
            |_| {},
        );
        assert_eq!(session.accept(current).unwrap().unwrap(), MD5_ABC);

        gate_tx.send(()).unwrap();
        let stale = old_handle.join().unwrap();
        assert_eq!(stale.generation, old_gen);
        assert!(stale.outcome.is_ok());
        assert!(session.accept(stale).is_none());
    }

    #[test]
    fn streaming_temp_file_matches_independent_digest() {
        let payload = vec![b'a'; 200_000];
        let temp = TempPath::create("200k-a", &payload);
        let cancel = HashCancel::new();
        let mut last = HashProgress::default();
        let result = hash_path(
            &temp.0,
            HashAlgorithm::Md5,
            None,
            false,
            2,
            &cancel,
            |progress| last = progress,
        );
        assert_eq!(result.outcome.as_deref(), Ok(MD5_200K_A));
        assert_eq!(
            last,
            HashProgress {
                bytes_read: 200_000,
                total_bytes: Some(200_000)
            }
        );
        assert_eq!(
            compute_hash(
                temp.0.to_str().unwrap(),
                HashAlgorithm::Md5,
                None,
                false,
                true
            )
            .unwrap(),
            MD5_200K_A
        );
    }

    #[test]
    fn cancel_releases_file_without_digest() {
        let payload = vec![b'a'; 200_000];
        let temp = TempPath::create("cancel-release", &payload);
        let cancel = HashCancel::new();
        let mut first_read = 0;
        let result = hash_path(
            &temp.0,
            HashAlgorithm::Md5,
            None,
            false,
            1,
            &cancel,
            |progress| {
                if first_read == 0 {
                    first_read = progress.bytes_read;
                    cancel.cancel();
                }
            },
        );
        assert_eq!(result.outcome, Err(HashError::Cancelled));
        assert!(first_read > 0);
        assert!(first_read < 200_000);
        fs::write(&temp.0, b"replaced").unwrap();
        assert_eq!(fs::read(&temp.0).unwrap(), b"replaced");
    }

    #[test]
    fn read_error_mid_stream_is_file_read() {
        let result = hash_source(
            FailingAfterFirst { sent: false },
            HashAlgorithm::Md5,
            None,
            false,
            1,
            &HashCancel::new(),
            |_| {},
        );
        assert_eq!(result.outcome, Err(HashError::FileRead));
    }

    #[test]
    fn reader_io_error_is_file_read() {
        let result = hash_reader(
            FailRead,
            None,
            HashAlgorithm::Md5,
            None,
            false,
            1,
            &HashCancel::new(),
            |_| {},
        );
        assert_eq!(result.outcome, Err(HashError::FileRead));
    }

    #[test]
    fn checksum_file_contents_match_not_filename() {
        let temp = TempPath::create("digest", format!("\t{MD5_ABC}\r\n").as_bytes());
        let path = temp.0.to_str().unwrap();
        assert_ne!(path, MD5_ABC);
        assert!(
            !checksum_matches(MD5_ABC, path),
            "raw compare must use the path string, not file contents"
        );
        assert_eq!(resolve_checksum_expected(path), MD5_ABC);
        assert!(checksum_matches_input(MD5_ABC, path));
        assert!(checksum_matches_input(&MD5_ABC.to_ascii_uppercase(), path));
    }

    #[test]
    fn checksum_literal_hex_still_matches() {
        assert!(checksum_matches_input(MD5_ABC, MD5_ABC));
        assert!(checksum_matches_input(
            MD5_ABC,
            &format!("  {}  ", MD5_ABC.to_ascii_uppercase())
        ));
        assert!(!checksum_matches_input(MD5_ABC, "deadbeef"));
    }

    #[test]
    fn missing_checksum_path_is_literal_and_does_not_panic() {
        let missing = std::env::temp_dir().join(format!(
            "devtoys-hash-missing-digest-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_file(&missing);
        let path = missing.to_str().unwrap();
        assert!(!Path::new(path).is_file());
        assert_eq!(resolve_checksum_expected(path), path);
        assert!(!checksum_matches_input(MD5_ABC, path));
    }

    #[test]
    fn hash_path_matches_compute_hash_as_file_on_same_bytes() {
        let temp = TempPath::create("small-abc", b"abc");
        let path = temp.0.to_str().unwrap();
        let via_path = hash_path(
            &temp.0,
            HashAlgorithm::Md5,
            None,
            false,
            1,
            &HashCancel::new(),
            |_| {},
        )
        .outcome
        .unwrap();
        let via_as_file = compute_hash(path, HashAlgorithm::Md5, None, false, true).unwrap();
        let via_text = compute_hash("abc", HashAlgorithm::Md5, None, false, false).unwrap();
        assert_eq!(via_path, MD5_ABC);
        assert_eq!(via_as_file, MD5_ABC);
        assert_eq!(via_text, MD5_ABC);
    }
}
