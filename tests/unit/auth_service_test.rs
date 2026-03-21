use axis_core::models::user::User;
use axis_core::services::auth_service::{generate_rsa_keys, generate_tokens, validate_token};
use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_generate_rsa_keys() {
    let (private_key, public_key) = generate_rsa_keys();

    assert!(private_key.contains("BEGIN PRIVATE KEY"));
    assert!(public_key.contains("BEGIN PUBLIC KEY"));
    assert!(private_key.len() > 1000);
}

#[test]
fn test_generate_and_validate_tokens() {
    let (private_key, public_key) = generate_rsa_keys();

    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        password_hash: "hash".to_string(),
        role: "driver".to_string(),
        home_hub_id: None,
        active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let (access_token, refresh_token) = generate_tokens(&user, &private_key).unwrap();

    // Validar token de acesso
    let claims = validate_token(&access_token, &public_key).unwrap();
    assert_eq!(claims.sub, user.id.to_string());
    assert_eq!(claims.role, "driver");
    assert_eq!(claims.token_type, "access");

    // Validar token de refresh
    let refresh_claims = validate_token(&refresh_token, &public_key).unwrap();
    assert_eq!(refresh_claims.sub, user.id.to_string());
    assert_eq!(refresh_claims.token_type, "refresh");
}

#[test]
fn test_validate_token_with_invalid_signature() {
    let (private_key1, _) = generate_rsa_keys();
    let (_, public_key2) = generate_rsa_keys();

    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        password_hash: "hash".to_string(),
        role: "driver".to_string(),
        home_hub_id: None,
        active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let (token, _) = generate_tokens(&user, &private_key1).unwrap();

    // Deve falhar com chave pública errada
    let result = validate_token(&token, &public_key2);
    assert!(result.is_err());
}

#[test]
fn test_access_and_refresh_tokens_are_different() {
    let (private_key, _) = generate_rsa_keys();

    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        password_hash: "hash".to_string(),
        role: "franchisee".to_string(),
        home_hub_id: None,
        active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let (access_token, refresh_token) = generate_tokens(&user, &private_key).unwrap();

    // Os dois tokens devem ser diferentes
    assert_ne!(access_token, refresh_token);
}
