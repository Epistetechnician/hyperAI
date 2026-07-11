//! Regenerate immutable protocol schemas and local golden claim fixtures.

use std::{error::Error, fs, path::Path};

use outcomes_domain::{
    AdapterCapabilityRecordV1, AdapterHealthV1, AdmissionDecisionV1, AgentHarnessDescriptorV1,
    AgentRunPlanV1, AgentRunResultV1, AgentTraceBundleV1, CandidateEnvelopeV1,
    ChallengeDescriptorV1, ChallengePackageV1, ClaimSpecV1, ContentBlobRefV1, CorpusDescriptorV1,
    CorpusSnapshotV1, DisputeDecisionRecordV1, DisputeRecordV1, ExecutionManifestV1,
    MarketBindingV1, ObservationSnapshotV1, OutcomeEvidenceBundleV1, ReconciliationResultV1,
    ResolutionAttestationV1, ResolutionRecordV1, ResolutionSupersessionRecordV1,
    RightsAndProvenanceAssessmentV1, SettlementHandoffV1, SolverViewManifestV1,
    SubmissionReceiptV1, UnsignedSettlementIntentV1, canonical_json,
    fixtures::local_demo_claim_draft,
};
use schemars::schema_for;
use serde::Serialize;

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schemas = root.join("schemas");
    let fixtures = root.join("fixtures/claims");
    fs::create_dir_all(&schemas)?;
    fs::create_dir_all(&fixtures)?;

    macro_rules! write_schema {
        ($type:ty, $name:literal) => {
            write_pretty(&schemas.join($name), &schema_for!($type))?;
        };
    }
    write_schema!(ChallengeDescriptorV1, "challenge-descriptor-v1.schema.json");
    write_schema!(ClaimSpecV1, "claim-spec-v1.schema.json");
    write_schema!(
        AdapterCapabilityRecordV1,
        "adapter-capability-record-v1.schema.json"
    );
    write_schema!(AdapterHealthV1, "adapter-health-v1.schema.json");
    write_schema!(CorpusDescriptorV1, "corpus-descriptor-v1.schema.json");
    write_schema!(CorpusSnapshotV1, "corpus-snapshot-v1.schema.json");
    write_schema!(ChallengePackageV1, "challenge-package-v1.schema.json");
    write_schema!(SolverViewManifestV1, "solver-view-manifest-v1.schema.json");
    write_schema!(
        RightsAndProvenanceAssessmentV1,
        "rights-and-provenance-assessment-v1.schema.json"
    );
    write_schema!(ObservationSnapshotV1, "observation-snapshot-v1.schema.json");
    write_schema!(CandidateEnvelopeV1, "candidate-envelope-v1.schema.json");
    write_schema!(
        AgentHarnessDescriptorV1,
        "agent-harness-descriptor-v1.schema.json"
    );
    write_schema!(AgentRunPlanV1, "agent-run-plan-v1.schema.json");
    write_schema!(AgentRunResultV1, "agent-run-result-v1.schema.json");
    write_schema!(AgentTraceBundleV1, "agent-trace-bundle-v1.schema.json");
    write_schema!(AdmissionDecisionV1, "admission-decision-v1.schema.json");
    write_schema!(ExecutionManifestV1, "execution-manifest-v1.schema.json");
    write_schema!(ContentBlobRefV1, "content-blob-ref-v1.schema.json");
    write_schema!(
        OutcomeEvidenceBundleV1,
        "outcome-evidence-bundle-v1.schema.json"
    );
    write_schema!(
        ResolutionAttestationV1,
        "resolution-attestation-v1.schema.json"
    );
    write_schema!(ResolutionRecordV1, "resolution-record-v1.schema.json");
    write_schema!(DisputeRecordV1, "dispute-record-v1.schema.json");
    write_schema!(
        DisputeDecisionRecordV1,
        "dispute-decision-record-v1.schema.json"
    );
    write_schema!(
        ResolutionSupersessionRecordV1,
        "resolution-supersession-record-v1.schema.json"
    );
    write_schema!(MarketBindingV1, "market-binding-v1.schema.json");
    write_schema!(
        UnsignedSettlementIntentV1,
        "unsigned-settlement-intent-v1.schema.json"
    );
    write_schema!(SettlementHandoffV1, "settlement-handoff-v1.schema.json");
    write_schema!(SubmissionReceiptV1, "submission-receipt-v1.schema.json");
    write_schema!(
        ReconciliationResultV1,
        "reconciliation-result-v1.schema.json"
    );

    let claim = local_demo_claim_draft()?.freeze()?;
    write_pretty(&fixtures.join("local-demo-claim-spec-v1.json"), &claim)?;
    fs::write(
        fixtures.join("local-demo-claim-spec-v1.canonical.json"),
        canonical_json(&claim)?,
    )?;
    let identity = serde_json::json!({
        "schema_version": "hsai.claim-identity-fixture.v1",
        "claim_id": claim.claim_id,
        "claim_spec_digest": claim.claim_spec_digest()?,
    });
    write_pretty(
        &fixtures.join("local-demo-claim-identity-v1.json"),
        &identity,
    )?;
    Ok(())
}

fn write_pretty(path: &Path, value: &impl Serialize) -> Result<(), Box<dyn Error>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}
