use crate::error::{MaenifoldError, Result};
use crate::mcp::schema::{
    AcpExecContextBlock, AcpExecImageContext, AcpExecResourceContext, AcpExecTextContext,
};
use serde_json::{Map, Value};

pub(crate) fn optional_context_pairs(payload: &Value) -> Result<Vec<(String, String)>> {
    let Some(arr) = payload.get("context").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    arr.iter()
        .enumerate()
        .map(|(i, item)| {
            let block = serde_json::from_value::<AcpExecContextBlock>(item.clone())
                .map_err(|e| MaenifoldError::InvalidArgument(format!("context[{i}]: {e}")))?;
            context_block_into_pair(i, block)
        })
        .collect()
}

fn context_block_into_pair(index: usize, block: AcpExecContextBlock) -> Result<(String, String)> {
    Ok(match block {
        AcpExecContextBlock::Text(AcpExecTextContext { text }) => {
            (format!("context_text_{index}"), text)
        }
        AcpExecContextBlock::Image(AcpExecImageContext {
            data,
            media_type,
            uri,
        }) => (
            format!("context_image_{index}"),
            format!(
                "media_type={}; uri={}; bytes={}",
                media_type,
                uri.unwrap_or_default(),
                data.len()
            ),
        ),
        AcpExecContextBlock::Resource(AcpExecResourceContext {
            uri,
            text,
            blob,
            media_type,
        }) => {
            let value = match (text, blob) {
                (Some(text), None) => {
                    let mut resource = Map::new();
                    resource.insert("uri".to_string(), Value::String(uri));
                    resource.insert(
                        "mediaType".to_string(),
                        media_type.map(Value::String).unwrap_or(Value::Null),
                    );
                    resource.insert("text".to_string(), Value::String(text));
                    Value::Object(resource)
                }
                (None, Some(blob)) => {
                    let mut resource = Map::new();
                    resource.insert("uri".to_string(), Value::String(uri));
                    resource.insert(
                        "mediaType".to_string(),
                        media_type.map(Value::String).unwrap_or(Value::Null),
                    );
                    resource.insert(
                        "blobBytes".to_string(),
                        Value::Number(serde_json::Number::from(blob.len() as u64)),
                    );
                    Value::Object(resource)
                }
                (Some(_), Some(_)) => {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "context[{index}]: resource requires exactly one of 'text' or 'blob'"
                    )));
                }
                (None, None) => {
                    return Err(MaenifoldError::InvalidArgument(format!(
                        "context[{index}]: resource requires either 'text' or 'blob'"
                    )));
                }
            };

            (
                format!("context_resource_{index}"),
                serde_json::to_string(&value)?,
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::context_block_into_pair;
    use crate::mcp::schema::{AcpExecContextBlock, AcpExecResourceContext};

    #[test]
    fn resource_context_with_text_is_serialized() {
        let pair = context_block_into_pair(
            0,
            AcpExecContextBlock::Resource(AcpExecResourceContext {
                uri: "memory://note".to_string(),
                text: Some("hello".to_string()),
                blob: None,
                media_type: Some("text/plain".to_string()),
            }),
        )
        .expect("context pair");

        assert_eq!(pair.0, "context_resource_0");
        assert!(pair.1.contains("memory://note"));
        assert!(pair.1.contains("hello"));
    }
}
