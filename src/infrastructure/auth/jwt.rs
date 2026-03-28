use super::claims::BeeClaims;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use std::env;

/// JWT 服务
pub struct JwtService {
    secret: String,
    expiry_secs: u64,
}

impl Default for JwtService {
    fn default() -> Self {
        Self::new()
    }
}

impl JwtService {
    pub fn new() -> Self {
        let secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| "default-secret-change-in-production".to_string());
        let expiry_secs = env::var("JWT_EXPIRY_SECS")
            .unwrap_or_else(|_| "86400".to_string())
            .parse()
            .unwrap_or(86400);

        Self { secret, expiry_secs }
    }

    /// 生成 JWT token
    pub fn generate_token(&self, claims: &BeeClaims) -> Result<String, jsonwebtoken::errors::Error> {
        encode(&Header::default(), claims, &EncodingKey::from_secret(self.secret.as_bytes()))
    }

    /// 验证并解析 token
    pub fn validate_token(&self, token: &str) -> Result<BeeClaims, JwtError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        match decode::<BeeClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        ) {
            Ok(token_data) => Ok(token_data.claims),
            Err(e) => match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                    Err(JwtError::TokenExpired)
                }
                _ => Err(JwtError::InvalidToken(e)),
            },
        }
    }

    /// 刷新 token
    pub fn refresh_token(&self, old_claims: &BeeClaims) -> Result<String, jsonwebtoken::errors::Error> {
        let mut new_claims = old_claims.clone();
        let now = chrono::Utc::now();
        new_claims.exp = (now + chrono::Duration::seconds(self.expiry_secs as i64)).timestamp() as usize;
        new_claims.iat = now.timestamp() as usize;

        encode(&Header::default(), &new_claims, &EncodingKey::from_secret(self.secret.as_bytes()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    #[error("Invalid token: {0}")]
    InvalidToken(#[from] jsonwebtoken::errors::Error),
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid claims")]
    InvalidClaims,
}
