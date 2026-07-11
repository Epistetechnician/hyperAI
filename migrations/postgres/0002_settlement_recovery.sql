ALTER TABLE settlement_intents ADD COLUMN request_digest TEXT;

UPDATE settlement_intents
SET request_digest = payload_digest
WHERE request_digest IS NULL;

ALTER TABLE settlement_intents ALTER COLUMN request_digest SET NOT NULL;

CREATE UNIQUE INDEX settlement_intents_request_digest_unique
  ON settlement_intents(request_digest);

CREATE UNIQUE INDEX settlement_receipts_backend_receipt_unique
  ON settlement_receipts(backend_receipt_id);

CREATE TABLE settlement_reconciliations (
  reconciliation_digest TEXT PRIMARY KEY,
  intent_id TEXT NOT NULL REFERENCES settlement_intents(intent_id) ON DELETE RESTRICT,
  receipt_digest TEXT,
  external_status_digest TEXT NOT NULL,
  status TEXT NOT NULL,
  lifecycle TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  checked_at TEXT NOT NULL
);

CREATE INDEX settlement_reconciliations_by_intent
  ON settlement_reconciliations(intent_id, checked_at, reconciliation_digest);

CREATE TRIGGER settlement_intents_no_update
  BEFORE UPDATE OR DELETE ON settlement_intents
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
CREATE TRIGGER settlement_receipts_no_update
  BEFORE UPDATE OR DELETE ON settlement_receipts
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
CREATE TRIGGER settlement_reconciliations_no_update
  BEFORE UPDATE OR DELETE ON settlement_reconciliations
  FOR EACH ROW EXECUTE FUNCTION hsai_reject_append_only_mutation();
