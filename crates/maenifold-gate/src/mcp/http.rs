use super::{GateHandler, TransportKind};
use axum::{
    Json,
    extract::State,
    routing::{get, post},
};
use chrono::Utc;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::never::NeverSessionManager,
};
use serde_json::{Map, Value, json};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    trace_buffer: crate::trace::TraceBuffer,
}

async fn trace_data(State(state): State<AppState>) -> Json<Vec<crate::trace::TraceEntry>> {
    Json(state.trace_buffer.snapshot())
}

fn board_trace(
    trace_buffer: &crate::trace::TraceBuffer,
    direction: crate::trace::Direction,
    method: &str,
    id: &Value,
    body: Value,
) {
    trace_buffer.push(crate::trace::TraceEntry {
        timestamp: Utc::now().format("%H:%M:%S%.3f").to_string(),
        direction,
        method: method.to_string(),
        id: Some(id.clone()),
        body: serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()),
        session_id: None,
    });
}

fn tool_result(text: String, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

fn jsonrpc_error(id: Value, message: impl Into<String>) -> Json<Value> {
    Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32603, "message": message.into() }
    }))
}

fn should_strip_gate_status_sections(tool_name: &str, arguments: &Map<String, Value>) -> bool {
    if tool_name != "gate_status" {
        return false;
    }

    match arguments.get("id") {
        None | Some(Value::Null) => true,
        Some(Value::String(id)) => id.trim().is_empty(),
        _ => false,
    }
}

fn strip_sections_from_gate_status_list(text: String) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(&text) else {
        return text;
    };
    let Value::Array(tasks) = &mut value else {
        return text;
    };

    for task in tasks {
        if let Value::Object(task) = task {
            task.remove("sections");
        }
    }

    serde_json::to_string(&value).unwrap_or(text)
}

async fn board_api(State(state): State<AppState>, Json(body): Json<Value>) -> Json<Value> {
    let id = body.get("id").cloned().unwrap_or(Value::Null);
    let method = body
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if method != "tools/call" {
        return jsonrpc_error(id, format!("unsupported method: {method}"));
    }

    let params = body
        .get("params")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let tool_name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut arguments = match params.get("arguments") {
        None => Map::new(),
        Some(Value::Object(map)) => map.clone(),
        Some(_) => return jsonrpc_error(id, "tool arguments must be a JSON object"),
    };

    board_trace(
        &state.trace_buffer,
        crate::trace::Direction::Incoming,
        &tool_name,
        &id,
        json!({
            "jsonrpc": "2.0",
            "id": id.clone(),
            "method": method,
            "params": params,
        }),
    );

    let strip_gate_status_sections = should_strip_gate_status_sections(&tool_name, &arguments);
    let tool_name_for_dispatch = tool_name.clone();
    let result = match tokio::task::spawn_blocking(move || {
        super::tools::dispatch(&tool_name_for_dispatch, &mut arguments)
    })
    .await
    {
        Ok(Ok(dispatch)) => {
            let output = if strip_gate_status_sections {
                strip_sections_from_gate_status_list(dispatch.output)
            } else {
                dispatch.output
            };
            Ok(tool_result(output, false))
        }
        Ok(Err(err)) => Ok(tool_result(err, true)),
        Err(err) => Err(format!("tool execution failed: {err}")),
    };

    let response = match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(message) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32603, "message": message }
        }),
    };
    board_trace(
        &state.trace_buffer,
        crate::trace::Direction::Outgoing,
        &tool_name,
        &id,
        response.clone(),
    );

    Json(response)
}

impl super::GateMcpServer {
    pub async fn serve_http(port: u16) -> anyhow::Result<()> {
        let ct = tokio_util::sync::CancellationToken::new();
        let trace_buffer = crate::trace::TraceBuffer::new(20);
        let app_state = AppState {
            trace_buffer: trace_buffer.clone(),
        };
        let session_manager = Arc::new(NeverSessionManager::default());
        // Serve both HTTP entrypoints in stateless JSON-response mode so daemon restarts do not
        // reject clients that resend a stale `Mcp-Session-Id`. In this mode MCP remains POST-only;
        // clients should continue sending `MCP-Protocol-Version`, but the server no longer requires
        // session affinity via `Mcp-Session-Id` and responds with JSON instead of SSE setup.

        let root_service = StreamableHttpService::new(
            {
                let trace_buffer = trace_buffer.clone();
                move || {
                    Ok::<_, std::io::Error>(GateHandler::new(
                        TransportKind::Http,
                        Some(trace_buffer.clone()),
                    ))
                }
            },
            session_manager.clone(),
            StreamableHttpServerConfig {
                stateful_mode: false,
                json_response: true,
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );
        let mcp_service = StreamableHttpService::new(
            {
                let trace_buffer = trace_buffer.clone();
                move || {
                    Ok::<_, std::io::Error>(GateHandler::new(
                        TransportKind::Http,
                        Some(trace_buffer.clone()),
                    ))
                }
            },
            session_manager,
            StreamableHttpServerConfig {
                stateful_mode: false,
                json_response: true,
                cancellation_token: ct.child_token(),
                ..Default::default()
            },
        );
        let router = axum::Router::new()
            .route("/", get(crate::board::index).post_service(root_service))
            .route("/trace", get(crate::board::trace))
            .route("/trace/data", get(trace_data))
            .route("/board/api", post(board_api))
            .nest_service("/mcp", mcp_service)
            .layer(super::localhost_cors())
            .with_state(app_state);

        let bind_addr = format!("127.0.0.1:{port}");
        let tcp_listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP bind failed on {bind_addr}: {e}"))?;

        eprintln!("gate board listening on http://{bind_addr}/");

        axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move {
                let _ = tokio::signal::ctrl_c().await;
                ct.cancel();
            })
            .await
            .map_err(|e| anyhow::anyhow!("HTTP server error: {e}"))?;

        Ok(())
    }
}
