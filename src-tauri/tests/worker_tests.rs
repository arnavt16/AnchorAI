//! Indexing queue behavior that doesn't need a model runtime: retry
//! backoff, re-queueing after outcome deletion, and the reflection guard.

use anchor_lib::db::{open_vault, repo};
use anchor_lib::indexing::worker::{enqueue_if_eligible, next_due_job, IndexingControl};

fn setup() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sqlite");
    let state = open_vault(&path).unwrap();
    drop(state.pool);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    (dir, conn)
}

fn make_entry(conn: &rusqlite::Connection) -> String {
    repo::create_entry(
        conn,
        repo::NewEntry { title: None, body: "body".into(), mood: None, tags: vec![], memory_enabled: true, origin: "user".into() },
    )
    .unwrap()
    .id
}

fn insert_job(conn: &rusqlite::Connection, id: &str, entry_id: &str, next_attempt_at: Option<&str>) {
    conn.execute(
        "INSERT INTO indexing_jobs (id, entry_id, requested_aggregate_version, requested_embedding_space_version, status, next_attempt_at, vault_generation, created_at, updated_at)
         VALUES (?1, ?2, 1, 1, 'pending', ?3, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        rusqlite::params![id, entry_id, next_attempt_at],
    )
    .unwrap();
}

#[test]
fn job_in_backoff_is_not_picked_up() {
    let (_dir, conn) = setup();
    let entry = make_entry(&conn);
    insert_job(&conn, "j1", &entry, Some("2999-01-01T00:00:00.123456789Z"));
    assert!(next_due_job(&conn).unwrap().is_none(), "a job whose next_attempt_at is in the future must wait");
}

#[test]
fn job_past_its_backoff_is_picked_up() {
    let (_dir, conn) = setup();
    let entry = make_entry(&conn);
    insert_job(&conn, "j1", &entry, Some("2001-01-01T00:00:00.5Z"));
    assert_eq!(next_due_job(&conn).unwrap().map(|j| j.id).as_deref(), Some("j1"));
}

#[test]
fn backoff_job_does_not_block_a_due_one() {
    let (_dir, conn) = setup();
    let a = make_entry(&conn);
    let b = make_entry(&conn);
    insert_job(&conn, "waiting", &a, Some("2999-01-01T00:00:00Z"));
    insert_job(&conn, "fresh", &b, None);
    assert_eq!(next_due_job(&conn).unwrap().map(|j| j.id).as_deref(), Some("fresh"));
}

#[test]
fn deleting_an_outcome_can_requeue_the_entry() {
    let (_dir, conn) = setup();
    conn.execute("UPDATE app_settings SET local_ai_enabled = 1 WHERE id = 1", []).unwrap();
    let entry = make_entry(&conn);
    let worry = repo::create_worry(&conn, &entry, "worry", None).unwrap();
    let outcome = repo::create_outcome(&conn, &worry.id, "outcome", None, None).unwrap();
    conn.execute("DELETE FROM indexing_jobs", []).unwrap();

    repo::delete_outcome(&conn, &outcome.id).unwrap();
    let after = repo::get_entry(&conn, &entry).unwrap().unwrap();
    assert_eq!(after.indexing_status, "pending");

    // What the delete_outcome command now does after the repo call.
    enqueue_if_eligible(&conn, &entry).unwrap();
    let job = next_due_job(&conn).unwrap().expect("a job for the post-delete snapshot");
    assert_eq!(job.entry_id, entry);
    assert_eq!(job.requested_aggregate_version, after.aggregate_version);
}

#[test]
fn overlapping_reflections_keep_indexing_paused_until_both_finish() {
    let control = IndexingControl::default();
    assert!(!control.should_skip());
    let first = control.begin_reflection();
    let second = control.begin_reflection();
    drop(first);
    assert!(control.should_skip(), "the second reflection is still running");
    drop(second);
    assert!(!control.should_skip());
}

#[test]
fn manual_pause_survives_a_reflection_ending() {
    let control = IndexingControl::default();
    control.paused.store(true, std::sync::atomic::Ordering::SeqCst);
    drop(control.begin_reflection());
    assert!(control.should_skip(), "finishing a reflection must not undo a manual pause");
}
