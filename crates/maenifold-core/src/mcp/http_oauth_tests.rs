use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[tokio::test]
#[serial_test::serial]
async fn mcp_unauthorized_advertises_protected_resource_metadata() {
    let (_temp, state, _guard) = test_state();
    let response = test_router(state)
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .header(header::HOST, "127.0.0.1:4567")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        response.headers()[header::WWW_AUTHENTICATE],
        "Bearer resource_metadata=\"http://127.0.0.1:4567/.well-known/oauth-protected-resource/mcp\""
    );
}

#[tokio::test]
#[serial_test::serial]
async fn oauth_metadata_is_available_at_bare_and_mcp_paths() {
    let (_temp, state, _guard) = test_state();
    for path in [
        "/.well-known/oauth-protected-resource",
        "/.well-known/oauth-protected-resource/mcp",
    ] {
        let json = get_json(state.clone(), path).await;
        assert_eq!(json["resource"], "http://127.0.0.1:8080/mcp");
        assert_eq!(
            json["authorization_servers"],
            json!(["http://127.0.0.1:8080"])
        );
    }
    for path in [
        "/.well-known/oauth-authorization-server",
        "/.well-known/oauth-authorization-server/mcp",
    ] {
        let json = get_json(state.clone(), path).await;
        assert_eq!(
            json["authorization_endpoint"],
            "http://127.0.0.1:8080/authorize"
        );
        assert_eq!(
            json["registration_endpoint"],
            "http://127.0.0.1:8080/register"
        );
        assert_eq!(json["code_challenge_methods_supported"], json!(["S256"]));
    }
}

#[tokio::test]
#[serial_test::serial]
async fn oauth_registration_rejects_non_loopback_redirect_uri() {
    let (_temp, state, _guard) = test_state();
    let response = post_json(
        state,
        "/register",
        json!({"redirect_uris":["https://example.com/cb"],"token_endpoint_auth_method":"none"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial_test::serial]
async fn oauth_pkce_round_trip_returns_token_that_reaches_mcp() {
    let (temp, state, _guard) = test_state();
    crate::api::auth::generate_and_write_token(temp.path()).expect("token");
    let token = std::fs::read_to_string(crate::api::auth::token_path(temp.path()))
        .expect("read token")
        .trim()
        .to_owned();
    let client_id = register_client(state.clone(), "http://127.0.0.1/cb").await;
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    let challenge = pkce(verifier);
    let code = authorize_code(
        state.clone(),
        &client_id,
        "http://127.0.0.1/cb",
        &challenge,
        "http://127.0.0.1:8080/mcp",
    )
    .await;
    let json = token_json(
        state.clone(),
        &format!(
            "grant_type=authorization_code&code={code}&code_verifier={verifier}&redirect_uri=http%3A%2F%2F127.0.0.1%2Fcb&client_id={client_id}&resource=http%3A%2F%2F127.0.0.1%3A8080%2Fmcp"
        ),
    )
    .await;

    assert_eq!(json["access_token"], token);
    let response = test_router(state)
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial_test::serial]
async fn oauth_authorize_requires_s256_pkce_and_resource() {
    let (_temp, state, _guard) = test_state();
    let client_id = register_client(state.clone(), "http://localhost/cb").await;
    let response = authorize_response(
        state.clone(),
        &format!(
            "/authorize?response_type=code&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge=abc&code_challenge_method=plain&resource=http%3A%2F%2F127.0.0.1%3A8080%2Fmcp"
        ),
    )
    .await;
    assert!(
        response.headers()[header::LOCATION]
            .to_str()
            .unwrap()
            .contains("error=")
    );

    let response = authorize_response(
        state,
        &format!(
            "/authorize?response_type=code&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge=abc&code_challenge_method=S256"
        ),
    )
    .await;
    assert!(
        response.headers()[header::LOCATION]
            .to_str()
            .unwrap()
            .contains("error=")
    );
}

#[tokio::test]
#[serial_test::serial]
async fn oauth_token_rejects_expired_mismatched_resource_client_and_verifier() {
    let (temp, state, _guard) = test_state();
    crate::api::auth::generate_and_write_token(temp.path()).expect("token");
    let client_id = register_client(state.clone(), "http://127.0.0.1/cb").await;
    let verifier = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";
    let challenge = pkce(verifier);
    let code = authorize_code(
        state.clone(),
        &client_id,
        "http://127.0.0.1/cb",
        &challenge,
        "http://127.0.0.1:8080/mcp",
    )
    .await;
    assert_eq!(
        token_response(
            state.clone(),
            &token_form(&code, verifier, "other", "http://127.0.0.1:8080/mcp"),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );

    let code = authorize_code(
        state.clone(),
        &client_id,
        "http://127.0.0.1/cb",
        &challenge,
        "http://127.0.0.1:8080/mcp",
    )
    .await;
    assert_eq!(
        token_response(
            state.clone(),
            &token_form(&code, verifier, &client_id, "http://127.0.0.1:8080/other"),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );

    let code = authorize_code(
        state.clone(),
        &client_id,
        "http://127.0.0.1/cb",
        &challenge,
        "http://127.0.0.1:8080/mcp",
    )
    .await;
    assert_eq!(
        token_response(
            state.clone(),
            &token_form(&code, "bad", &client_id, "http://127.0.0.1:8080/mcp"),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );

    crate::api::oauth::insert_expired_code_for_test(
        "expired-code".to_owned(),
        client_id.clone(),
        "http://127.0.0.1/cb".to_owned(),
        challenge,
        "http://127.0.0.1:8080/mcp".to_owned(),
    );
    assert_eq!(
        token_response(
            state,
            &token_form(
                "expired-code",
                verifier,
                &client_id,
                "http://127.0.0.1:8080/mcp"
            ),
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
}
