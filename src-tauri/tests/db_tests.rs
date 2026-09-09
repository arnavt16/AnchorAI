//! Persistence + invalidation tests using temporary on-disk SQLite
//! databases for persistence and race cases.

use anchor_lib::db::{open_vault, repo};

fn temp_conn() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sqlite");
    let state = open_vault(&path).expect("open vault");
    drop(state.pool); // we just needed migrations applied; open our own plain connection
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    (dir, conn)
}

#[test]
fn entry_persists_and_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sqlite");
    {
        let state = open_vault(&path).unwrap();
        let conn = state.pool.get().unwrap();
        repo::create_entry(
            &conn,
            repo::NewEntry {
                title: Some("Hello".into()),
                body: "First entry".into(),
                mood: None,
                tags: vec!["a".into()],
                memory_enabled: false,
                origin: "user".into(),
            },
        )
        .unwrap();
    }
    // Reopen fresh, simulating an app restart.
    let state = open_vault(&path).unwrap();
    let conn = state.pool.get().unwrap();
    let entries = repo::list_entries(&conn, None, 10).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title.as_deref(), Some("Hello"));
}

#[test]
fn empty_body_is_rejected() {
    let (_dir, conn) = temp_conn();
    let result = repo::create_entry(
        &conn,
        repo::NewEntry { title: None, body: "   ".into(), mood: None, tags: vec![], memory_enabled: false, origin: "user".into() },
    );
    assert!(result.is_err());
}

#[test]
fn deleting_entry_cascades_worry_outcomes_steps() {
    let (_dir, conn) = temp_conn();
    let entry = repo::create_entry(
        &conn,
        repo::NewEntry { title: None, body: "worried about the exam".into(), mood: None, tags: vec![], memory_enabled: false, origin: "user".into() },
    )
    .unwrap();
    let worry = repo::create_worry(&conn, &entry.id, "I might fail", None).unwrap();
    repo::create_outcome(&conn, &worry.id, "It went fine", Some("about_as_expected"), None).unwrap();
    repo::create_step(&conn, &entry.id, Some(worry.id.as_str()), "Studied an extra hour").unwrap();

    repo::delete_entry(&conn, &entry.id).unwrap();

    let worry_gone: i64 = conn.query_row("SELECT COUNT(*) FROM worries WHERE id = ?1", [&worry.id], |r| r.get(0)).unwrap();
    let outcomes_gone: i64 = conn.query_row("SELECT COUNT(*) FROM worry_outcomes", [], |r| r.get(0)).unwrap();
    let steps_gone: i64 = conn.query_row("SELECT COUNT(*) FROM small_steps", [], |r| r.get(0)).unwrap();
    assert_eq!(worry_gone, 0);
    assert_eq!(outcomes_gone, 0);
    assert_eq!(steps_gone, 0);
}

#[test]
fn only_one_worry_per_entry() {
    let (_dir, conn) = temp_conn();
    let entry = repo::create_entry(
        &conn,
        repo::NewEntry { title: None, body: "body".into(), mood: None, tags: vec![], memory_enabled: false, origin: "user".into() },
    )
    .unwrap();
    repo::create_worry(&conn, &entry.id, "first worry", None).unwrap();
    let second = repo::create_worry(&conn, &entry.id, "second worry", None);
    assert!(second.is_err(), "a second worry on the same entry must be rejected");
}

#[test]
fn disabling_memory_removes_chunks_and_cancels_jobs() {
    let (_dir, conn) = temp_conn();
    let entry = repo::create_entry(
        &conn,
        repo::NewEntry { title: None, body: "body".into(), mood: None, tags: vec![], memory_enabled: true, origin: "user".into() },
    )
    .unwrap();
    // Simulate an indexed chunk + a queued job as if indexing had run.
    conn.execute(
        "INSERT INTO retrieval_chunks (id, entry_id, source_kind, chunk_index, content, content_hash, embedding_blob, embedding_dimension, embedding_model_digest, embedding_space_version, source_version, aggregate_version, vault_generation, created_at)
         VALUES ('c1', ?1, 'entry', 0, 'body', 'hash', X'0000', 1, 'digest', 1, 1, 1, 1, '2026-01-01T00:00:00Z')",
        [&entry.id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO indexing_jobs (id, entry_id, requested_aggregate_version, requested_embedding_space_version, status, vault_generation, created_at, updated_at)
         VALUES ('j1', ?1, 1, 1, 'pending', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        [&entry.id],
    )
    .unwrap();

    repo::set_memory_eligibility(&conn, &entry.id, false).unwrap();

    let chunk_count: i64 = conn.query_row("SELECT COUNT(*) FROM retrieval_chunks WHERE entry_id = ?1", [&entry.id], |r| r.get(0)).unwrap();
    let job_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM indexing_jobs WHERE entry_id = ?1 AND status = 'pending'", [&entry.id], |r| r.get(0))
        .unwrap();
    assert_eq!(chunk_count, 0, "disabling memory must immediately remove derived chunks");
    assert_eq!(job_count, 0, "disabling memory must cancel queued indexing jobs");
}

#[test]
fn editing_worry_outcome_bumps_entry_aggregate_version() {
    let (_dir, conn) = temp_conn();
    let entry = repo::create_entry(
        &conn,
        repo::NewEntry { title: None, body: "body".into(), mood: None, tags: vec![], memory_enabled: true, origin: "user".into() },
    )
    .unwrap();
    let worry = repo::create_worry(&conn, &entry.id, "worry text", None).unwrap();
    let outcome = repo::create_outcome(&conn, &worry.id, "first version", None, None).unwrap();
    let before = repo::get_entry(&conn, &entry.id).unwrap().unwrap().aggregate_version;

    repo::update_outcome(&conn, &outcome.id, "corrected version", None, None).unwrap();

    let after = repo::get_entry(&conn, &entry.id).unwrap().unwrap().aggregate_version;
    assert!(after > before, "correcting an outcome must invalidate the entry's retrieval snapshot");
}

#[test]
fn step_must_belong_to_same_entry_as_its_worry() {
    let (_dir, conn) = temp_conn();
    let e1 = repo::create_entry(
        &conn,
        repo::NewEntry { title: None, body: "one".into(), mood: None, tags: vec![], memory_enabled: false, origin: "user".into() },
    )
    .unwrap();
    let e2 = repo::create_entry(
        &conn,
        repo::NewEntry { title: None, body: "two".into(), mood: None, tags: vec![], memory_enabled: false, origin: "user".into() },
    )
    .unwrap();
    let worry = repo::create_worry(&conn, &e1.id, "worry on entry one", None).unwrap();

    let result = repo::create_step(&conn, &e2.id, Some(worry.id.as_str()), "mismatched step");
    assert!(result.is_err(), "a step must not link to a worry from a different entry");
}
