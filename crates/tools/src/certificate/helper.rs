use sha2::{Digest, Sha256};
use x509_parser::prelude::*;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CertificateError {
    #[error("非法证书")]
    InvalidCertificate,
    #[error("无法解析 PFX")]
    UnsupportedPfx,
}

pub fn decode_certificate(
    input: &[u8],
    password: Option<&str>,
) -> Result<String, CertificateError> {
    if looks_like_pfx(input) {
        return Err(CertificateError::UnsupportedPfx);
    }
    match parse_pem_or_der(input) {
        Ok(text) => Ok(text),
        Err(err) => {
            if password.is_some() {
                Err(CertificateError::UnsupportedPfx)
            } else {
                Err(err)
            }
        }
    }
}

pub fn looks_like_pem_certificate(text: &str) -> bool {
    text.contains("-----BEGIN CERTIFICATE-----")
}

fn looks_like_pfx(input: &[u8]) -> bool {
    let text = String::from_utf8_lossy(input);
    let upper = text.to_ascii_uppercase();
    upper.contains("BEGIN PKCS") || upper.contains("BEGIN PFX") || upper.contains("BEGIN P12")
}

fn parse_pem_or_der(input: &[u8]) -> Result<String, CertificateError> {
    if let Some(pem_slice) = find_pem(input) {
        let pem = parse_x509_pem(pem_slice)
            .map_err(|_| CertificateError::InvalidCertificate)?
            .1;
        let cert = pem
            .parse_x509()
            .map_err(|_| CertificateError::InvalidCertificate)?;
        return Ok(format_cert(&cert, &pem.contents));
    }
    let cert = X509Certificate::from_der(input)
        .map_err(|_| CertificateError::InvalidCertificate)?
        .1;
    Ok(format_cert(&cert, input))
}

fn find_pem(input: &[u8]) -> Option<&[u8]> {
    let text = std::str::from_utf8(input).ok()?;
    let start = text.find("-----BEGIN ")?;
    Some(text[start..].as_bytes())
}

fn format_cert(cert: &X509Certificate<'_>, der: &[u8]) -> String {
    let fingerprint = hex::encode(Sha256::digest(der));
    format!(
        "主题: {}\n颁发者: {}\n生效: {}\n过期: {}\nSHA256: {}",
        cert.subject(),
        cert.issuer(),
        cert.validity().not_before,
        cert.validity().not_after,
        fingerprint
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEM: &str = "-----BEGIN CERTIFICATE-----
MIIDJjCCAg6gAwIBAgIUXujLRFcMc4hn+bOyXek+9vVI6pUwDQYJKoZIhvcNAQEL
BQAwKDEUMBIGA1UEAwwLRGV2VG95c1Rlc3QxEDAOBgNVBAoMB0RldlRveXMwHhcN
MjYwODI2MTYxMDEyWhcNMzYwODIzMTYxMDEyWjAoMRQwEgYDVQQDDAtEZXZUb3lz
VGVzdDEQMA4GA1UECgwHRGV2VG95czCCASIwDQYJKoZIhvcNAQEBBQADggEPADCC
AQoCggEBAJvPtEQAlwkmFJpA9OmLOTz+LQhBOIZv4jlIkWA8N+lcWX//F5/COLqE
dWwklEtjkFgcxWW0SV/A/oqYIi+bH9lszmwrkMgDhk35dWwSTZavdbeVybLo3deI
FjYUpjOq0XaNzrFGww43hCvF+kkTIxK988Fb2+x492GlntrKdTIk9GBHRJMY44IQ
tZUP4JxGtWrmxQFmUNid5sR4myk8javGy1aRYXtXzfIdGhobdLEWK3wAYal0ZvcU
xBSYPyfUUkkttdRtkwy+jpHstikNuorIymNEfGRiPovvd+Ma/LEtZGoJOStwFtmY
FWWa4Khq1ST3XbfaaMbBbQc82QXtNC0CAwEAAaNIMEYwHQYDVR0OBBYEFO1s4mCK
lyi5iJHudz3jNRxcgthXMA8GA1UdEwEB/wQFMAMBAf8wFAYDVR0RBA0wC4IJbG9j
YWxob3N0MA0GCSqGSIb3DQEBCwUAA4IBAQB9ZuluOV6Vj4YEZR9hZDrqUtn9/rVv
ifZ8SVU67jtqaaCxtAE1Lm8pM1GAydGYCdQSj5zQxtBxKqV/5Bh7qZJVmxuJIk7H
5XiLJXTAL5kdJKCOzMqnLM2GlgE+CvQnOc4eSLYLMQa2PlsjZSGSpLc/KZlaWM46
1JwmGSGNAzWqLFqPEmCle0hwp76lALxJUKbsQmbxUCVDNGfNZmQFMh08svlvnY8b
11BoKCb6hYO+4s6X1IzaK5Tqh0GWTNo4lTzzuPEylHPlHyZfAe6JPPj38neXlT4s
rTyFj9ZjK1rUjyJ8CPyXGhh+GDU7qQBKu7yJK6jMLM0sYVa65Vs2cT7j
-----END CERTIFICATE-----";

    #[test]
    fn pem_shows_subject_and_issuer() {
        let got = decode_certificate(PEM.as_bytes(), None).unwrap();
        assert!(got.contains("DevToysTest"), "subject must appear: {got}");
        assert!(got.contains("DevToys"), "issuer/org must appear: {got}");
    }

    #[test]
    fn invalid_is_err_without_input() {
        let err = decode_certificate(b"not-a-cert", None).unwrap_err();
        assert_eq!(err, CertificateError::InvalidCertificate);
        let message = err.to_string();
        assert!(
            !message.contains("not-a-cert"),
            "error must not include user input"
        );
    }

    #[test]
    fn error_does_not_contain_password() {
        let err = decode_certificate(b"not-a-cert", Some("s3cret-pass")).unwrap_err();
        let message = err.to_string();
        assert!(
            !message.contains("s3cret-pass"),
            "error must not include password"
        );
        let debug = format!("{err:?}");
        assert!(
            !debug.contains("s3cret-pass"),
            "debug must not include password"
        );
    }
}
