use std::fs;
use std::path::Path;

use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};

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
}

/// Hash UTF-8 text, or file bytes when `as_file` or the path exists.
pub fn compute_hash(
    input: &str,
    algorithm: HashAlgorithm,
    hmac_key: Option<&str>,
    uppercase: bool,
    as_file: bool,
) -> Result<String, HashError> {
    let data = load_bytes(input, as_file)?;
    let digest = match hmac_key {
        Some(key) => hmac_digest(algorithm, &data, key.as_bytes()),
        None => plain_digest(algorithm, &data),
    };
    Ok(if uppercase {
        hex::encode_upper(digest)
    } else {
        hex::encode(digest)
    })
}

pub fn checksum_matches(actual: &str, expected: &str) -> bool {
    actual.eq_ignore_ascii_case(expected.trim())
}

fn load_bytes(input: &str, as_file: bool) -> Result<Vec<u8>, HashError> {
    let path = Path::new(input);
    if as_file || path.is_file() {
        fs::read(path).map_err(|_| HashError::FileRead)
    } else {
        Ok(input.as_bytes().to_vec())
    }
}

fn plain_digest(algorithm: HashAlgorithm, data: &[u8]) -> Vec<u8> {
    match algorithm {
        HashAlgorithm::Md5 => Md5::digest(data).to_vec(),
        HashAlgorithm::Sha1 => Sha1::digest(data).to_vec(),
        HashAlgorithm::Sha256 => Sha256::digest(data).to_vec(),
        HashAlgorithm::Sha384 => Sha384::digest(data).to_vec(),
        HashAlgorithm::Sha512 => Sha512::digest(data).to_vec(),
    }
}

fn hmac_digest(algorithm: HashAlgorithm, data: &[u8], key: &[u8]) -> Vec<u8> {
    match algorithm {
        HashAlgorithm::Md5 => finish_hmac(Hmac::<Md5>::new_from_slice(key).unwrap(), data),
        HashAlgorithm::Sha1 => finish_hmac(Hmac::<Sha1>::new_from_slice(key).unwrap(), data),
        HashAlgorithm::Sha256 => finish_hmac(Hmac::<Sha256>::new_from_slice(key).unwrap(), data),
        HashAlgorithm::Sha384 => finish_hmac(Hmac::<Sha384>::new_from_slice(key).unwrap(), data),
        HashAlgorithm::Sha512 => finish_hmac(Hmac::<Sha512>::new_from_slice(key).unwrap(), data),
    }
}

fn finish_hmac(mut mac: impl Mac, data: &[u8]) -> Vec<u8> {
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MD5_EMPTY: &str = "d41d8cd98f00b204e9800998ecf8427e";
    const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const HMAC_SHA256_ABC_KEY: &str =
        "9c196e32dc0175f86f4b1cb89289d6619de6bee699e4c378e68309ed97a1a6ab";

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
        let err = compute_hash(path.to_str().unwrap(), HashAlgorithm::Md5, None, false, true)
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
}
