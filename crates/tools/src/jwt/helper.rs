use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use p521::ecdsa::signature::{Signer, Verifier};
use p521::ecdsa::{Signature, SigningKey, VerifyingKey};
use p521::elliptic_curve::sec1::ToEncodedPoint;
use p521::pkcs8::{DecodePrivateKey, DecodePublicKey};
use serde_json::{json, Map, Value};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JwtAlgorithm {
    #[default]
    Hs256,
    Hs384,
    Hs512,
    Rs256,
    Rs384,
    Rs512,
    Es256,
    Es384,
    Es512,
    Ps256,
    Ps384,
    Ps512,
}

impl JwtAlgorithm {
    pub const ALL: [JwtAlgorithm; 12] = [
        Self::Hs256,
        Self::Hs384,
        Self::Hs512,
        Self::Rs256,
        Self::Rs384,
        Self::Rs512,
        Self::Es256,
        Self::Es384,
        Self::Es512,
        Self::Ps256,
        Self::Ps384,
        Self::Ps512,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hs256 => "HS256",
            Self::Hs384 => "HS384",
            Self::Hs512 => "HS512",
            Self::Rs256 => "RS256",
            Self::Rs384 => "RS384",
            Self::Rs512 => "RS512",
            Self::Es256 => "ES256",
            Self::Es384 => "ES384",
            Self::Es512 => "ES512",
            Self::Ps256 => "PS256",
            Self::Ps384 => "PS384",
            Self::Ps512 => "PS512",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|alg| alg.as_str() == value)
    }

    fn to_jwt(self) -> Algorithm {
        match self {
            Self::Hs256 => Algorithm::HS256,
            Self::Hs384 => Algorithm::HS384,
            Self::Hs512 => Algorithm::HS512,
            Self::Rs256 => Algorithm::RS256,
            Self::Rs384 => Algorithm::RS384,
            Self::Rs512 => Algorithm::RS512,
            Self::Es256 => Algorithm::ES256,
            Self::Es384 => Algorithm::ES384,
            Self::Es512 => unreachable!("ES512 is handled outside jsonwebtoken"),
            Self::Ps256 => Algorithm::PS256,
            Self::Ps384 => Algorithm::PS384,
            Self::Ps512 => Algorithm::PS512,
        }
    }

    fn is_hmac(self) -> bool {
        matches!(self, Self::Hs256 | Self::Hs384 | Self::Hs512)
    }

    fn is_rsa(self) -> bool {
        matches!(
            self,
            Self::Rs256 | Self::Rs384 | Self::Rs512 | Self::Ps256 | Self::Ps384 | Self::Ps512
        )
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JwtError {
    #[error("非法 Token")]
    InvalidToken,
    #[error("校验失败")]
    VerificationFailed,
    #[error("非法 Payload")]
    InvalidPayload,
    #[error("非法密钥")]
    InvalidKey,
    #[error("不支持的算法")]
    UnsupportedAlgorithm,
}

#[derive(Clone, Debug, Default)]
pub struct JwtEncodeOptions {
    pub algorithm: JwtAlgorithm,
    pub secret: String,
    pub secret_is_base64: bool,
    pub issuer: Option<String>,
    pub audience: Option<String>,
    pub add_default_time_claims: bool,
    pub expiration_secs: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct JwtDecodeOptions {
    pub secret: Option<String>,
    pub secret_is_base64: bool,
    pub issuer: Option<String>,
    pub audience: Option<String>,
    pub actor: Option<String>,
    pub validate_token: bool,
    pub validate_signature: bool,
    pub validate_issuer: bool,
    pub validate_audience: bool,
    pub validate_lifetime: bool,
    pub validate_actor: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JwtDecoded {
    pub header: String,
    pub payload: String,
    pub signature: String,
}

pub fn encode_jwt(payload: &str, opts: &JwtEncodeOptions) -> Result<String, JwtError> {
    let mut claims: Value = serde_json::from_str(payload).map_err(|_| JwtError::InvalidPayload)?;
    apply_encode_claims(&mut claims, opts)?;
    if opts.algorithm == JwtAlgorithm::Es512 {
        return encode_es512(&claims, opts);
    }
    let header = Header::new(opts.algorithm.to_jwt());
    let key = encoding_key(opts)?;
    encode(&header, &claims, &key).map_err(map_encode_error)
}

pub fn decode_jwt(token: &str, opts: &JwtDecodeOptions) -> Result<JwtDecoded, JwtError> {
    let token = token.trim();
    let mut parts = token.split('.');
    let header_part = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(JwtError::InvalidToken)?;
    let payload_part = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(JwtError::InvalidToken)?;
    let signature_part = parts.next().ok_or(JwtError::InvalidToken)?;
    if parts.next().is_some() {
        return Err(JwtError::InvalidToken);
    }

    let header = pretty_json(&b64url_decode(header_part)?)?;
    let payload = pretty_json(&b64url_decode(payload_part)?)?;
    let decoded = JwtDecoded {
        header,
        payload,
        signature: signature_part.to_string(),
    };

    if opts.validate_token {
        verify_jwt(token, opts, &decoded)?;
    }
    Ok(decoded)
}

pub fn claims_table(payload_json: &str) -> Vec<(String, String)> {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(payload_json) else {
        return Vec::new();
    };
    map.into_iter()
        .map(|(key, value)| (key, claim_display(&value)))
        .collect()
}

fn claim_display(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "null".into())
        }
    }
}

