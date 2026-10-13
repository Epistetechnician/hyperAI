//! Contract coverage for metadata-only execution crash recovery.

use hyperintel_core::{ProtocolId, Sha256Digest, UtcTimestamp};
use hyperintel_execution_journal::{
    ExecutionJournalError, ExecutionJournalStartDraftV1, ExecutionJournalTerminalDraftV1,
    ExecutionJournalTerminalStatusV1, ExecutionRecoveryStateV1, JournalWriteOutcomeV1,
    SqliteExecutionJournalV1, build_execution_journal_start_v1,
    build_execution_journal_terminal_v1,
};
use hyperintel_market::build_buyer_replay_state_v1;
use hyperintel_persistence::{
    AuthenticatedReplayConsumptionV1, DURABLE_REPLAY_STATE_SLICE_V1, SqliteReplayStoreV1,
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn contract_concurrent_exact_retries_restart_and_consumption_binding_are_fail_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("execution-journal.sqlite");
    let replay_path = directory.path().join("replay.sqlite");
    let replay_store = SqliteReplayStoreV1::connect_path(&replay_path, 2).await?;
    replay_store.migrate().await?;
    assert!(matches!(
        SqliteExecutionJournalV1::connect_path(&database_path, 0).await,
        Err(ExecutionJournalError::InvalidConnectionCount)
    ));
    let journal = SqliteExecutionJournalV1::connect_path(&database_path, 4).await?;
    journal.migrate().await?;

    let start = start([1; 32], "2026-07-21T10:00:00Z")?;
    assert_start_retries_and_conflict(&journal, &start).await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &replay_store)
            .await?,
        ExecutionRecoveryStateV1::StartedNotConsumed(value) if value == start
    ));

    let consumption = replay_consumption(start.admission_digest, 21)?;
    seed_replay_consumption(&replay_path, &consumption).await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &replay_store)
            .await?,
        ExecutionRecoveryStateV1::ConsumedWithoutTerminal { .. }
    ));
    let terminal = build_execution_journal_terminal_v1(
        &start,
        ExecutionJournalTerminalDraftV1 {
            consumption_digest: Some(consumption.consumption_digest),
            outcome_digest: Some(Sha256Digest::from_bytes([31; 32])),
            status: ExecutionJournalTerminalStatusV1::Completed,
            finished_at: timestamp("2026-07-21T10:00:03Z")?,
        },
    )?;
    assert_terminal_retries_and_conflict(&journal, &replay_store, &start, &terminal, &consumption)
        .await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &replay_store)
            .await?,
        ExecutionRecoveryStateV1::Terminal { start: value_start, terminal: value_terminal }
            if value_start == start && value_terminal == terminal
    ));
    let empty_replay_store = SqliteReplayStoreV1::in_memory().await?;
    empty_replay_store.migrate().await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &empty_replay_store)
            .await,
        Err(ExecutionJournalError::StoredBindingMismatch)
    ));
    let wrong_replay_path = directory.path().join("wrong-replay.sqlite");
    let wrong_replay_store = SqliteReplayStoreV1::connect_path(&wrong_replay_path, 1).await?;
    wrong_replay_store.migrate().await?;
    let wrong_consumption = replay_consumption(start.admission_digest, 22)?;
    seed_replay_consumption(&wrong_replay_path, &wrong_consumption).await?;
    assert!(matches!(
        journal
            .recovery_state(start.admission_digest, &wrong_replay_store)
            .await,
        Err(ExecutionJournalError::StoredBindingMismatch)
    ));

    let persisted = format!(
        "{}{}",
        serde_json::to_string(&start)?,
        serde_json::to_string(&terminal)?
    );
    for forbidden in [
        "messages",
        "content",
        "chunks",
        "signature_hex",
        "request_body",
        "response_body",
    ] {
        assert!(!persisted.contains(forbidden));
    }

    journal.checkpoint_and_close().await?;
    let reopened = SqliteExecutionJournalV1::connect_path(&database_path, 2).await?;
    reopened.migrate().await?;
    assert_eq!(reopened.start(start.admission_digest).await?, Some(start));
    assert_eq!(
        reopened.terminal(terminal.admission_digest).await?,
        Some(terminal)
    );
    reopened.checkpoint_and_close().await?;
    empty_replay_store.checkpoint_and_close().await?;
    wrong_replay_store.checkpoint_and_close().await?;
    replay_store.checkpoint_and_close().await?;
    Ok(())
}

