use anyhow::{anyhow, Result};
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey},
    RsaPrivateKey,
};

use crate::models::user::{Claims, User};

pub fn generate_rsa_keys() -> (String, String) {
    let mut rng = OsRng;
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate RSA key");
    let private_pem = private_key
        .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
        .unwrap()
        .to_string();
    let public_key = private_key.to_public_key();
    let public_pem = public_key
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .unwrap();
    (private_pem, public_pem)
}

pub fn generate_tokens(
    user: &User,
    private_key_pem: &str,
) -> Result<(String, String)> {
    let now = Utc::now().timestamp() as usize;

    let access_claims = Claims {
        sub: user.id.to_string(),
        role: user.role.clone(),
        home_hub_id: user.home_hub_id.map(|id| id.to_string()),
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
    };

    let refresh_claims = Claims {
        sub: user.id.to_string(),
        role: user.role.clone(),
        home_hub_id: user.home_hub_id.map(|id| id.to_string()),
        exp: now + 30 * 24 * 3600,
        iat: now,
        token_type: "refresh".to_string(),
    };

    let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| anyhow!("Failed to create encoding key: {}", e))?;

    let header = Header::new(Algorithm::RS256);

    let access_token = encode(&header, &access_claims, &encoding_key)
        .map_err(|e| anyhow!("Failed to encode access token: {}", e))?;

    let refresh_token = encode(&header, &refresh_claims, &encoding_key)
        .map_err(|e| anyhow!("Failed to encode refresh token: {}", e))?;

    Ok((access_token, refresh_token))
}

pub fn validate_token(token: &str, public_key_pem: &str) -> Result<Claims> {
    let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
        .map_err(|e| anyhow!("Failed to create decoding key: {}", e))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true;

    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|e| anyhow!("Failed to validate token: {}", e))?;

    Ok(token_data.claims)
}
