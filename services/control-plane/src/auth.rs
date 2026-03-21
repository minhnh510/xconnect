use std::{collections::HashSet, sync::Arc};

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::http::HeaderMap;
use chrono::Utc;
use dashmap::DashSet;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::ApiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub scope: String,
}

#[derive(Clone)]
pub struct TokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    revoked_refresh_tokens: Arc<DashSet<String>>,
}

impl TokenService {
    pub fn new(secret: &[u8]) -> anyhow::Result<Self> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.required_spec_claims =
            HashSet::from(["exp".to_string(), "iat".to_string(), "sub".to_string()]);

        Ok(Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
            validation,
            revoked_refresh_tokens: Arc::new(DashSet::default()),
        })
    }

    pub fn issue_access_token(&self, user_id: Uuid, ttl_seconds: i64) -> Result<String, ApiError> {
        self.issue_token(user_id, ttl_seconds, "access")
    }

    pub fn issue_refresh_token(&self, user_id: Uuid, ttl_seconds: i64) -> Result<String, ApiError> {
        self.issue_token(user_id, ttl_seconds, "refresh")
    }

    fn issue_token(
        &self,
        user_id: Uuid,
        ttl_seconds: i64,
        scope: &str,
    ) -> Result<String, ApiError> {
        let now = Utc::now().timestamp();
        let claims = TokenClaims {
            sub: user_id,
            iat: now,
            exp: now + ttl_seconds,
            scope: scope.to_string(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|_| ApiError::Internal)
    }

    pub fn verify(&self, token: &str) -> Result<TokenClaims, ApiError> {
        if self.revoked_refresh_tokens.contains(token) {
            return Err(ApiError::Unauthorized);
        }
        decode::<TokenClaims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|_| ApiError::Unauthorized)
    }

    pub fn revoke_refresh(&self, token: String) {
        self.revoked_refresh_tokens.insert(token);
    }
}

pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::Internal)
}

pub fn verify_password(password: &str, expected_hash: &str) -> Result<bool, ApiError> {
    let parsed = PasswordHash::new(expected_hash).map_err(|_| ApiError::Internal)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn extract_access_subject(
    headers: &HeaderMap,
    token_service: &TokenService,
) -> Result<Uuid, ApiError> {
    let token = bearer_token(headers)?;
    let claims = token_service.verify(token)?;
    if claims.scope != "access" {
        return Err(ApiError::Unauthorized);
    }
    Ok(claims.sub)
}

pub fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get("authorization")
        .ok_or(ApiError::Unauthorized)?
        .to_str()
        .map_err(|_| ApiError::Unauthorized)?;

    let mut parts = value.splitn(2, ' ');
    let scheme = parts.next().unwrap_or_default();
    let token = parts.next().unwrap_or_default();

    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() {
        return Err(ApiError::Unauthorized);
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_scope_roundtrip() {
        let svc = TokenService::new(b"secret").expect("token service");
        let user_id = Uuid::new_v4();

        let access = svc.issue_access_token(user_id, 60).expect("access token");
        let refresh = svc.issue_refresh_token(user_id, 60).expect("refresh token");

        let access_claims = svc.verify(&access).expect("verify access");
        let refresh_claims = svc.verify(&refresh).expect("verify refresh");

        assert_eq!(access_claims.sub, user_id);
        assert_eq!(refresh_claims.sub, user_id);
        assert_eq!(access_claims.scope, "access");
        assert_eq!(refresh_claims.scope, "refresh");
    }

    #[test]
    fn revoked_refresh_is_rejected() {
        let svc = TokenService::new(b"secret").expect("token service");
        let token = svc
            .issue_refresh_token(Uuid::new_v4(), 60)
            .expect("refresh token");
        svc.revoke_refresh(token.clone());
        assert!(svc.verify(&token).is_err());
    }
}