fn apply_encode_claims(claims: &mut Value, opts: &JwtEncodeOptions) -> Result<(), JwtError> {
    let wants_claims = nonempty(opts.issuer.as_deref())
        || nonempty(opts.audience.as_deref())
        || opts.add_default_time_claims
        || opts.expiration_secs.is_some();
    if !wants_claims {
        return Ok(());
    }
    let obj = claims.as_object_mut().ok_or(JwtError::InvalidPayload)?;
    if let Some(iss) = opts.issuer.as_deref().filter(|s| !s.is_empty()) {
        obj.insert("iss".into(), Value::String(iss.to_string()));
    }
    if let Some(aud) = opts.audience.as_deref().filter(|s| !s.is_empty()) {
        obj.insert("aud".into(), Value::String(aud.to_string()));
    }
    if opts.add_default_time_claims {
        let now = jsonwebtoken::get_current_timestamp();
        obj.entry("iat").or_insert(json!(now));
        obj.entry("nbf").or_insert(json!(now));
        let exp = now.saturating_add(opts.expiration_secs.unwrap_or(3600));
        obj.entry("exp").or_insert(json!(exp));
    } else if let Some(secs) = opts.expiration_secs {
        let now = jsonwebtoken::get_current_timestamp();
        obj.insert("exp".into(), json!(now.saturating_add(secs)));
    }
    Ok(())
}

fn nonempty(value: Option<&str>) -> bool {
    value.map(|s| !s.is_empty()).unwrap_or(false)
}

fn parse_list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or("")
        .split(|c| c == ',' || c == '\n' || c == '\r')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn verify_jwt(token: &str, opts: &JwtDecodeOptions, decoded: &JwtDecoded) -> Result<(), JwtError> {
    if header_alg_name(&decoded.header)?.eq_ignore_ascii_case("ES512") {
        return verify_es512(token, opts, decoded);
    }

    let header = decode_header(token).map_err(|_| JwtError::InvalidToken)?;
    let alg = algorithm_from_jwt(header.alg)?;

    let mut validation = Validation::new(header.alg);
    validation.required_spec_claims.clear();
    validation.validate_exp = opts.validate_lifetime;
    validation.validate_nbf = opts.validate_lifetime;
    validation.validate_aud = opts.validate_audience;
    if opts.validate_lifetime {
        validation.required_spec_claims.insert("exp".into());
    }
    if opts.validate_issuer {
        let issuers = parse_list(opts.issuer.as_deref());
        if issuers.is_empty() {
            return Err(JwtError::VerificationFailed);
        }
        validation.set_issuer(&issuers);
        validation.required_spec_claims.insert("iss".into());
    }
    if opts.validate_audience {
        let audiences = parse_list(opts.audience.as_deref());
        if audiences.is_empty() {
            return Err(JwtError::VerificationFailed);
        }
        validation.set_audience(&audiences);
        validation.required_spec_claims.insert("aud".into());
    }

    if !opts.validate_signature {
        validation.insecure_disable_signature_validation();
    }
    let key = decoding_key(alg, opts, opts.validate_signature)?;
    decode::<Map<String, Value>>(token, &key, &validation)
        .map_err(|_| JwtError::VerificationFailed)?;

    if opts.validate_actor {
        let actor = opts
            .actor
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or(JwtError::VerificationFailed)?;
        let payload: Value =
            serde_json::from_str(&decoded.payload).map_err(|_| JwtError::InvalidToken)?;
        if !actor_matches(&payload, actor) {
            return Err(JwtError::VerificationFailed);
        }
    }
    Ok(())
}

fn actor_matches(payload: &Value, expected: &str) -> bool {
    match payload.get("act") {
        Some(Value::String(s)) => s == expected,
        Some(Value::Object(map)) => map.get("sub").and_then(Value::as_str) == Some(expected),
        _ => false,
    }
}

fn algorithm_from_jwt(alg: Algorithm) -> Result<JwtAlgorithm, JwtError> {
    Ok(match alg {
        Algorithm::HS256 => JwtAlgorithm::Hs256,
        Algorithm::HS384 => JwtAlgorithm::Hs384,
        Algorithm::HS512 => JwtAlgorithm::Hs512,
        Algorithm::RS256 => JwtAlgorithm::Rs256,
        Algorithm::RS384 => JwtAlgorithm::Rs384,
        Algorithm::RS512 => JwtAlgorithm::Rs512,
        Algorithm::ES256 => JwtAlgorithm::Es256,
        Algorithm::ES384 => JwtAlgorithm::Es384,
        Algorithm::PS256 => JwtAlgorithm::Ps256,
        Algorithm::PS384 => JwtAlgorithm::Ps384,
        Algorithm::PS512 => JwtAlgorithm::Ps512,
        _ => return Err(JwtError::UnsupportedAlgorithm),
    })
}

fn encoding_key(opts: &JwtEncodeOptions) -> Result<EncodingKey, JwtError> {
    let secret = opts.secret.as_str();
    if opts.algorithm.is_hmac() {
        if opts.secret_is_base64 {
            EncodingKey::from_base64_secret(secret).map_err(|_| JwtError::InvalidKey)
        } else {
            Ok(EncodingKey::from_secret(secret.as_bytes()))
        }
    } else if opts.algorithm.is_rsa() {
        EncodingKey::from_rsa_pem(secret.as_bytes()).map_err(|_| JwtError::InvalidKey)
    } else {
        EncodingKey::from_ec_pem(secret.as_bytes()).map_err(|_| JwtError::InvalidKey)
    }
}

