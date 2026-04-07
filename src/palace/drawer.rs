use std::collections::HashMap;

use turso::Connection;

use crate::db;
use crate::error::Result;

/// Index the words in a drawer's content into the `drawer_words` table.
pub async fn index_words(conn: &Connection, drawer_id: &str, content: &str) -> Result<()> {
    let mut word_counts: HashMap<String, i32> = HashMap::new();
    for word in tokenize(content) {
        *word_counts.entry(word).or_insert(0) += 1;
    }

    for (word, count) in &word_counts {
        conn.execute(
            "INSERT OR IGNORE INTO drawer_words (word, drawer_id, count) VALUES (?1, ?2, ?3)",
            turso::params![word.as_str(), drawer_id, *count],
        )
        .await?;
    }

    Ok(())
}

/// Tokenize text into lowercase words, filtering stop words and short words.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() >= 3)
        .map(str::to_lowercase)
        .filter(|w| !is_stop_word(w))
        .collect()
}

fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "and"
            | "for"
            | "are"
            | "but"
            | "not"
            | "you"
            | "all"
            | "can"
            | "had"
            | "her"
            | "was"
            | "one"
            | "our"
            | "out"
            | "has"
            | "have"
            | "from"
            | "they"
            | "been"
            | "said"
            | "each"
            | "which"
            | "their"
            | "will"
            | "other"
            | "about"
            | "many"
            | "then"
            | "them"
            | "these"
            | "some"
            | "would"
            | "make"
            | "like"
            | "into"
            | "time"
            | "very"
            | "when"
            | "come"
            | "could"
            | "more"
            | "than"
            | "that"
            | "this"
            | "with"
            | "what"
            | "just"
            | "also"
            | "there"
            | "where"
            | "after"
            | "back"
            | "only"
            | "most"
            | "over"
            | "such"
            | "here"
            | "should"
            | "because"
            | "does"
            | "did"
            | "get"
            | "how"
            | "its"
            | "may"
            | "let"
            | "new"
            | "now"
            | "old"
            | "see"
            | "way"
            | "who"
            | "use"
            | "being"
            | "well"
    )
}

/// Check if a file has already been mined.
pub async fn file_already_mined(conn: &Connection, source_file: &str) -> Result<bool> {
    let rows = db::query_all(
        conn,
        "SELECT 1 FROM drawers WHERE source_file = ?1 LIMIT 1",
        turso::params![source_file],
    )
    .await?;
    Ok(!rows.is_empty())
}

pub struct DrawerParams<'a> {
    pub id: &'a str,
    pub wing: &'a str,
    pub room: &'a str,
    pub content: &'a str,
    pub source_file: &'a str,
    pub chunk_index: usize,
    pub added_by: &'a str,
    pub ingest_mode: &'a str,
}

/// Add a drawer and index its words.
pub async fn add_drawer(conn: &Connection, p: &DrawerParams<'_>) -> Result<bool> {
    // SQLite only has i64 integers, so we cast chunk_index (usize) to i32 at the SQL boundary.
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let chunk_index_sql = p.chunk_index as i32;
    let result = conn
        .execute(
            "INSERT OR IGNORE INTO drawers (id, wing, room, content, source_file, chunk_index, added_by, ingest_mode) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            turso::params![p.id, p.wing, p.room, p.content, p.source_file, chunk_index_sql, p.added_by, p.ingest_mode],
        )
        .await?;

    if result > 0 {
        index_words(conn, p.id, p.content).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
