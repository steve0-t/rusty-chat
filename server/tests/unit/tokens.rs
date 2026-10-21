use std::collections::HashSet;

use crate::{generate_access_token, hash_token};

#[test]
fn access_token_is_48_random_bytes_encoded_as_lowercase_hex() {
    let token = generate_access_token().0;
    assert_eq!(token.len(), 96);
    assert!(
        token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert_eq!(hex::decode(&token).expect("hex access token").len(), 48);
}

#[test]
fn separately_generated_tokens_do_not_repeat() {
    // A collision here has negligible probability for 384-bit random tokens.
    let tokens: HashSet<_> = (0..512).map(|_| generate_access_token().0).collect();
    assert_eq!(
        tokens.len(),
        512,
        "access tokens must be fresh for each session"
    );
}

#[test]
fn token_generation_is_safe_across_threads() {
    let workers: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                (0..64)
                    .map(|_| generate_access_token().0)
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let tokens: HashSet<_> = workers
        .into_iter()
        .flat_map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(tokens.len(), 512);
}

#[test]
fn debug_output_does_not_disclose_a_bearer_credential() {
    let token = generate_access_token();
    assert!(
        !format!("{token:?}").contains(&token.0),
        "AccessToken Debug must redact the secret"
    );
}

// Independent, published SHA-512 vectors pin down the persistent hash format.
macro_rules! sha512_vector {
    ($name:ident, $input:expr, $expected:literal) => {
        #[test]
        fn $name() {
            assert_eq!(hash_token($input).as_deref(), Some($expected));
        }
    };
}

sha512_vector!(
    empty_string_sha512,
    "",
    "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
);
sha512_vector!(
    abc_sha512,
    "abc",
    "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
);
sha512_vector!(
    long_message_sha512,
    "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu",
    "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909"
);

#[test]
fn generated_tokens_can_always_be_hashed_for_storage() {
    for _ in 0..64 {
        let token = generate_access_token().0;
        let hash = hash_token(&token).expect("all generated tokens must be hashable");
        assert_eq!(
            hash.len(),
            128,
            "SHA-512 storage format must use hexadecimal"
        );
        assert!(
            hash.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        assert_ne!(hash, token);
    }
}

#[test]
fn hashing_is_deterministic() {
    let first = hash_token("repeatable-session").expect("valid token hash");
    assert_eq!(hash_token("repeatable-session"), Some(first));
}

#[test]
fn hashing_preserves_case_and_whitespace() {
    let hashes: HashSet<_> = ["token", "Token", " token", "token ", "token\n"]
        .into_iter()
        .map(|token| hash_token(token).expect("valid token hash"))
        .collect();
    assert_eq!(hashes.len(), 5, "credentials must be hashed byte for byte");
}

#[test]
fn hashing_supports_unicode_and_embedded_nul_without_truncation() {
    let hashes: HashSet<_> = ["žluťoučký 🔐", "token\0suffix", "token\0", "token"]
        .into_iter()
        .map(|token| hash_token(token).expect("UTF-8 strings are valid hash inputs"))
        .collect();
    assert_eq!(hashes.len(), 4);
}