fn decoding_key(
    alg: JwtAlgorithm,
    opts: &JwtDecodeOptions,
    verify_sig: bool,
) -> Result<DecodingKey, JwtError> {
    if !verify_sig {
        return Ok(DecodingKey::from_secret(b""));
    }
    let secret = opts
        .secret
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or(JwtError::InvalidKey)?;
    if alg.is_hmac() {
        if opts.secret_is_base64 {
            DecodingKey::from_base64_secret(secret).map_err(|_| JwtError::InvalidKey)
        } else {
            Ok(DecodingKey::from_secret(secret.as_bytes()))
        }
    } else if alg.is_rsa() {
        DecodingKey::from_rsa_pem(secret.as_bytes()).map_err(|_| JwtError::InvalidKey)
    } else {
        DecodingKey::from_ec_pem(secret.as_bytes()).map_err(|_| JwtError::InvalidKey)
    }
}

fn b64url_decode(input: &str) -> Result<Vec<u8>, JwtError> {
    use data_encoding::{BASE64URL, BASE64URL_NOPAD};

    match BASE64URL_NOPAD.decode(input.as_bytes()) {
        Ok(bytes) => Ok(bytes),
        Err(_) => {
            let mut padded = input.to_string();
            while padded.len() % 4 != 0 {
                padded.push('=');
            }
            BASE64URL
                .decode(padded.as_bytes())
                .map_err(|_| JwtError::InvalidToken)
        }
    }
}

fn pretty_json(bytes: &[u8]) -> Result<String, JwtError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| JwtError::InvalidToken)?;
    serde_json::to_string_pretty(&value).map_err(|_| JwtError::InvalidToken)
}

fn map_encode_error(err: jsonwebtoken::errors::Error) -> JwtError {
    match err.kind() {
        jsonwebtoken::errors::ErrorKind::InvalidKeyFormat
        | jsonwebtoken::errors::ErrorKind::InvalidRsaKey(_)
        | jsonwebtoken::errors::ErrorKind::InvalidEcdsaKey => JwtError::InvalidKey,
        jsonwebtoken::errors::ErrorKind::Json(_) => JwtError::InvalidPayload,
        _ => JwtError::InvalidKey,
    }
}

fn header_alg_name(header_json: &str) -> Result<String, JwtError> {
    let value: Value = serde_json::from_str(header_json).map_err(|_| JwtError::InvalidToken)?;
    value
        .get("alg")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(JwtError::InvalidToken)
}

fn encode_es512(claims: &Value, opts: &JwtEncodeOptions) -> Result<String, JwtError> {
    let signing_key = es512_signing_key(&opts.secret)?;
    let header = json!({"typ": "JWT", "alg": "ES512"});
    let header_b64 = b64url_encode_json(&header)?;
    let payload_b64 = b64url_encode_json(claims)?;
    let signing_input = format!("{header_b64}.{payload_b64}");
    let signature: Signature = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = data_encoding::BASE64URL_NOPAD.encode(signature.to_bytes().as_ref());
    Ok(format!("{signing_input}.{sig_b64}"))
}

fn verify_es512(
    token: &str,
    opts: &JwtDecodeOptions,
    decoded: &JwtDecoded,
) -> Result<(), JwtError> {
    if opts.validate_signature {
        let secret = opts
            .secret
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or(JwtError::InvalidKey)?;
        let verifying_key = es512_verifying_key(secret)?;
        let (signing_input, sig_b64) = token.rsplit_once('.').ok_or(JwtError::InvalidToken)?;
        let sig_bytes = b64url_decode(sig_b64)?;
        let signature =
            Signature::from_slice(&sig_bytes).map_err(|_| JwtError::VerificationFailed)?;
        verifying_key
            .verify(signing_input.as_bytes(), &signature)
            .map_err(|_| JwtError::VerificationFailed)?;
    }

    let payload: Value =
        serde_json::from_str(&decoded.payload).map_err(|_| JwtError::InvalidToken)?;
    verify_decoded_claims(&payload, opts)
}

fn verify_decoded_claims(payload: &Value, opts: &JwtDecodeOptions) -> Result<(), JwtError> {
    const LEEWAY: u64 = 60;
    if opts.validate_lifetime {
        let now = jsonwebtoken::get_current_timestamp();
        let exp = payload
            .get("exp")
            .and_then(json_to_u64)
            .ok_or(JwtError::VerificationFailed)?;
        if now > exp.saturating_add(LEEWAY) {
            return Err(JwtError::VerificationFailed);
        }
        if let Some(nbf) = payload.get("nbf").and_then(json_to_u64) {
            if now.saturating_add(LEEWAY) < nbf {
                return Err(JwtError::VerificationFailed);
            }
        }
    }
    if opts.validate_issuer {
        let issuers = parse_list(opts.issuer.as_deref());
        if issuers.is_empty() {
            return Err(JwtError::VerificationFailed);
        }
        let iss = payload.get("iss").and_then(Value::as_str);
        if !issuers
            .iter()
            .any(|expected| iss == Some(expected.as_str()))
        {
            return Err(JwtError::VerificationFailed);
        }
    }
    if opts.validate_audience {
        let audiences = parse_list(opts.audience.as_deref());
        if audiences.is_empty() {
            return Err(JwtError::VerificationFailed);
        }
        if !audiences
            .iter()
            .any(|expected| audience_matches(payload.get("aud"), expected))
        {
            return Err(JwtError::VerificationFailed);
        }
    }
    if opts.validate_actor {
        let actor = opts
            .actor
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or(JwtError::VerificationFailed)?;
        if !actor_matches(payload, actor) {
            return Err(JwtError::VerificationFailed);
        }
    }
    Ok(())
}

