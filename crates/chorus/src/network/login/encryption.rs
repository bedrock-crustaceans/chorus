use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use p384::SecretKey;
use p384::pkcs8::{EncodePrivateKey, EncodePublicKey};
use serde_json::json;

pub fn get_handshake_jwt(secret: &SecretKey, token: &[u8; 16]) -> Option<String> {
    let public = secret.public_key();
    let public_der = BASE64_STANDARD.encode(public.to_public_key_der().ok()?.as_bytes());

    let mut header = Header::new(Algorithm::ES384);
    header.typ = None;
    header.x5u = Some(public_der);

    let claims = json!({
        "salt": BASE64_STANDARD.encode(token)
    });

    let enc_key = EncodingKey::from_ec_der(secret.to_pkcs8_der().ok()?.as_bytes());

    encode(&header, &claims, &enc_key).ok()
}
