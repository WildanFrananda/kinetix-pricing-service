use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use kinetix_pricing_service::security::jwt::{JwtError, JwtVerifier};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const JWKS: &str = r#"{"keys":[{"kty":"RSA","kid":"k1","alg":"RS256","use":"sig","n":"y5itqBFwHEddc-c4foOzOpgZf9OcVFrqYBrcKN09FrPXrgAZIolbACIMXh9i6Q0ispV6ZEHZ1jQ9-hrVPKJi4gBafwBnxC76Y5TJV4Ea5-5yjw7xyKWSjUR1TD5zFmif2ASzYel_jUJMEI3qo7IHHrktWdbAtx-44AWhF8z5VcQTk9B0oqdctUzssP5GswZcOzQqUDriZ2jHUHfU6HS1EqH5eH2UOBcPUrNc63M36KtH7LNgM_-HFZHWHUGixOi1Wv9MYkAzj8jZQz9MIE5Yj2Cl6ETKOLkEDY_9YvpbL3hLhpPxp-oS1u0WRSKilCRNIyyRMuPzFWG0TH7qWckfPA","e":"AQAB"}]}"#;

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let symbols = chunk.len() + 1;
        for i in 0..symbols {
            out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
        }
    }
    return out;
}

fn forged(kid: &str) -> String {
    let header = base64url(format!(r#"{{"alg":"RS256","typ":"JWT","kid":"{kid}"}}"#).as_bytes());
    let claims = base64url(br#"{"sub":"p"}"#);
    return format!("{header}.{claims}.c2lnbmF0dXJl");
}

async fn identity(fetches: Arc<AtomicUsize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            fetches.fetch_add(1, Ordering::SeqCst);
            let mut request = [0_u8; 2048];
            let _ = socket.read(&mut request).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                JWKS.len(),
                JWKS
            );
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });

    return format!("http://{address}/.well-known/jwks.json");
}

#[tokio::test]
async fn unknown_keys_do_not_make_identity_answer_every_request() {
    let fetches = Arc::new(AtomicUsize::new(0));
    let url = identity(fetches.clone()).await;
    std::env::set_var("IDENTITY_JWKS_URL", &url);
    std::env::set_var("JWT_ISSUER", "https://identity.test");
    std::env::set_var("JWT_AUDIENCE", "kinetix");
    let verifier = JwtVerifier::from_env().expect("verifier");

    verifier.refresh().await.expect("first fetch");

    for i in 0..50 {
        let answer = verifier
            .verify_access(&forged(&format!("forged-{i}")))
            .await;
        assert!(
            matches!(answer, Err(JwtError::Invalid)),
            "forged-{i} answered {answer:?}"
        );
    }

    assert_eq!(fetches.load(Ordering::SeqCst), 1);
}