fn audience_matches(aud: Option<&Value>, expected: &str) -> bool {
    match aud {
        Some(Value::String(s)) => s == expected,
        Some(Value::Array(items)) => items.iter().any(|v| v.as_str() == Some(expected)),
        _ => false,
    }
}

fn json_to_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64().or_else(|| n.as_f64().map(|f| f as u64)),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn es512_signing_key(pem: &str) -> Result<SigningKey, JwtError> {
    let pem = pem.trim();
    if pem.is_empty() {
        return Err(JwtError::InvalidKey);
    }
    let secret = if let Ok(secret) = p521::SecretKey::from_pkcs8_pem(pem) {
        secret
    } else {
        p521::SecretKey::from_sec1_pem(pem).map_err(|_| JwtError::InvalidKey)?
    };
    SigningKey::from_bytes(&secret.to_bytes()).map_err(|_| JwtError::InvalidKey)
}

fn es512_verifying_key(pem: &str) -> Result<VerifyingKey, JwtError> {
    let pem = pem.trim();
    if pem.is_empty() {
        return Err(JwtError::InvalidKey);
    }
    if let Ok(public) = p521::PublicKey::from_public_key_pem(pem) {
        return VerifyingKey::from_encoded_point(&public.to_encoded_point(false))
            .map_err(|_| JwtError::InvalidKey);
    }
    Ok(VerifyingKey::from(&es512_signing_key(pem)?))
}

