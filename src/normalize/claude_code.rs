use super::messages_to_transcript;

/// Parse Claude Code JSONL format.
pub fn try_parse(content: &str) -> Option<String> {
    let mut messages: Vec<(String, String)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: serde_json::Value = serde_json::from_str(line).ok()?;
        let obj = entry.as_object()?;

        let msg_type = obj.get("type")?.as_str()?;
        let message = obj.get("message")?.as_object()?;
        let text = extract_content(message.get("content")?);

        if text.is_empty() {
            continue;
        }

        match msg_type {
            "human" => messages.push(("user".to_string(), text)),
            "assistant" => messages.push(("assistant".to_string(), text)),
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
        serde_json::Value::Object(obj) => obj
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string(),
        _ => String::new(),
    }
}
