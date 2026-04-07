use std::collections::HashMap;

use super::messages_to_transcript;

/// Parse Slack JSON export: [{"type": "message", "user": "...", "text": "..."}]
pub fn try_parse(data: &serde_json::Value) -> Option<String> {
    let items = data.as_array()?;
    let mut messages: Vec<(String, String)> = Vec::new();
    let mut seen_users: HashMap<String, String> = HashMap::new();
    let mut last_role: Option<String> = None;

    for item in items {
        let obj = item.as_object()?;
        if obj.get("type").and_then(|t| t.as_str()) != Some("message") {
            continue;
        }

        let user_id = obj
            .get("user")
            .or_else(|| obj.get("username"))
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();
        let text = obj
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        if text.is_empty() || user_id.is_empty() {
            continue;
        }

        if !seen_users.contains_key(&user_id) {
            let role = if seen_users.is_empty() {
                "user".to_string()
            } else if last_role.as_deref() == Some("user") {
                "assistant".to_string()
            } else {
                "user".to_string()
            };
            seen_users.insert(user_id.clone(), role);
        }

        let role = seen_users[&user_id].clone();
        last_role = Some(role.clone());
        messages.push((role, text));
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
