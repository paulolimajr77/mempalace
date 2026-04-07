use super::messages_to_transcript;

/// Parse Claude.ai JSON export: [{"role": "user", "content": "..."}]
pub fn try_parse(data: &serde_json::Value) -> Option<String> {
    let items = if let Some(arr) = data.as_array() {
        arr.clone()
    } else if let Some(obj) = data.as_object() {
        obj.get("messages")
            .or_else(|| obj.get("chat_messages"))
            .and_then(|v| v.as_array())
            .cloned()?
    } else {
        return None;
    };

    let mut messages: Vec<(String, String)> = Vec::new();

    for item in &items {
        let obj = item.as_object()?;
        let role = obj.get("role")?.as_str()?;
        let content = obj.get("content")?;
        let text = extract_content(content);
        if text.is_empty() {
            continue;
        }

        match role {
            "user" | "human" => messages.push(("user".to_string(), text)),
            "assistant" | "ai" => messages.push(("assistant".to_string(), text)),
            _ => {}
        }
    }

    if messages.len() >= 2 {
        let refs: Vec<(&str, &str)> = messages
            .iter()
            .map(|(r, t)| (r.as_str(), t.as_str()))
            .collect();
        Some(messages_to_transcript(&refs))
    } else {
        None
    }
}

fn extract_content(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.trim().to_string(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                if let Some(s) = item.as_str() {
                    Some(s.to_string())
                } else if let Some(obj) = item.as_object() {
                    if obj.get("type").and_then(|t| t.as_str()) == Some("text") {
                        obj.get("text")
                            .and_then(|t| t.as_str())
                            .map(std::string::ToString::to_string)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string(),
        _ => String::new(),
    }
}
