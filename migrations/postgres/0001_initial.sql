CREATE TABLE challenge_descriptors (
  challenge_id TEXT PRIMARY KEY,
  canonical_spec_digest TEXT NOT NULL UNIQUE,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE corpus_descriptors (
  corpus_id TEXT PRIMARY KEY,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE corpus_snapshots (
  snapshot_digest TEXT PRIMARY KEY,
  corpus_id TEXT NOT NULL REFERENCES corpus_descriptors(corpus_id) ON DELETE RESTRICT,
  resolved_commit TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  received_at TEXT NOT NULL,
  UNIQUE(corpus_id, resolved_commit)
);

CREATE TABLE challenge_packages (
  package_digest TEXT PRIMARY KEY,
  snapshot_digest TEXT NOT NULL REFERENCES corpus_snapshots(snapshot_digest) ON DELETE RESTRICT,
  challenge_ref TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  UNIQUE(snapshot_digest, challenge_ref)
);

CREATE TABLE runtime_manifests (
  manifest_digest TEXT PRIMARY KEY,
  package_digest TEXT NOT NULL REFERENCES challenge_packages(package_digest) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL
);

CREATE TABLE verifier_manifests (
  manifest_digest TEXT PRIMARY KEY,
  package_digest TEXT NOT NULL REFERENCES challenge_packages(package_digest) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL
);

CREATE TABLE solver_view_manifests (
  manifest_digest TEXT PRIMARY KEY,
  package_digest TEXT NOT NULL REFERENCES challenge_packages(package_digest) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL
);

CREATE TABLE rights_provenance_assessments (
  assessment_digest TEXT PRIMARY KEY,
  package_digest TEXT NOT NULL REFERENCES challenge_packages(package_digest) ON DELETE RESTRICT,
  rights_status TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  assessed_at TEXT NOT NULL
);

CREATE TABLE adapter_capability_snapshots (
  capability_digest TEXT PRIMARY KEY,
  adapter_id TEXT NOT NULL,
  adapter_version TEXT NOT NULL,
  settlement_readiness TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  verified_at TEXT NOT NULL,
  UNIQUE(adapter_id, adapter_version, verified_at)
);

CREATE TABLE agent_harness_descriptors (
  harness_id TEXT PRIMARY KEY,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE agent_run_plans (
  plan_id TEXT PRIMARY KEY,
  harness_id TEXT NOT NULL REFERENCES agent_harness_descriptors(harness_id) ON DELETE RESTRICT,
  package_digest TEXT NOT NULL REFERENCES challenge_packages(package_digest) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE agent_runs (
  run_id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL REFERENCES agent_run_plans(plan_id) ON DELETE RESTRICT,
  status TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT
);

CREATE TABLE agent_trace_bundles (
  trace_digest TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES agent_runs(run_id) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL
);

CREATE TABLE claim_specs (
  claim_id TEXT PRIMARY KEY,
  claim_spec_digest TEXT NOT NULL UNIQUE,
  challenge_descriptor_digest TEXT NOT NULL,
  canonical_json BYTEA NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  frozen_at TEXT NOT NULL,
  version INTEGER NOT NULL DEFAULT 1 CHECK(version = 1)
);

CREATE TABLE claim_events (
  event_id TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  aggregate_version INTEGER NOT NULL,
  previous_version INTEGER NOT NULL,
  event_type TEXT NOT NULL,
  schema_version TEXT NOT NULL,
  actor_type TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  request_id TEXT NOT NULL,
  trace_id TEXT NOT NULL,
  payload_digest TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  received_at TEXT NOT NULL,
  source_time TEXT,
  build_version TEXT NOT NULL,
  previous_event_digest TEXT,
  event_digest TEXT NOT NULL UNIQUE,
  UNIQUE(claim_id, aggregate_version)
);

CREATE TABLE observation_snapshots (
  snapshot_id TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  cursor_end TEXT NOT NULL,
  payload_digest TEXT NOT NULL UNIQUE,
  payload_json TEXT NOT NULL,
  received_at TEXT NOT NULL
);

CREATE TABLE observation_cursors (
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  source_adapter_id TEXT NOT NULL,
  cursor_value TEXT NOT NULL,
  version INTEGER NOT NULL,
  updated_at TEXT NOT NULL,
  PRIMARY KEY(claim_id, source_adapter_id),
  UNIQUE(claim_id, source_adapter_id, version)
);

CREATE TABLE candidates (
  candidate_id TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  source_namespace TEXT NOT NULL,
  source_candidate_ref TEXT NOT NULL,
  source_revision TEXT NOT NULL,
  artifact_digest TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  received_at TEXT NOT NULL,
  UNIQUE(source_namespace, source_candidate_ref, source_revision, artifact_digest)
);

CREATE TABLE candidate_artifacts (
  artifact_digest TEXT PRIMARY KEY,
  candidate_id TEXT NOT NULL REFERENCES candidates(candidate_id) ON DELETE RESTRICT,
  byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
  media_type TEXT NOT NULL,
  storage_locator TEXT NOT NULL
);

CREATE TABLE verification_runs (
  verification_run_id TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  candidate_id TEXT NOT NULL REFERENCES candidates(candidate_id) ON DELETE RESTRICT,
  policy_attempt INTEGER NOT NULL,
  normalized_outcome TEXT,
  execution_manifest_digest TEXT,
  payload_json TEXT NOT NULL,
  UNIQUE(claim_id, candidate_id, policy_attempt)
);

CREATE TABLE run_attempts (
  attempt_id TEXT PRIMARY KEY,
  verification_run_id TEXT NOT NULL REFERENCES verification_runs(verification_run_id) ON DELETE RESTRICT,
  attempt_number INTEGER NOT NULL,
  status TEXT NOT NULL,
  lease_owner TEXT,
  lease_expires_at TEXT,
  heartbeat_at TEXT,
  payload_json TEXT NOT NULL,
  UNIQUE(verification_run_id, attempt_number)
);

CREATE TABLE evidence_manifests (
  evidence_digest TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE evidence_blobs (
  blob_digest TEXT PRIMARY KEY,
  byte_length INTEGER NOT NULL CHECK(byte_length >= 0),
  media_type TEXT NOT NULL,
  classification TEXT NOT NULL,
  storage_locator TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE attestations (
  attestation_digest TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  evidence_digest TEXT NOT NULL REFERENCES evidence_manifests(evidence_digest) ON DELETE RESTRICT,
  signer_id TEXT NOT NULL,
  key_id TEXT NOT NULL,
  logical_sequence INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  issued_at TEXT NOT NULL,
  UNIQUE(claim_id, signer_id, logical_sequence)
);

CREATE TABLE resolution_records (
  resolution_digest TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  truth_value TEXT NOT NULL,
  finality TEXT NOT NULL,
  evidence_digest TEXT NOT NULL REFERENCES evidence_manifests(evidence_digest) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX one_final_resolution_per_claim
  ON resolution_records(claim_id) WHERE finality = 'FINAL';

CREATE TABLE disputes (
  dispute_id TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  provisional_resolution_digest TEXT NOT NULL REFERENCES resolution_records(resolution_digest) ON DELETE RESTRICT,
  status TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  opened_at TEXT NOT NULL,
  closes_at TEXT NOT NULL
);

CREATE TABLE market_bindings (
  binding_digest TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  backend TEXT NOT NULL,
  network TEXT NOT NULL,
  external_market_id TEXT NOT NULL,
  true_side_id TEXT NOT NULL,
  false_side_id TEXT NOT NULL,
  metadata_digest TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(backend, network, external_market_id, true_side_id),
  UNIQUE(backend, network, external_market_id, false_side_id),
  CHECK(true_side_id <> false_side_id)
);

CREATE TABLE settlement_intents (
  intent_id TEXT PRIMARY KEY,
  binding_digest TEXT NOT NULL REFERENCES market_bindings(binding_digest) ON DELETE RESTRICT,
  final_resolution_digest TEXT NOT NULL REFERENCES resolution_records(resolution_digest) ON DELETE RESTRICT,
  idempotency_key TEXT NOT NULL UNIQUE,
  payload_digest TEXT NOT NULL UNIQUE,
  payload_json TEXT NOT NULL,
  lifecycle TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE settlement_attempts (
  attempt_id TEXT PRIMARY KEY,
  intent_id TEXT NOT NULL REFERENCES settlement_intents(intent_id) ON DELETE RESTRICT,
  attempt_number INTEGER NOT NULL,
  request_digest TEXT NOT NULL,
  response_digest TEXT,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  UNIQUE(intent_id, attempt_number)
);

CREATE TABLE settlement_receipts (
  receipt_id TEXT PRIMARY KEY,
  intent_id TEXT NOT NULL UNIQUE REFERENCES settlement_intents(intent_id) ON DELETE RESTRICT,
  backend_receipt_id TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  accepted_at TEXT NOT NULL,
  UNIQUE(intent_id, backend_receipt_id)
);

CREATE TABLE idempotency_keys (
  scope TEXT NOT NULL,
  idempotency_key TEXT NOT NULL,
  request_digest TEXT NOT NULL,
  response_digest TEXT,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  completed_at TEXT,
  PRIMARY KEY(scope, idempotency_key)
);

CREATE TABLE outbox_events (
  outbox_id TEXT PRIMARY KEY,
  aggregate_type TEXT NOT NULL,
  aggregate_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  payload_digest TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  published_at TEXT,
  attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count >= 0),
  next_attempt_at TEXT,
  last_error_code TEXT
);

CREATE INDEX outbox_delivery_queue
  ON outbox_events(published_at, next_attempt_at, created_at);

CREATE TABLE audit_events (
  audit_id TEXT PRIMARY KEY,
  event_type TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  request_id TEXT NOT NULL,
  declared_state_slices_json TEXT NOT NULL,
  aggregate_type TEXT NOT NULL,
  aggregate_id TEXT NOT NULL,
  payload_digest TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  received_at TEXT NOT NULL,
  previous_audit_digest TEXT,
  audit_digest TEXT NOT NULL UNIQUE
);

CREATE TABLE operator_pauses (
  pause_id TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  active BOOLEAN NOT NULL,
  reason TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  released_at TEXT
);

CREATE UNIQUE INDEX one_active_pause_per_claim
  ON operator_pauses(claim_id) WHERE active = TRUE;

CREATE TABLE durable_jobs (
  job_id TEXT PRIMARY KEY,
  job_type TEXT NOT NULL,
  idempotency_key TEXT NOT NULL UNIQUE,
  payload_digest TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  status TEXT NOT NULL,
  attempt_count INTEGER NOT NULL DEFAULT 0 CHECK(attempt_count >= 0),
  lease_owner TEXT,
  lease_expires_at TEXT,
  heartbeat_at TEXT,
  next_attempt_at TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT,
  last_error_code TEXT
);

CREATE INDEX durable_job_queue
  ON durable_jobs(status, next_attempt_at, lease_expires_at, created_at);

CREATE FUNCTION hsai_reject_append_only_mutation() RETURNS trigger AS $$
BEGIN
  RAISE EXCEPTION '% is append-only', TG_TABLE_NAME;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER claim_specs_no_update
  BEFORE UPDATE OR DELETE ON claim_specs
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
CREATE TRIGGER claim_events_no_update
  BEFORE UPDATE OR DELETE ON claim_events
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
CREATE TRIGGER evidence_manifests_no_update
  BEFORE UPDATE OR DELETE ON evidence_manifests
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
CREATE TRIGGER evidence_blobs_no_update
  BEFORE UPDATE OR DELETE ON evidence_blobs
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
CREATE TRIGGER attestations_no_update
  BEFORE UPDATE OR DELETE ON attestations
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
CREATE TRIGGER resolution_records_no_update
  BEFORE UPDATE OR DELETE ON resolution_records
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
CREATE TRIGGER audit_events_no_update
  BEFORE UPDATE OR DELETE ON audit_events
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