#[tokio::test]
async fn contract_cross_row_and_regressing_time_tampering_fail_restart()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let database_path = directory.path().join("tamper-journal.sqlite");
    let replay_path = directory.path().join("tamper-replay.sqlite");
    let replay_store = SqliteReplayStoreV1::connect_path(&replay_path, 1).await?;
    replay_store.migrate().await?;
    let journal = SqliteExecutionJournalV1::connect_path(&database_path, 2).await?;
    journal.migrate().await?;
    let first = start([41; 32], "2026-07-21T11:00:00Z")?;
    let second = start([42; 32], "2026-07-21T11:10:00Z")?;
    journal.begin(&first).await?;
    journal.begin(&second).await?;
    let consumption = replay_consumption(second.admission_digest, 43)?;
    seed_replay_consumption(&replay_path, &consumption).await?;
    let terminal = build_execution_journal_terminal_v1(
        &second,
        ExecutionJournalTerminalDraftV1 {
            consumption_digest: Some(consumption.consumption_digest),
            outcome_digest: Some(Sha256Digest::from_bytes([44; 32])),
            status: ExecutionJournalTerminalStatusV1::Completed,
            finished_at: timestamp("2026-07-21T11:10:03Z")?,
        },
    )?;
    journal.finish(&terminal, &replay_store).await?;

    let mut cross_row = terminal.clone();
    cross_row.start_digest = first.start_digest;
    cross_row.terminal_digest = Sha256Digest::from_bytes([0; 32]);
    cross_row.terminal_digest = cross_row.digest()?;
    overwrite_terminal(&database_path, &cross_row).await?;
    assert!(matches!(
        journal
            .recovery_state(second.admission_digest, &replay_store)
            .await,
        Err(ExecutionJournalError::StoredBindingMismatch)
    ));

    let mut regressed = terminal;
    regressed.finished_at = timestamp("2026-07-21T11:09:59Z")?;
    regressed.terminal_digest = Sha256Digest::from_bytes([0; 32]);
    regressed.terminal_digest = regressed.digest()?;
    overwrite_terminal(&database_path, &regressed).await?;
    assert!(matches!(
        journal
            .recovery_state(second.admission_digest, &replay_store)
            .await,
        Err(ExecutionJournalError::StoredBindingMismatch)
    ));
    journal.checkpoint_and_close().await?;
    replay_store.checkpoint_and_close().await?;
    Ok(())
}

fn start(
    admission: [u8; 32],
    started_at: &str,
) -> Result<hyperintel_execution_journal::ExecutionJournalStartV1, Box<dyn std::error::Error>> {
    Ok(build_execution_journal_start_v1(
        ExecutionJournalStartDraftV1 {
            admission_digest: Sha256Digest::from_bytes(admission),
            buyer_id: "buyer:journal".parse()?,
            provider_id: "provider:journal".parse()?,
            prompt_digest: Sha256Digest::from_bytes([2; 32]),
            started_at: timestamp(started_at)?,
        },
    )?)
}

fn replay_consumption(
    admission_digest: Sha256Digest,
    marker: u8,
) -> Result<AuthenticatedReplayConsumptionV1, Box<dyn std::error::Error>> {
    let buyer_id: ProtocolId = "buyer:journal".parse()?;
    let before_state = build_buyer_replay_state_v1(buyer_id.clone(), 7)?;
    let after_state = build_buyer_replay_state_v1(buyer_id.clone(), 8)?;
    let mut value = AuthenticatedReplayConsumptionV1 {
        schema_version: "hyperintel.authenticated-replay-consumption.v1".to_owned(),
        state_slice: DURABLE_REPLAY_STATE_SLICE_V1.to_owned(),
        admission_digest,
        transition_digest: Sha256Digest::from_bytes([marker; 32]),
        buyer_id,
        consumed_nonce: 7,
        before_state,
        after_state,
        consumed_at: timestamp("2026-07-21T10:00:01Z")?,
        authority_granted: false,
        consumption_digest: Sha256Digest::from_bytes([0; 32]),
    };
    value.consumption_digest = value.digest()?;
    Ok(value)
}

