use std::collections::HashSet;

use super::messages_to_transcript;

/// Parse `ChatGPT` conversations.json with mapping tree.
pub fn try_parse(data: &serde_json::Value) -> Option<String> {
    let mapping = data.as_object()?.get("mapping")?.as_object()?;

    // Find root node (parent=null, no message)
    let mut root_id: Option<&str> = None;
    let mut fallback_root: Option<&str> = None;

    for (node_id, node) in mapping {
        if node.get("parent").is_some_and(serde_json::Value::is_null) {
            if node.get("message").is_none_or(serde_json::Value::is_null) {
                root_id = Some(node_id.as_str());
                break;
            } else if fallback_root.is_none() {
                fallback_root = Some(node_id.as_str());
            }
        }
    }

    let root = root_id.or(fallback_root)?;
    let mut messages: Vec<(String, String)> = Vec::new();
    let mut current_id = root.to_string();
    let mut visited = HashSet::new();

    while !visited.contains(&current_id) {
        visited.insert(current_id.clone());
        let node = mapping.get(&current_id)?;

        if let Some(msg) = node.get("message")
            && !msg.is_null()
        {
            let role = msg.get("author")?.get("role")?.as_str()?;
            let content = msg.get("content")?;
            let parts = content.get("parts").and_then(|p| p.as_array());

            let text = parts
                .map(|ps| {
                    ps.iter()
                        .filter_map(|p| p.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default()
                .trim()
                .to_string();

            if !text.is_empty() {
                match role {
                    "user" => messages.push(("user".to_string(), text)),
                    "assistant" => messages.push(("assistant".to_string(), text)),
                    _ => {}
                }
            }
        }

        let children = node.get("children").and_then(|c| c.as_array());
        if let Some(kids) = children {
            if let Some(first) = kids.first().and_then(|k| k.as_str()) {
                current_id = first.to_string();
            } else {
                break;
            }
        } else {
            break;
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
