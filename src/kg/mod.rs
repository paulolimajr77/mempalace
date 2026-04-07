pub mod query;

use turso::Connection;

use crate::db;
use crate::error::Result;

/// Normalize an entity name to an ID: lowercase, spaces→underscores, strip apostrophes.
pub fn entity_id(name: &str) -> String {
    name.to_lowercase().replace(' ', "_").replace('\'', "")
}

/// Add or update an entity node.
#[allow(dead_code)]
pub async fn add_entity(
    conn: &Connection,
    name: &str,
    entity_type: &str,
    properties: Option<&str>,
) -> Result<String> {
    let eid = entity_id(name);
    let props = properties.unwrap_or("{}");
    conn.execute(
        "INSERT OR REPLACE INTO entities (id, name, type, properties) VALUES (?1, ?2, ?3, ?4)",
        turso::params![eid.as_str(), name, entity_type, props],
    )
    .await?;
    Ok(eid)
}

/// Parameters for [`add_triple`].
pub struct TripleParams<'a> {
    pub subject: &'a str,
    pub predicate: &'a str,
    pub object: &'a str,
    pub valid_from: Option<&'a str>,
    pub valid_to: Option<&'a str>,
    pub confidence: f64,
    pub source_closet: Option<&'a str>,
    pub source_file: Option<&'a str>,
}

/// Add a relationship triple. Auto-creates entities if they don't exist.
/// Returns the triple ID.
pub async fn add_triple(conn: &Connection, p: &TripleParams<'_>) -> Result<String> {
    let sub_id = entity_id(p.subject);
    let obj_id = entity_id(p.object);
    let pred = p.predicate.to_lowercase().replace(' ', "_");

    // Auto-create entities
    conn.execute(
        "INSERT OR IGNORE INTO entities (id, name) VALUES (?1, ?2)",
        turso::params![sub_id.as_str(), p.subject],
    )
    .await?;
    conn.execute(
        "INSERT OR IGNORE INTO entities (id, name) VALUES (?1, ?2)",
        turso::params![obj_id.as_str(), p.object],
    )
    .await?;

    // Check for existing identical active triple
    let existing = db::query_all(
        conn,
        "SELECT id FROM triples WHERE subject=?1 AND predicate=?2 AND object=?3 AND valid_to IS NULL",
        turso::params![sub_id.as_str(), pred.as_str(), obj_id.as_str()],
    )
    .await?;

    if let Some(row) = existing.first()
        && let Ok(val) = row.get_value(0)
        && let Some(id) = val.as_text()
    {
        return Ok(id.clone());
    }

    let triple_id = format!(
        "t_{sub_id}_{pred}_{obj_id}_{}",
        &uuid::Uuid::new_v4().to_string().replace('-', "")[..8]
    );

    let vf: turso::Value = match p.valid_from {
        Some(v) => turso::Value::from(v),
        None => turso::Value::Null,
    };
    let vt: turso::Value = match p.valid_to {
        Some(v) => turso::Value::from(v),
        None => turso::Value::Null,
    };
    let sc: turso::Value = match p.source_closet {
        Some(v) => turso::Value::from(v),
        None => turso::Value::Null,
    };
    let sf: turso::Value = match p.source_file {
        Some(v) => turso::Value::from(v),
        None => turso::Value::Null,
    };

    conn.execute(
        "INSERT INTO triples (id, subject, predicate, object, valid_from, valid_to, confidence, source_closet, source_file) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        turso::params![triple_id.as_str(), sub_id.as_str(), pred.as_str(), obj_id.as_str(), vf, vt, p.confidence, sc, sf],
    )
    .await?;

    Ok(triple_id)
}

/// Mark a relationship as ended (set `valid_to`).
pub async fn invalidate(
    conn: &Connection,
    subject: &str,
    predicate: &str,
    object: &str,
    ended: Option<&str>,
) -> Result<()> {
    let sub_id = entity_id(subject);
    let obj_id = entity_id(object);
    let pred = predicate.to_lowercase().replace(' ', "_");
    let ended = ended.map_or_else(
        || chrono::Local::now().format("%Y-%m-%d").to_string(),
        std::string::ToString::to_string,
    );

    conn.execute(
        "UPDATE triples SET valid_to=?1 WHERE subject=?2 AND predicate=?3 AND object=?4 AND valid_to IS NULL",
        turso::params![ended.as_str(), sub_id.as_str(), pred.as_str(), obj_id.as_str()],
    )
    .await?;

    Ok(())
}
