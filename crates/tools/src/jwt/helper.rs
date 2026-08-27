use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
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
    Ps256,
    Ps384,
    Ps512,
}

impl JwtAlgorithm {
    pub const ALL: [JwtAlgorithm; 11] = [
        Self::Hs256,
        Self::Hs384,
        Self::Hs512,
        Self::Rs256,
        Self::Rs384,
        Self::Rs512,
        Self::Es256,
        Self::Es384,
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
    pub validate_lifetime: bool,
    pub actor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JwtDecoded {
    pub header: String,
    pub payload: String,
    pub signature: String,
}

pub fn encode_jwt(payload: &str, opts: &JwtEncodeOptions) -> Result<String, JwtError> {
    let mut claims: Value =
        serde_json::from_str(payload).map_err(|_| JwtError::InvalidPayload)?;
    apply_encode_claims(&mut claims, opts)?;
    let header = Header::new(opts.algorithm.to_jwt());
    let key = encoding_key(opts)?;
    encode(&header, &claims, &key).map_err(map_encode_error)
}

pub fn decode_jwt(token: &str, opts: &JwtDecodeOptions) -> Result<JwtDecoded, JwtError> {
    let token = token.trim();
    let mut parts = token.split('.');
    let header_part = parts.next().filter(|s| !s.is_empty()).ok_or(JwtError::InvalidToken)?;
    let payload_part = parts.next().filter(|s| !s.is_empty()).ok_or(JwtError::InvalidToken)?;
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

    if needs_verification(opts) {
        verify_jwt(token, opts, &decoded)?;
    }
    Ok(decoded)
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

fn needs_verification(opts: &JwtDecodeOptions) -> bool {
    nonempty(opts.secret.as_deref())
        || nonempty(opts.issuer.as_deref())
        || nonempty(opts.audience.as_deref())
        || nonempty(opts.actor.as_deref())
        || opts.validate_lifetime
}

fn nonempty(value: Option<&str>) -> bool {
    value.map(|s| !s.is_empty()).unwrap_or(false)
}

fn verify_jwt(token: &str, opts: &JwtDecodeOptions, decoded: &JwtDecoded) -> Result<(), JwtError> {
    let header = decode_header(token).map_err(|_| JwtError::InvalidToken)?;
    let alg = algorithm_from_jwt(header.alg)?;

    let mut validation = Validation::new(header.alg);
    validation.required_spec_claims.clear();
    validation.validate_exp = opts.validate_lifetime;
    validation.validate_nbf = opts.validate_lifetime;
    validation.validate_aud = nonempty(opts.audience.as_deref());
    if opts.validate_lifetime {
        validation.required_spec_claims.insert("exp".into());
    }
    if let Some(iss) = opts.issuer.as_deref().filter(|s| !s.is_empty()) {
        validation.set_issuer(&[iss]);
        validation.required_spec_claims.insert("iss".into());
    }
    if let Some(aud) = opts.audience.as_deref().filter(|s| !s.is_empty()) {
        validation.set_audience(&[aud]);
        validation.required_spec_claims.insert("aud".into());
    }

    let verify_sig = nonempty(opts.secret.as_deref());
    if !verify_sig {
        validation.insecure_disable_signature_validation();
    }
    let key = decoding_key(alg, opts, verify_sig)?;
    decode::<Map<String, Value>>(token, &key, &validation).map_err(|_| JwtError::VerificationFailed)?;

    if let Some(actor) = opts.actor.as_deref().filter(|s| !s.is_empty()) {
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
    let secret = opts.secret.as_deref().unwrap_or("");
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
        let token = encode_jwt(r#"{"sub":"1"}"#, &hs256("secret")).unwrap();
        let err = decode_jwt(
            &token,
            &JwtDecodeOptions {
                secret: Some("wrong".into()),
                ..JwtDecodeOptions::default()
            },
        )
        .unwrap_err();
        assert_eq!(err, JwtError::VerificationFailed);
        let message = err.to_string();
        assert!(!message.contains("wrong"));
        assert!(!message.contains(&token));
    }

    #[test]
    fn verification_succeeds_with_matching_secret() {
        let token = encode_jwt(r#"{"sub":"1"}"#, &hs256("secret")).unwrap();
        let decoded = decode_jwt(
            &token,
            &JwtDecodeOptions {
                secret: Some("secret".into()),
                ..JwtDecodeOptions::default()
            },
        )
        .unwrap();
        assert!(decoded.header.contains("HS256"));
    }

    #[test]
    fn invalid_payload_is_err() {
        let err = encode_jwt("{", &hs256("secret")).unwrap_err();
        assert_eq!(err, JwtError::InvalidPayload);
        assert!(!err.to_string().contains('{'));
    }
}
