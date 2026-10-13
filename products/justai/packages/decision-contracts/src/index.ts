/** Type-only projection of the Rust justai-contracts canonical owner. */
export type Disposition = "promote" | "unknown" | "quarantine";
export interface ProvenanceRef {
  source_repository: string;
  source_commit: string;
  source_path: string;
  source_digest: string;
  license_basis: string;
  custody_ref: string;
}
export interface GovernanceFields {
  state_slice: string;
  proposal_digest: string;
  claim_ceiling: string;
  provenance: ProvenanceRef[];
  disposition: string;
  evidence_refs: string[];
  content_digest: string;
}
export interface DecisionRecord extends GovernanceFields {
  schema: "justai.decision.v1";
  record_id: string;
  created_at: string;
  algorithmic_proposal: string;
  model_role: "proposal-only";
  reversible: boolean;
  confidence: number;
  alternative_refs: string[];
  requested_action: string;
}
export interface SerendipityRecord extends GovernanceFields {
  schema: "justai.serendipity.v1";
  record_id: string;
  created_at: string;
  sampling_method: string;
  candidate_refs: string[];
  preserved_alternatives: string[];
  candidate_digests: string[];
  budget: string;
  selected_candidate_ref: string | null;
}
export interface IntuitionReview extends GovernanceFields {
  schema: "justai.intuition_review.v1";
  record_id: string;
  created_at: string;
  hypothesis: string;
  confidence: number;
  observed_evidence: string[];
  rejected_alternatives: string[];
  recommended_action_or_restraint: string;
  reviewer_identity: string;
  later_outcome: string | null;
  intuition_added_value: boolean | null;
  human_disposition: Disposition | null;
}
export interface DissentRecord extends GovernanceFields {
  schema: "justai.dissent.v1";
  record_id: string;
  created_at: string;
  dissenting_reviewer: string;
  reason: string;
  alternatives_preserved: string[];
  resolution: string | null;
  resolution_evidence_refs: string[];
}
export interface PromotionDecision extends GovernanceFields {
  schema: "justai.promotion_decision.v1";
  record_id: string;
  created_at: string;
  decision: Disposition;
  reason: string;
  validator_identity: string;
  validator_independence: string;
  human_disposition_ref: string | null;
}
export interface UnknownRecord extends GovernanceFields {
  schema: "justai.unknown.v1";
  record_id: string;
  created_at: string;
  unknown_reason: string;
  missing_evidence: string[];
  next_review_condition: string;
  preserved_alternatives: string[];
  owner: string;
}
export interface EvidenceManifest extends GovernanceFields {
  schema: "justai.evidence_manifest.v1";
  record_id: string;
  created_at: string;
  artifact_refs: string[];
  source_commit_refs: string[];
  custody_root_ref: string;
  integrity_status: string;
  excluded_artifacts: string[];
}
export interface IntegrationResult extends GovernanceFields {
  schema: "justai.integration_result.v1";
  record_id: string;
  created_at: string;
  phase_policy: string;
  math: string;
  linguistics: string;
  evidence: string;
  observed_ids: string[];
  required_digest_refs: string[];
  nonclaims: string[];
  external_seal_ref: string | null;
}
export interface IntegrationHandoffSeal extends GovernanceFields {
  schema: "justai.integration_handoff_seal.v1";
  record_id: string;
  created_at: string;
  result_digest: string;
  sealed_payload_digest: string;
  external_reviewer_id: string;
  external_public_key: string;
  signature: string;
}
