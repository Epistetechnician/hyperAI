CREATE TABLE dispute_decisions (
  decision_digest TEXT PRIMARY KEY,
  dispute_id TEXT NOT NULL REFERENCES disputes(dispute_id) ON DELETE RESTRICT,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  terminal_status TEXT NOT NULL CHECK(terminal_status IN (
    'REJECTED', 'ACCEPTED_SUPERSEDED_BEFORE_FINALITY', 'UNRESOLVED_ESCALATED'
  )),
  successor_resolution_digest TEXT REFERENCES resolution_records(resolution_digest) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL,
  decided_at TEXT NOT NULL,
  UNIQUE(dispute_id)
);

CREATE TABLE resolution_supersessions (
  supersession_digest TEXT PRIMARY KEY,
  claim_id TEXT NOT NULL REFERENCES claim_specs(claim_id) ON DELETE RESTRICT,
  predecessor_resolution_digest TEXT NOT NULL REFERENCES resolution_records(resolution_digest) ON DELETE RESTRICT,
  successor_resolution_digest TEXT NOT NULL REFERENCES resolution_records(resolution_digest) ON DELETE RESTRICT,
  dispute_decision_digest TEXT NOT NULL REFERENCES dispute_decisions(decision_digest) ON DELETE RESTRICT,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE(predecessor_resolution_digest),
  UNIQUE(successor_resolution_digest),
  CHECK(predecessor_resolution_digest <> successor_resolution_digest)
);

CREATE TRIGGER disputes_no_update BEFORE UPDATE ON disputes
BEGIN SELECT RAISE(ABORT, 'disputes is append-only'); END;
CREATE TRIGGER disputes_no_delete BEFORE DELETE ON disputes
BEGIN SELECT RAISE(ABORT, 'disputes is append-only'); END;
CREATE TRIGGER dispute_decisions_no_update BEFORE UPDATE ON dispute_decisions
BEGIN SELECT RAISE(ABORT, 'dispute_decisions is append-only'); END;
CREATE TRIGGER dispute_decisions_no_delete BEFORE DELETE ON dispute_decisions
BEGIN SELECT RAISE(ABORT, 'dispute_decisions is append-only'); END;
CREATE TRIGGER resolution_supersessions_no_update BEFORE UPDATE ON resolution_supersessions
BEGIN SELECT RAISE(ABORT, 'resolution_supersessions is append-only'); END;
CREATE TRIGGER resolution_supersessions_no_delete BEFORE DELETE ON resolution_supersessions
BEGIN SELECT RAISE(ABORT, 'resolution_supersessions is append-only'); END;
