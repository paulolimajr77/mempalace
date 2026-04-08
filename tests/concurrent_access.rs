/// Verify that multiple processes can open the same database file concurrently
/// when `LIMBO_DISABLE_FILE_LOCK` is set. Regression test for
/// <https://github.com/bunkerlab-net/mempalace/issues/9>
#[allow(unsafe_code)]
#[tokio::test]
async fn two_connections_to_same_file() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let db_path = dir.path().join("palace.db");
    let path_str = db_path.to_str().expect("non-utf8 path");

    // Disable turso's exclusive file lock (same as open_db does at runtime).
    unsafe {
        std::env::set_var("LIMBO_DISABLE_FILE_LOCK", "1");
    }

    // First "process" opens the database and holds the connection.
    let db1 = turso::Builder::new_local(path_str)
        .build()
        .await
        .expect("first open failed");
    let conn1 = db1.connect().expect("first connect failed");
    let mut rows = conn1
        .query("PRAGMA journal_mode=WAL", ())
        .await
        .expect("WAL pragma failed");
    while rows.next().await.expect("row error").is_some() {}

    conn1
        .execute(
            "CREATE TABLE IF NOT EXISTS test_table (id TEXT PRIMARY KEY, val TEXT)",
            (),
        )
        .await
        .expect("create table failed");

    // Second "process" opens the same file — this would fail without the fix.
    let db2 = turso::Builder::new_local(path_str)
        .build()
        .await
        .expect("second open failed — file lock not disabled?");
    let conn2 = db2.connect().expect("second connect failed");

    conn2
        .execute(
            "INSERT INTO test_table (id, val) VALUES ('k1', 'hello')",
            (),
        )
        .await
        .expect("insert from second connection failed");

    // Verify the first connection can read the write.
    let mut read_rows = conn1
        .query("SELECT val FROM test_table WHERE id = 'k1'", ())
        .await
        .expect("select failed");
    let row = read_rows
        .next()
        .await
        .expect("row error")
        .expect("expected one row");
    let val: String = row.get(0).expect("get column failed");
    assert_eq!(val, "hello");
}

/// Verify that a second open of the same database file fails with a locking
/// error when `LIMBO_DISABLE_FILE_LOCK` is not set. This confirms that the
/// positive test above is actually testing something meaningful.
///
/// POSIX `fcntl` locks are per-process, so opening the file twice from the
/// same process does not produce a conflict. A subprocess is used to hold the
/// lock while the parent attempts a second open.
///
/// Child-process protocol (invoked via `_MEMPALACE_TEST_LOCK_PATH`):
///   exit 0 — open was blocked by the lock (expected)
///   exit 1 — open succeeded despite the lock (unexpected)
#[allow(unsafe_code)]
#[tokio::test]
async fn second_open_fails_without_lock_disabled() {
    // --- Child-process path -------------------------------------------
    // When spawned as the lock-probe, try to open the file and report
    // whether the lock blocked us. Exit immediately so the test harness
    // does not run further tests in this subprocess.
    if let Ok(path) = std::env::var("_MEMPALACE_TEST_LOCK_PATH") {
        let blocked = turso::Builder::new_local(&path).build().await.is_err();
        std::process::exit(i32::from(!blocked));
    }

    // --- Parent-process path ------------------------------------------
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let db_path = dir.path().join("palace.db");
    let path_str = db_path.to_str().expect("non-utf8 path");

    // Ensure the escape-hatch env var is absent so turso acquires its
    // default exclusive file lock.
    //
    // SAFETY: remove_var is unsafe due to thread-safety, but this runs
    // before any other threads access the environment in this process.
    unsafe {
        std::env::remove_var("LIMBO_DISABLE_FILE_LOCK");
    }

    // Open the database; this acquires an exclusive fcntl lock on the file.
    let _db1 = turso::Builder::new_local(path_str)
        .build()
        .await
        .expect("first open failed");

    // Spawn a child process that tries to open the same file without the
    // env-var escape hatch. Exit code 0 means the lock correctly blocked it.
    let current_exe = std::env::current_exe().expect("failed to get current exe");
    let status = std::process::Command::new(current_exe)
        .env("_MEMPALACE_TEST_LOCK_PATH", path_str)
        .env_remove("LIMBO_DISABLE_FILE_LOCK")
        // Filter to this test so the child harness does not run other tests
        // before hitting the early-exit branch above.
        .args(["second_open_fails_without_lock_disabled"])
        .status()
        .expect("failed to spawn child process");

    assert!(
        status.success(),
        "second open should have been blocked by the exclusive file lock"
    );
}
