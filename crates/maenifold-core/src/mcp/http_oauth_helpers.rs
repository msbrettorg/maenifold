async fn get_json(state: crate::api::routes::ApiState, uri: &str) -> Value {
    let response = test_router(state)
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&body).expect("json")
}

async fn post_json(
    state: crate::api::routes::ApiState,
    uri: &str,
    body: Value,
) -> axum::response::Response {
    test_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn register_client(state: crate::api::routes::ApiState, redirect_uri: &str) -> String {
    let response = post_json(
        state,
        "/register",
        json!({"redirect_uris":[redirect_uri],"token_endpoint_auth_method":"none"}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice::<Value>(&body).expect("json")["client_id"]
        .as_str()
        .expect("client_id")
        .to_owned()
}

async fn authorize_response(
    state: crate::api::routes::ApiState,
    uri: &str,
) -> axum::response::Response {
    let response = test_router(state)
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::FOUND);
    response
}

async fn authorize_code(
    state: crate::api::routes::ApiState,
    client_id: &str,
    redirect_uri: &str,
    challenge: &str,
    resource: &str,
) -> String {
    let uri = format!(
        "/authorize?response_type=code&client_id={client_id}&redirect_uri={}&code_challenge={challenge}&code_challenge_method=S256&resource={}",
        urlencoding::encode(redirect_uri),
        urlencoding::encode(resource)
    );
    let location = authorize_response(state, &uri).await;
    query_value(
        location.headers()[header::LOCATION].to_str().unwrap(),
        "code",
    )
    .expect("code")
}

async fn token_response(
    state: crate::api::routes::ApiState,
    body: &str,
) -> axum::response::Response {
    test_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(body.to_owned()))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn token_json(state: crate::api::routes::ApiState, body: &str) -> Value {
    let response = token_response(state, body).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&body).expect("json")
}

fn token_form(code: &str, verifier: &str, client_id: &str, resource: &str) -> String {
    format!(
        "grant_type=authorization_code&code={code}&code_verifier={verifier}&redirect_uri=http%3A%2F%2F127.0.0.1%2Fcb&client_id={client_id}&resource={}",
        urlencoding::encode(resource)
    )
}

fn pkce(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn query_value(location: &str, key: &str) -> Option<String> {
    location.split_once('?')?.1.split('&').find_map(|pair| {
        let (candidate, value) = pair.split_once('=')?;
        (candidate == key).then(|| value.to_owned())
    })
}
