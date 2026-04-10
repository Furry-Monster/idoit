#![allow(dead_code)]

/// Parse Server-Sent Events from raw SSE bytes.
/// Returns an iterator of data payloads (lines starting with "data: ").
pub fn parse_sse_events(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|line| {
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return None;
                }
                Some(data.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Extract content delta from an OpenAI-style SSE chunk.
pub fn extract_openai_delta(json_data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json_data).ok()?;
    v.get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")?
        .as_str()
        .map(|s| s.to_string())
}

/// Extract content delta from an Anthropic-style SSE chunk.
pub fn extract_anthropic_delta(json_data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json_data).ok()?;
    let event_type = v.get("type")?.as_str()?;
    if event_type == "content_block_delta" {
        v.get("delta")?.get("text")?.as_str().map(|s| s.to_string())
    } else {
        None
    }
}

/// Extract content from a Gemini streaming chunk.
pub fn extract_gemini_delta(json_data: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json_data).ok()?;
    v.get("candidates")?
        .get(0)?
        .get("content")?
        .get("parts")?
        .get(0)?
        .get("text")?
        .as_str()
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_events() {
        let raw = "data: {\"id\":\"1\"}\n\ndata: {\"id\":\"2\"}\n\ndata: [DONE]\n\n";
        let events = parse_sse_events(raw);
        assert_eq!(events.len(), 2);
        assert!(events[0].contains("\"id\":\"1\""));
    }

    #[test]
    fn test_extract_openai_delta() {
        let json = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
        assert_eq!(extract_openai_delta(json), Some("hello".to_string()));
    }

    #[test]
    fn test_extract_anthropic_delta() {
        let json = r#"{"type":"content_block_delta","delta":{"text":"world"}}"#;
        assert_eq!(extract_anthropic_delta(json), Some("world".to_string()));
    }

    #[test]
    fn test_extract_gemini_delta() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"foo"}]}}]}"#;
        assert_eq!(extract_gemini_delta(json), Some("foo".to_string()));
    }
}
