/// Verify inverted index search works with turso (FTS5 is not supported).
#[tokio::test]
async fn test_inverted_index_search() {
    let db = turso::Builder::new_local(":memory:")
        .build()
        .await
        .expect("failed to create db");

    let conn = db.connect().expect("failed to connect");

    conn.execute(
        "CREATE TABLE docs (id TEXT PRIMARY KEY, content TEXT NOT NULL, wing TEXT, room TEXT)",
        (),
    )
    .await
    .unwrap();

    conn.execute(
        "CREATE TABLE doc_words (word TEXT NOT NULL, doc_id TEXT NOT NULL, count INTEGER DEFAULT 1, PRIMARY KEY (word, doc_id))",
        (),
    )
    .await
    .unwrap();

    conn.execute("CREATE INDEX idx_w ON doc_words(word)", ())
        .await
        .unwrap();

    // Insert docs
    conn.execute(
        "INSERT INTO docs VALUES ('1', 'the quick brown fox jumps over the lazy dog', 'animals', 'general')",
        (),
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO docs VALUES ('2', 'rust programming language is fast and safe', 'tech', 'backend')",
        (),
    )
    .await
    .unwrap();

    // Build inverted index for doc 1
    for (word, count) in [
        ("quick", 1),
        ("brown", 1),
        ("fox", 1),
        ("jumps", 1),
        ("lazy", 1),
        ("dog", 1),
    ] {
        conn.execute(
            "INSERT INTO doc_words (word, doc_id, count) VALUES (?1, '1', ?2)",
            turso::params![word, count],
        )
        .await
        .unwrap();
    }

    // Build inverted index for doc 2
    for (word, count) in [
        ("rust", 1),
        ("programming", 1),
        ("language", 1),
        ("fast", 1),
        ("safe", 1),
    ] {
        conn.execute(
            "INSERT INTO doc_words (word, doc_id, count) VALUES (?1, '2', ?2)",
            turso::params![word, count],
        )
        .await
        .unwrap();
    }

    // Search for "fox" — should find doc 1
    let mut rows = conn
        .query(
            "SELECT d.id, d.content, SUM(dw.count) as relevance FROM docs d JOIN doc_words dw ON d.id = dw.doc_id WHERE dw.word IN ('fox') GROUP BY d.id ORDER BY relevance DESC",
            (),
        )
        .await
        .unwrap();

    let row = rows.next().await.unwrap().expect("no results for 'fox'");
    assert_eq!(row.get_value(0).unwrap().as_text().unwrap(), "1");

    // Search for "rust fast" — should find doc 2
    let mut rows = conn
        .query(
            "SELECT d.id, d.content, SUM(dw.count) as relevance FROM docs d JOIN doc_words dw ON d.id = dw.doc_id WHERE dw.word IN ('rust', 'fast') GROUP BY d.id ORDER BY relevance DESC",
            (),
        )
        .await
        .unwrap();

    let row = rows
        .next()
        .await
        .unwrap()
        .expect("no results for 'rust fast'");
    assert_eq!(row.get_value(0).unwrap().as_text().unwrap(), "2");
    let relevance = row.get_value(2).unwrap();
    assert_eq!(*relevance.as_integer().unwrap(), 2); // matched 2 words

    // Search for "elephant" — should return nothing
    let mut rows = conn
        .query(
            "SELECT d.id FROM docs d JOIN doc_words dw ON d.id = dw.doc_id WHERE dw.word IN ('elephant') GROUP BY d.id",
            (),
        )
        .await
        .unwrap();

    assert!(
        rows.next().await.unwrap().is_none(),
        "should find no results for 'elephant'"
    );

    println!("Inverted index search works correctly with turso");
}