fn b64url_encode_json(value: &Value) -> Result<String, JwtError> {
    let json = serde_json::to_vec(value).map_err(|_| JwtError::InvalidPayload)?;
    Ok(data_encoding::BASE64URL_NOPAD.encode(&json))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hs256(secret: &str) -> JwtEncodeOptions {
        JwtEncodeOptions {
            algorithm: JwtAlgorithm::Hs256,
            secret: secret.to_string(),
            ..JwtEncodeOptions::default()
        }
    }

    // HMAC-SHA256 with secret "secret"; literals are independent of encode_jwt.
    pub(crate) const HS256_SECRET: &str = "secret";
    pub(crate) const HS256_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.XbPfbIHMI6arZ3Y922BhjWgQzWXcXNrz0ogtVhfEd2o";
    const HS256_ISS_AUD_ACT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwiaXNzIjoiZGV2dG95cyIsImF1ZCI6ImFwcCIsImFjdCI6InRlc3RlciIsImV4cCI6OTk5OTk5OTk5OX0.bOKLhzVbGQGd1LBvrbrrtt2X2uVCiI9J8rdSolke9pw";
    const HS256_AUD_ARRAY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwiaXNzIjoiaXNzdWVyLWIiLCJhdWQiOlsieCIsInkiXX0.h4tPuPGNyoRFWv4IQDxlEUjP-Sb-i8XksyVoUviZJGo";
    const HS256_EXPIRED: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwiZXhwIjoxLCJuYmYiOjF9.g6oalJ2val1DFq6zpOBo4w1QvqOfFQmB6lE2kILvo0c";
    const HS256_FUTURE_NBF: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwiZXhwIjo5OTk5OTk5OTk5LCJuYmYiOjk5OTk5OTk5OTl9.2ZiwJHltk1W_3rWrkVMtIAaBJoKZenkpWEk0A_iUDLY";
    const HS256_ACT_OBJECT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIiwiYWN0Ijp7InN1YiI6InRlc3RlciJ9fQ.NXya3nCzuDYVAPu6jtCCOu5uU1_649qq9_iNcGFGKBw";

    fn decode_opts(token_flags: impl FnOnce(&mut JwtDecodeOptions)) -> JwtDecodeOptions {
        let mut opts = JwtDecodeOptions::default();
        token_flags(&mut opts);
        opts
    }

    #[test]
    fn encode_hs256_then_decode_header_alg() {
        let token = encode_jwt(r#"{"sub":"1"}"#, &hs256("secret")).unwrap();
        let decoded = decode_jwt(&token, &JwtDecodeOptions::default()).unwrap();
        assert!(
            decoded.header.contains("HS256"),
            "header should name HS256: {}",
            decoded.header
        );
        assert!(decoded.payload.contains("\"sub\""));
        assert!(!decoded.signature.is_empty());
    }

    #[test]
    fn invalid_token_is_err() {
        let err = decode_jwt("not-a-jwt", &JwtDecodeOptions::default()).unwrap_err();
        assert_eq!(err, JwtError::InvalidToken);
        assert!(!err.to_string().contains("not-a-jwt"));
        assert_eq!(
            decode_jwt("a.b", &JwtDecodeOptions::default()).unwrap_err(),
            JwtError::InvalidToken
        );
    }

    #[test]
    fn verification_fail_is_err() {
        let err = decode_jwt(
            HS256_TOKEN,
            &decode_opts(|opts| {
                opts.secret = Some("wrong".into());
                opts.validate_token = true;
                opts.validate_signature = true;
            }),
        )
        .unwrap_err();
        assert_eq!(err, JwtError::VerificationFailed);
        let message = err.to_string();
        assert!(!message.contains("wrong"));
        assert!(!message.contains(HS256_TOKEN));
        assert!(!message.contains(HS256_SECRET));
    }

    #[test]
    fn verification_succeeds_with_matching_secret() {
        let decoded = decode_jwt(
            HS256_TOKEN,
            &decode_opts(|opts| {
                opts.secret = Some(HS256_SECRET.into());
                opts.validate_token = true;
                opts.validate_signature = true;
            }),
        )
        .unwrap();
        assert!(decoded.header.contains("HS256"));
        assert!(decoded.payload.contains("John Doe"));
        assert_eq!(
            decoded.signature,
            "XbPfbIHMI6arZ3Y922BhjWgQzWXcXNrz0ogtVhfEd2o"
        );
    }

    #[test]
    fn invalid_payload_is_err() {
        let err = encode_jwt("{", &hs256("secret")).unwrap_err();
        assert_eq!(err, JwtError::InvalidPayload);
        assert!(!err.to_string().contains('{'));
    }

    #[test]
    fn hs256_still_encodes_after_es512() {
        let token = encode_jwt(r#"{"sub":"1"}"#, &hs256("secret")).unwrap();
        let decoded = decode_jwt(&token, &JwtDecodeOptions::default()).unwrap();
        assert!(decoded.header.contains("HS256"));
        assert!(!decoded.header.contains("ES512"));
    }

    // Independent ES512 vector: PyJWT 2.13.0 + cryptography 50.0.0 (ECDSA P-521 / SHA-512).
    // Not produced by encode_jwt.
    pub(crate) const ES512_PRIVATE_SEC1: &str = "-----BEGIN EC PRIVATE KEY-----\n\
MIHcAgEBBEIB1zyqNkKL8kFjgS2Ctwp06Hs+2pkZWVNFQ0Mb+9yRU9NKP0joLQR4\n\
RI+56xcGVg4+HCdX1LaWMxbQJrz14VyI8oigBwYFK4EEACOhgYkDgYYABADgvOml\n\
CsKLsJIXTkV4hKeuimsnUkZrdaVBYs/uilR+Uy55GZVKNm0Afv3LQGIlKJjbhjZc\n\
GwONlryq7+S9TXowdgCX3Bqn1+TZxRcIv9nyKwqTI9QIO/G2WqfpmqYiWnTSG5jt\n\
fcU9R//QwPzxBOI9PjzFKHPeV+u3Qn2b5KcoJU1HqQ==\n\
-----END EC PRIVATE KEY-----\n";

    pub(crate) const ES512_PRIVATE_PKCS8: &str = "-----BEGIN PRIVATE KEY-----\n\
MIHuAgEAMBAGByqGSM49AgEGBSuBBAAjBIHWMIHTAgEBBEIB1zyqNkKL8kFjgS2C\n\
twp06Hs+2pkZWVNFQ0Mb+9yRU9NKP0joLQR4RI+56xcGVg4+HCdX1LaWMxbQJrz1\n\
4VyI8oihgYkDgYYABADgvOmlCsKLsJIXTkV4hKeuimsnUkZrdaVBYs/uilR+Uy55\n\
GZVKNm0Afv3LQGIlKJjbhjZcGwONlryq7+S9TXowdgCX3Bqn1+TZxRcIv9nyKwqT\n\
I9QIO/G2WqfpmqYiWnTSG5jtfcU9R//QwPzxBOI9PjzFKHPeV+u3Qn2b5KcoJU1H\n\
qQ==\n\
-----END PRIVATE KEY-----\n";

    pub(crate) const ES512_PUBLIC: &str = "-----BEGIN PUBLIC KEY-----\n\
MIGbMBAGByqGSM49AgEGBSuBBAAjA4GGAAQA4LzppQrCi7CSF05FeISnroprJ1JG\n\
a3WlQWLP7opUflMueRmVSjZtAH79y0BiJSiY24Y2XBsDjZa8qu/kvU16MHYAl9wa\n\
p9fk2cUXCL/Z8isKkyPUCDvxtlqn6ZqmIlp00huY7X3FPUf/0MD88QTiPT48xShz\n\
3lfrt0J9m+SnKCVNR6k=\n\
-----END PUBLIC KEY-----\n";

    pub(crate) const ES512_INDEPENDENT_TOKEN: &str = "eyJhbGciOiJFUzUxMiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJlczUxMi12ZWN0b3IiLCJpc3MiOiJkZXZ0b3lzLXRlc3QifQ.AQuuUe2s204-og6zmsZLccPim77kgqtvjC_-KdiUnRqs6trj-_h9ikSiZPp8LEMu3-1kgKphRbtijF9oyrXeBoAyAItSlJ6R6xb0_fgL0Je5KGLQw8czla65i3BCVe51D1ZxelGML8Jp4I2bMDwXYJLGNeQTyOIZT2QB2O4iFp8byv0x";

    const ES512_WRONG_PRIVATE: &str = "-----BEGIN EC PRIVATE KEY-----\n\
MIHcAgEBBEIAMaVk1vofuPZfM/rfc3p+tcF9W91HbHOob+J4n9elZPja/nfRQH3w\n\
J3bbAVt9tdR8zuznoAcIvu58rUiB/m93/XGgBwYFK4EEACOhgYkDgYYABAFKWOol\n\
/2aaI7yP7pXRL6Hmi8lF1ZWaEwVw4keSGXsvlFooG4RCV2nX5AIuzcmNXnIypz8n\n\
puelihnYYjS25IL83wB1vPTC8d23SOx0ae0obx99/a0Oz5bDK3ILYcXayoIgH8C8\n\
PKFdih9FtlX4f03LTC2uth6cTFg/OsEhqY+lBWXAuw==\n\
-----END EC PRIVATE KEY-----\n";

    const P256_PRIVATE: &str = "-----BEGIN EC PRIVATE KEY-----\n\
MHcCAQEEIAvuNY7JH+YHCCHa003kg/JdA6sHodmAHjA2kcHy/i5FoAoGCCqGSM49\n\
AwEHoUQDQgAEUVHjOT2M1LBwXCItEqOJd6ZvSZmX2to3xmw8eK/osQeWQg0w84tr\n\
eFrXe+BKLz6ygsKsT4kQ+7541zGenyNdvw==\n\
-----END EC PRIVATE KEY-----\n";

    fn es512(secret: &str) -> JwtEncodeOptions {
        JwtEncodeOptions {
            algorithm: JwtAlgorithm::Es512,
            secret: secret.to_string(),
            ..JwtEncodeOptions::default()
        }
    }

    fn decode_with_secret(token: &str, secret: &str) -> Result<JwtDecoded, JwtError> {
        decode_jwt(
            token,
            &decode_opts(|opts| {
                opts.secret = Some(secret.to_string());
                opts.validate_token = true;
                opts.validate_signature = true;
            }),
        )
    }

    #[test]
    fn parse_es512_and_all_contains_it() {
        assert_eq!(JwtAlgorithm::parse("ES512"), Some(JwtAlgorithm::Es512));
        assert_eq!(JwtAlgorithm::Es512.as_str(), "ES512");
        assert_eq!(JwtAlgorithm::ALL.len(), 12);
        assert!(JwtAlgorithm::ALL.contains(&JwtAlgorithm::Es512));
    }

    #[test]
    fn encode_es512_header_alg() {
        let token = encode_jwt(r#"{"sub":"1"}"#, &es512(ES512_PRIVATE_SEC1)).unwrap();
        let decoded = decode_jwt(&token, &JwtDecodeOptions::default()).unwrap();
        assert!(
            decoded.header.contains("ES512"),
            "header should name ES512: {}",
            decoded.header
        );
        assert!(!decoded.header.contains("ES256"));
        assert!(!decoded.header.contains("ES384"));
        assert!(decoded.payload.contains("\"sub\""));
        assert!(!decoded.signature.is_empty());
        decode_with_secret(&token, ES512_PUBLIC).unwrap();

        let pkcs8_token = encode_jwt(r#"{"sub":"1"}"#, &es512(ES512_PRIVATE_PKCS8)).unwrap();
        let pkcs8_decoded = decode_jwt(&pkcs8_token, &JwtDecodeOptions::default()).unwrap();
        assert!(pkcs8_decoded.header.contains("ES512"));
        decode_with_secret(&pkcs8_token, ES512_PUBLIC).unwrap();
    }

    #[test]
    fn independent_es512_vector_verifies() {
        let decoded = decode_jwt(ES512_INDEPENDENT_TOKEN, &JwtDecodeOptions::default()).unwrap();
        assert!(decoded.header.contains("ES512"));
        assert!(decoded.payload.contains("es512-vector"));
        assert!(!decoded.signature.is_empty());

        let with_public = decode_with_secret(ES512_INDEPENDENT_TOKEN, ES512_PUBLIC).unwrap();
        assert!(with_public.header.contains("ES512"));
        assert!(with_public.payload.contains("es512-vector"));

        let with_private = decode_with_secret(ES512_INDEPENDENT_TOKEN, ES512_PRIVATE_SEC1).unwrap();
        assert!(with_private.payload.contains("devtoys-test"));

        let with_pkcs8 = decode_with_secret(ES512_INDEPENDENT_TOKEN, ES512_PRIVATE_PKCS8).unwrap();
        assert!(with_pkcs8.header.contains("ES512"));
    }

    #[test]
    fn es512_wrong_secret_is_verification_failed() {
        let err = decode_with_secret(ES512_INDEPENDENT_TOKEN, ES512_WRONG_PRIVATE).unwrap_err();
        assert_eq!(err, JwtError::VerificationFailed);
        let message = err.to_string();
        assert!(!message.contains(ES512_WRONG_PRIVATE));
        assert!(!message.contains("BEGIN"));
        assert!(!message.contains(ES512_INDEPENDENT_TOKEN));
        assert!(!message.contains("MaVk1vofuPZf"));
    }

    #[test]
    fn es512_p256_key_is_invalid_key() {
        let encode_err = encode_jwt(r#"{"sub":"1"}"#, &es512(P256_PRIVATE)).unwrap_err();
        assert_eq!(encode_err, JwtError::InvalidKey);
        assert!(!encode_err.to_string().contains(P256_PRIVATE));
        assert!(!encode_err.to_string().contains("BEGIN"));

        let decode_err = decode_with_secret(ES512_INDEPENDENT_TOKEN, P256_PRIVATE).unwrap_err();
        assert!(
            matches!(
                decode_err,
                JwtError::InvalidKey | JwtError::VerificationFailed
            ),
            "P-256 key must not verify ES512: {decode_err:?}"
        );
        assert!(!decode_err.to_string().contains(P256_PRIVATE));
        let decoded = decode_jwt(ES512_INDEPENDENT_TOKEN, &JwtDecodeOptions::default()).unwrap();
        assert!(
            decoded.header.contains("ES512"),
            "must not downgrade to ES256: {}",
            decoded.header
        );
    }

    #[test]
    fn es512_tampered_token_is_rejected() {
        let mut parts: Vec<&str> = ES512_INDEPENDENT_TOKEN.split('.').collect();
        assert_eq!(parts.len(), 3);
        let mut sig = parts[2].to_string();
        let last = sig.pop().unwrap();
        sig.push(if last == 'A' { 'B' } else { 'A' });
        parts[2] = &sig;
        let tampered_sig = parts.join(".");
        let err = decode_with_secret(&tampered_sig, ES512_PUBLIC).unwrap_err();
        assert!(
            matches!(err, JwtError::VerificationFailed | JwtError::InvalidToken),
            "{err:?}"
        );
        assert!(!err.to_string().contains(&tampered_sig));
        assert!(!err.to_string().contains(ES512_PUBLIC));

        let mut payload = parts[1].to_string();
        payload.replace_range(0..1, if payload.starts_with('e') { "f" } else { "e" });
        let tampered_payload = format!(
            "{}.{}.{}",
            parts[0],
            payload,
            ES512_INDEPENDENT_TOKEN.split('.').nth(2).unwrap()
        );
        let err = decode_with_secret(&tampered_payload, ES512_PUBLIC).unwrap_err();
        assert!(
            matches!(err, JwtError::VerificationFailed | JwtError::InvalidToken),
            "{err:?}"
        );
    }

    #[test]
    fn es512_invalid_pem_is_invalid_key() {
        let pem = "-----BEGIN EC PRIVATE KEY-----\nnot-a-real-key\n-----END EC PRIVATE KEY-----";
        let err = encode_jwt(r#"{"sub":"1"}"#, &es512(pem)).unwrap_err();
        assert_eq!(err, JwtError::InvalidKey);
        let message = err.to_string();
        assert!(!message.contains(pem));
        assert!(!message.contains("not-a-real-key"));
        assert!(!message.contains("BEGIN"));

        let err = decode_with_secret(ES512_INDEPENDENT_TOKEN, pem).unwrap_err();
        assert_eq!(err, JwtError::InvalidKey);
        assert!(!err.to_string().contains(pem));
        assert!(!err.to_string().contains("not-a-real-key"));
    }

    fn pem_crlf(pem: &str) -> String {
        pem.replace('\n', "\r\n")
    }

    #[test]
    fn es512_multiline_pem_lf_and_crlf_sign_and_verify() {
        assert!(ES512_PRIVATE_SEC1.contains('\n'));
        assert!(!ES512_PRIVATE_SEC1.contains('\r'));
        assert!(ES512_PUBLIC.contains('\n'));

        let lf_token = encode_jwt(r#"{"sub":"1"}"#, &es512(ES512_PRIVATE_SEC1)).unwrap();
        decode_with_secret(&lf_token, ES512_PUBLIC).unwrap();
        decode_with_secret(&lf_token, ES512_PRIVATE_SEC1).unwrap();
        decode_with_secret(&lf_token, ES512_PRIVATE_PKCS8).unwrap();

        let crlf_sec1 = pem_crlf(ES512_PRIVATE_SEC1);
        let crlf_pkcs8 = pem_crlf(ES512_PRIVATE_PKCS8);
        let crlf_public = pem_crlf(ES512_PUBLIC);
        assert!(crlf_sec1.contains("\r\n"));
        assert!(crlf_public.contains("\r\n"));
        assert!(
            crlf_sec1.contains("-----BEGIN EC PRIVATE KEY-----"),
            "CRLF conversion must keep PEM fences"
        );

        let crlf_token = encode_jwt(r#"{"sub":"1"}"#, &es512(&crlf_sec1)).unwrap();
        decode_with_secret(&crlf_token, &crlf_public).unwrap();
        decode_with_secret(ES512_INDEPENDENT_TOKEN, &crlf_public).unwrap();

        let pkcs8_token = encode_jwt(r#"{"sub":"1"}"#, &es512(&crlf_pkcs8)).unwrap();
        decode_with_secret(&pkcs8_token, &crlf_public).unwrap();

        let decoded = decode_jwt(&crlf_token, &JwtDecodeOptions::default()).unwrap();
        assert!(decoded.header.contains("ES512"));
        assert!(decoded.payload.contains("\"sub\""));
    }

    #[test]
    fn master_off_decodes_despite_wrong_secret() {
        let decoded = decode_jwt(
            HS256_TOKEN,
            &decode_opts(|opts| {
                opts.secret = Some("wrong".into());
                opts.validate_token = false;
                opts.validate_signature = true;
                opts.validate_issuer = true;
                opts.validate_audience = true;
                opts.validate_lifetime = true;
                opts.validate_actor = true;
                opts.issuer = Some("nope".into());
                opts.audience = Some("nope".into());
                opts.actor = Some("nope".into());
            }),
        )
        .unwrap();
        assert!(decoded.header.contains("HS256"));
        assert!(decoded.payload.contains("John Doe"));
        assert_eq!(
            decoded.signature,
            "XbPfbIHMI6arZ3Y922BhjWgQzWXcXNrz0ogtVhfEd2o"
        );
    }

    #[test]
    fn master_on_bad_signature_fails() {
        let err = decode_jwt(
            HS256_TOKEN,
            &decode_opts(|opts| {
                opts.secret = Some("wrong".into());
                opts.validate_token = true;
                opts.validate_signature = true;
            }),
        )
        .unwrap_err();
        assert_eq!(err, JwtError::VerificationFailed);
    }

    #[test]
    fn master_on_without_signature_flag_ignores_wrong_secret() {
        let decoded = decode_jwt(
            HS256_TOKEN,
            &decode_opts(|opts| {
                opts.secret = Some("wrong".into());
                opts.validate_token = true;
                opts.validate_signature = false;
            }),
        )
        .unwrap();
        assert!(decoded.payload.contains("1234567890"));
    }

    #[test]
    fn issuer_toggle_matches_comma_and_newline_lists() {
        let decoded = decode_jwt(
            HS256_ISS_AUD_ACT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_issuer = true;
                opts.issuer = Some("other, devtoys\nextra".into());
            }),
        )
        .unwrap();
        assert!(decoded.payload.contains("devtoys"));

        let err = decode_jwt(
            HS256_ISS_AUD_ACT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_issuer = true;
                opts.issuer = Some("other".into());
            }),
        )
        .unwrap_err();
        assert_eq!(err, JwtError::VerificationFailed);

        decode_jwt(
            HS256_ISS_AUD_ACT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_issuer = false;
                opts.issuer = Some("other".into());
            }),
        )
        .unwrap();

        let newline_ok = decode_jwt(
            HS256_AUD_ARRAY,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_issuer = true;
                opts.issuer = Some("issuer-a\r\nissuer-b".into());
            }),
        )
        .unwrap();
        assert!(newline_ok.payload.contains("issuer-b"));
    }

    #[test]
    fn audience_toggle_matches_string_array_and_lists() {
        decode_jwt(
            HS256_ISS_AUD_ACT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_audience = true;
                opts.audience = Some("web,app".into());
            }),
        )
        .unwrap();

        assert_eq!(
            decode_jwt(
                HS256_ISS_AUD_ACT,
                &decode_opts(|opts| {
                    opts.validate_token = true;
                    opts.validate_audience = true;
                    opts.audience = Some("web".into());
                }),
            )
            .unwrap_err(),
            JwtError::VerificationFailed
        );

        decode_jwt(
            HS256_AUD_ARRAY,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_audience = true;
                opts.audience = Some("z\ny".into());
            }),
        )
        .unwrap();

        decode_jwt(
            HS256_ISS_AUD_ACT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_audience = false;
                opts.audience = Some("web".into());
            }),
        )
        .unwrap();
    }

    #[test]
    fn lifetime_toggle_exp_and_nbf() {
        assert_eq!(
            decode_jwt(
                HS256_TOKEN,
                &decode_opts(|opts| {
                    opts.validate_token = true;
                    opts.validate_lifetime = true;
                }),
            )
            .unwrap_err(),
            JwtError::VerificationFailed
        );
        assert_eq!(
            decode_jwt(
                HS256_EXPIRED,
                &decode_opts(|opts| {
                    opts.validate_token = true;
                    opts.validate_lifetime = true;
                }),
            )
            .unwrap_err(),
            JwtError::VerificationFailed
        );
        assert_eq!(
            decode_jwt(
                HS256_FUTURE_NBF,
                &decode_opts(|opts| {
                    opts.validate_token = true;
                    opts.validate_lifetime = true;
                }),
            )
            .unwrap_err(),
            JwtError::VerificationFailed
        );
        decode_jwt(
            HS256_ISS_AUD_ACT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_lifetime = true;
            }),
        )
        .unwrap();
        decode_jwt(
            HS256_TOKEN,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_lifetime = false;
            }),
        )
        .unwrap();
    }

    #[test]
    fn actor_toggle_string_and_object() {
        decode_jwt(
            HS256_ISS_AUD_ACT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_actor = true;
                opts.actor = Some("tester".into());
            }),
        )
        .unwrap();
        decode_jwt(
            HS256_ACT_OBJECT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_actor = true;
                opts.actor = Some("tester".into());
            }),
        )
        .unwrap();
        assert_eq!(
            decode_jwt(
                HS256_ISS_AUD_ACT,
                &decode_opts(|opts| {
                    opts.validate_token = true;
                    opts.validate_actor = true;
                    opts.actor = Some("other".into());
                }),
            )
            .unwrap_err(),
            JwtError::VerificationFailed
        );
        decode_jwt(
            HS256_ISS_AUD_ACT,
            &decode_opts(|opts| {
                opts.validate_token = true;
                opts.validate_actor = false;
                opts.actor = Some("other".into());
            }),
        )
        .unwrap();
    }

    #[test]
    fn claims_table_rows_from_object_keys() {
        let decoded = decode_jwt(HS256_TOKEN, &JwtDecodeOptions::default()).unwrap();
        assert_eq!(
            claims_table(&decoded.payload),
            vec![
                ("sub".into(), "1234567890".into()),
                ("name".into(), "John Doe".into()),
                ("iat".into(), "1516239022".into()),
            ]
        );
        assert_eq!(
            claims_table(r#"{"sub":"1","n":2,"ok":true,"obj":{"a":1},"arr":[1,2]}"#),
            vec![
                ("sub".into(), "1".into()),
                ("n".into(), "2".into()),
                ("ok".into(), "true".into()),
                ("obj".into(), r#"{"a":1}"#.into()),
                ("arr".into(), "[1,2]".into()),
            ]
        );
        assert!(claims_table("[]").is_empty());
        assert!(claims_table("1").is_empty());
        assert!(claims_table("\"x\"").is_empty());
        assert!(claims_table("not-json").is_empty());
    }
}

#[cfg(test)]
pub(crate) use tests::{
    ES512_INDEPENDENT_TOKEN, ES512_PRIVATE_PKCS8, ES512_PRIVATE_SEC1, ES512_PUBLIC, HS256_SECRET,
    HS256_TOKEN,
};