fn assert_one_insert_and_one_exact_retry(
    left: JournalWriteOutcomeV1,
    right: JournalWriteOutcomeV1,
) {
    assert!(matches!(
        (left, right),
        (
            JournalWriteOutcomeV1::Inserted,
            JournalWriteOutcomeV1::ExistingIdentical
        ) | (
            JournalWriteOutcomeV1::ExistingIdentical,
            JournalWriteOutcomeV1::Inserted
        )
    ));
}

async fn assert_start_retries_and_conflict(
    journal: &SqliteExecutionJournalV1,
    start: &hyperintel_execution_journal::ExecutionJournalStartV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let (left, right) = tokio::join!(journal.begin(start), journal.begin(start));
    assert_one_insert_and_one_exact_retry(left?, right?);
    let mut conflicting = start.clone();
    conflicting.provider_id = "provider:conflict".parse()?;
    conflicting.start_digest = Sha256Digest::from_bytes([0; 32]);
    conflicting.start_digest = conflicting.digest()?;
    assert!(matches!(
        journal.begin(&conflicting).await,
        Err(ExecutionJournalError::IdentityConflict)
    ));
    Ok(())
}

async fn assert_terminal_retries_and_conflict(
    journal: &SqliteExecutionJournalV1,
    replay_store: &SqliteReplayStoreV1,
    start: &hyperintel_execution_journal::ExecutionJournalStartV1,
    terminal: &hyperintel_execution_journal::ExecutionJournalTerminalV1,
    consumption: &AuthenticatedReplayConsumptionV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let (left, right) = tokio::join!(
        journal.finish(terminal, replay_store),
        journal.finish(terminal, replay_store)
    );
    assert_one_insert_and_one_exact_retry(left?, right?);
    let conflicting = build_execution_journal_terminal_v1(
        start,
        ExecutionJournalTerminalDraftV1 {
            consumption_digest: Some(consumption.consumption_digest),
            outcome_digest: Some(Sha256Digest::from_bytes([32; 32])),
            status: ExecutionJournalTerminalStatusV1::Incomplete,
            finished_at: timestamp("2026-07-21T10:00:04Z")?,
        },
    )?;
    assert!(matches!(
        journal.finish(&conflicting, replay_store).await,
        Err(ExecutionJournalError::IdentityConflict)
    ));
    Ok(())
}

async fn overwrite_terminal(
    path: &std::path::Path,
    terminal: &hyperintel_execution_journal::ExecutionJournalTerminalV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::query(
        "UPDATE execution_terminals SET terminal_digest = ?, payload_json = ?, finished_at = ? \
         WHERE admission_digest = ?",
    )
    .bind(terminal.terminal_digest.to_hex())
    .bind(serde_json::to_string(terminal)?)
    .bind(terminal.finished_at.as_str())
    .bind(terminal.admission_digest.to_hex())
    .execute(&pool)
    .await?;
    pool.close().await;
    Ok(())
}

async fn seed_replay_consumption(
    path: &std::path::Path,
    consumption: &AuthenticatedReplayConsumptionV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::query(
        "INSERT INTO buyer_replay_states(\
         buyer_id, state_digest, next_nonce, payload_json, version, updated_at) \
         VALUES(?, ?, ?, ?, 0, ?)",
    )
    .bind(consumption.buyer_id.as_str())
    .bind(consumption.after_state.state_digest.to_hex())
    .bind(consumption.after_state.next_nonce.to_string())
    .bind(serde_json::to_string(&consumption.after_state)?)
    .bind(consumption.consumed_at.as_str())
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO authenticated_replay_consumptions(\
         admission_digest, transition_digest, buyer_id, consumed_nonce, \
         before_state_digest, after_state_digest, payload_json, consumed_at) \
         VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(consumption.admission_digest.to_hex())
    .bind(consumption.transition_digest.to_hex())
    .bind(consumption.buyer_id.as_str())
    .bind(consumption.consumed_nonce.to_string())
    .bind(consumption.before_state.state_digest.to_hex())
    .bind(consumption.after_state.state_digest.to_hex())
    .bind(serde_json::to_string(consumption)?)
    .bind(consumption.consumed_at.as_str())
    .execute(&pool)
    .await?;
    pool.close().await;
    Ok(())
}

fn timestamp(value: &str) -> Result<UtcTimestamp, Box<dyn std::error::Error>> {
    Ok(value.parse()?)
}
