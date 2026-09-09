//! Retrieval correctness: embedding-space isolation, memory-eligibility
//! filtering, and stale-aggregate-version exclusion. These insert
//! `retrieval_chunks` rows directly (bypassing Ollama) so the SQL/ranking
//! logic in `rag::retrieval` can be tested without a running model runtime.

use anchor_lib::db::{open_vault, repo};
use anchor_lib::rag::retrieval::retrieve;
use rusqlite::params;

fn setup() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sqlite");
    let state = open_vault(&path).unwrap(); // applies migrations
    drop(state.pool);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    (dir, conn)
}

fn insert_chunk(
    conn: &rusqlite::Connection,
    id: &str,
    entry_id: &str,
    embedding: &[f32],
    space_version: i64,
    vault_generation: i64,
    aggregate_version: i64,
) {
    let blob: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
    conn.execute(
        "INSERT INTO retrieval_chunks
            (id, entry_id, source_kind, chunk_index, content, content_hash, embedding_blob, embedding_dimension,
             embedding_model_digest, embedding_space_version, source_version, aggregate_version, vault_generation, created_at)
         VALUES (?1, ?2, 'entry', 0, 'some content', 'hash', ?3, ?4, 'digest', ?5, ?6, ?6, ?7, '2026-06-01T00:00:00Z')",
        params![id, entry_id, blob, embedding.len() as i64, space_version, aggregate_version, vault_generation],
    )
    .unwrap();
}

fn make_entry(conn: &rusqlite::Connection, memory_enabled: bool) -> String {
    let e = repo::create_entry(
        conn,
        repo::NewEntry { title: None, body: "body text".into(), mood: None, tags: vec![], memory_enabled, origin: "user".into() },
    )
    .unwrap();
    // create_entry sets indexing_status based on memory_enabled; force 'ready'
    // and pin aggregate_version = 1 so our manually-inserted chunk (aggregate_version=1) matches.
    conn.execute("UPDATE journal_entries SET indexing_status = 'ready' WHERE id = ?1", [&e.id]).unwrap();
    e.id
}

#[test]
fn excludes_entries_with_memory_disabled() {
    let (_dir, conn) = setup();
    let entry_id = make_entry(&conn, false); // memory disabled
    insert_chunk(&conn, "c1", &entry_id, &[1.0, 0.0, 0.0], 1, 1, 1);

    let results = retrieve(&conn, &[1.0, 0.0, 0.0], 1, 1, None).unwrap();
    assert!(results.is_empty(), "a memory-disabled entry's chunk must never be retrieved");
}

#[test]
fn excludes_different_embedding_space() {
    let (_dir, conn) = setup();
    let entry_id = make_entry(&conn, true);
    insert_chunk(&conn, "c1", &entry_id, &[1.0, 0.0, 0.0], 2, 1, 1); // space 2

    let results = retrieve(&conn, &[1.0, 0.0, 0.0], 1, 1, None).unwrap(); // querying space 1
    assert!(results.is_empty(), "a chunk from a different embedding space must never match a query in another space");
}

#[test]
fn excludes_different_vault_generation() {
    let (_dir, conn) = setup();
    let entry_id = make_entry(&conn, true);
    insert_chunk(&conn, "c1", &entry_id, &[1.0, 0.0, 0.0], 1, 5, 1); // vault_generation 5

    let results = retrieve(&conn, &[1.0, 0.0, 0.0], 1, 1, None).unwrap(); // current generation 1
    assert!(results.is_empty(), "a chunk from a stale vault generation (e.g. before a restore/erase) must never match");
}

#[test]
fn ranks_more_similar_vector_higher() {
    let (_dir, conn) = setup();
    let entry_a = make_entry(&conn, true);
    let entry_b = make_entry(&conn, true);
    insert_chunk(&conn, "close", &entry_a, &[1.0, 0.0, 0.0], 1, 1, 1);
    insert_chunk(&conn, "far", &entry_b, &[0.0, 1.0, 0.0], 1, 1, 1);

    let results = retrieve(&conn, &[0.9, 0.1, 0.0], 1, 1, None).unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].id, "close", "the more cosine-similar chunk should rank first");
}

#[test]
fn stale_aggregate_version_excluded_after_edit() {
    let (_dir, conn) = setup();
    let entry_id = make_entry(&conn, true);
    // Chunk was indexed at aggregate_version 1, but the entry has since
    // moved to aggregate_version 2 (e.g. the user edited it) without being
    // re-indexed yet.
    conn.execute("UPDATE journal_entries SET aggregate_version = 2 WHERE id = ?1", [&entry_id]).unwrap();
    insert_chunk(&conn, "stale", &entry_id, &[1.0, 0.0, 0.0], 1, 1, 1);

    let results = retrieve(&conn, &[1.0, 0.0, 0.0], 1, 1, None).unwrap();
    assert!(results.is_empty(), "a chunk whose aggregate_version doesn't match the entry's current version must be excluded as stale");
}
