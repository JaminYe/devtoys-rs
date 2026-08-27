use std::io::{Read, Write};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GzipMode {
    #[default]
    Compress,
    Decompress,
}

impl GzipMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Compress" => Some(Self::Compress),
            "Decompress" => Some(Self::Decompress),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GzipError {
    #[error("非法 GZip")]
    InvalidGzip,
}

pub fn convert(input: &str, mode: GzipMode) -> Result<String, GzipError> {
    match mode {
        GzipMode::Compress => compress(input),
        GzipMode::Decompress => decompress(input),
    }
}

pub fn compress(input: &str) -> Result<String, GzipError> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(input.as_bytes())
        .map_err(|_| GzipError::InvalidGzip)?;
    let bytes = encoder.finish().map_err(|_| GzipError::InvalidGzip)?;
    Ok(STANDARD.encode(bytes))
}

pub fn decompress(input: &str) -> Result<String, GzipError> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = STANDARD
        .decode(cleaned.as_bytes())
        .map_err(|_| GzipError::InvalidGzip)?;
    let mut decoder = GzDecoder::new(bytes.as_slice());
    let mut out = String::new();
    decoder
        .read_to_string(&mut out)
        .map_err(|_| GzipError::InvalidGzip)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_hello() {
        let encoded = compress("hello").unwrap();
        let got = decompress(&encoded).unwrap();
        assert_eq!(got, "hello");
    }

    #[test]
    fn garbage_decompress_is_err_without_input() {
        let err = decompress("not-gzip-payload").unwrap_err();
        assert_eq!(err, GzipError::InvalidGzip);
        let message = err.to_string();
        assert!(!message.contains("not-gzip-payload"), "error must not include user input");
    }
}
