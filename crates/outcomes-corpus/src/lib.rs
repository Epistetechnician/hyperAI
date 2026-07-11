//! Challenge-corpus ports, normalized packages, and solver-view boundaries.

use std::collections::BTreeMap;

use outcomes_domain::{
    AdapterHealthStatus, AdapterHealthV1, ChallengeDescriptorV1, ChallengePackageV1,
    CorpusDescriptorV1, CorpusSnapshotV1, PackageArtifactV1, PackageVisibility,
    RightsAndProvenanceAssessmentV1, RightsStatus, SettlementReadiness, Sha256Digest,
    SolverViewManifestV1, UtcTimestamp, domain_separated_digest,
};
use serde::Deserialize;
use thiserror::Error;

/// Failures returned by corpus adapters and solver-view validation.
#[derive(Debug, Error)]
pub enum CorpusError {
    /// Requested immutable snapshot does not exist in this adapter.
    #[error("unknown corpus snapshot {0}")]
    UnknownSnapshot(String),
    /// Requested challenge reference does not exist in the snapshot.
    #[error("unknown challenge reference {0}")]
    UnknownChallenge(String),
    /// A mutable discovery reference was used where a frozen commit was required.
    #[error("ref {0} is mutable or unresolved and cannot be frozen")]
    MutableRef(String),
    /// Solver view exposed a runtime, verifier, secret, or contamination artifact.
    #[error("solver view exposes prohibited artifact {0}")]
    SecretExposure(Sha256Digest),
    /// Solver view references an artifact absent from the package.
    #[error("solver view references unknown artifact {0}")]
    UnknownSolverArtifact(Sha256Digest),
    /// Domain payload could not be canonicalized.
    #[error("corpus payload is not canonicalizable: {0}")]
    Canonicalization(#[from] outcomes_domain::CanonicalizationError),
    /// Bundled profile JSON was invalid.
    #[error("corpus profile catalog JSON is invalid: {0}")]
    ProfileJson(#[from] serde_json::Error),
    /// Bundled profile violates immutable pin or uniqueness requirements.
    #[error("invalid corpus profile {profile_id}: {reason}")]
    InvalidProfile {
        /// Invalid profile identifier.
        profile_id: String,
        /// Exact invariant failure.
        reason: String,
    },
}

/// Machine-readable catalog of reviewed generic-Git manifest profiles.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusProfileCatalog {
    /// Catalog schema version.
    pub schema_version: String,
    /// Dated verification snapshot.
    pub verified_at: UtcTimestamp,
    /// Seven required pinned corpus paths.
    pub profiles: Vec<CorpusProfile>,
}

/// One data-only profile over the shared generic-Git corpus engine.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusProfile {
    /// Stable profile identifier.
    pub profile_id: String,
    /// Shared adapter engine identifier.
    pub engine: String,
    /// Allowlisted public upstream repository.
    pub repository: String,
    /// Immutable 40-hex Git commit.
    pub commit: String,
    /// Reviewed inventory roots.
    pub roots: Vec<String>,
    /// Solver-view exclusion patterns.
    pub excluded_patterns: Vec<String>,
    /// Human-readable exact license boundary.
    pub license_status: String,
    /// Typed rights ceiling.
    pub rights_status: RightsStatus,
    /// Typed settlement readiness ceiling.
    pub readiness: SettlementReadiness,
}

/// Load and validate the seven required pinned corpus profiles.
///
/// # Errors
///
/// Returns JSON, schema, duplicate, mutable-ref, engine, or empty-boundary
/// errors. Loading performs no network access.
pub fn bundled_corpus_profiles() -> Result<CorpusProfileCatalog, CorpusError> {
    let catalog: CorpusProfileCatalog =
        serde_json::from_str(include_str!("../../../config/corpus-profiles.json"))?;
    if catalog.schema_version != "hsai.corpus-profile-catalog.v1" {
        return Err(CorpusError::InvalidProfile {
            profile_id: "catalog".to_owned(),
            reason: "unsupported schema version".to_owned(),
        });
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut repositories = std::collections::BTreeSet::new();
    for profile in &catalog.profiles {
        let invalid_commit = profile.commit.len() != 40
            || profile
                .commit
                .bytes()
                .any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f'));
        let reason = if !ids.insert(profile.profile_id.clone()) {
            Some("duplicate profile ID")
        } else if !repositories.insert(profile.repository.clone()) {
            Some("duplicate repository profile")
        } else if profile.engine != "generic-git-corpus.v1" {
            Some("profile does not use the shared generic-Git engine")
        } else if invalid_commit {
            Some("commit is not immutable lowercase 40-hex")
        } else if profile.roots.is_empty() || profile.excluded_patterns.is_empty() {
            Some("inventory roots and solver-view exclusions are required")
        } else if profile.readiness == SettlementReadiness::SettlementReady {
            Some("archive profile cannot be settlement-ready without live observation")
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(CorpusError::InvalidProfile {
                profile_id: profile.profile_id.clone(),
                reason: reason.to_owned(),
            });
        }
    }
    Ok(catalog)
}

/// Versioned port for immutable challenge corpus materialization.
// Each fallible method returns the adapter-specific `CorpusError`; concrete
// implementations document their narrower failure conditions.
#[allow(clippy::missing_errors_doc)]
pub trait ChallengeCorpusAdapter {
    /// Return the corpus descriptor.
    fn describe_corpus(&self) -> &CorpusDescriptorV1;
    /// Resolve a discovery ref to an immutable snapshot.
    fn snapshot_corpus(&self, reference: &str) -> Result<&CorpusSnapshotV1, CorpusError>;
    /// List stable challenge references in the snapshot.
    fn list_challenges(&self, snapshot_digest: Sha256Digest) -> Result<Vec<String>, CorpusError>;
    /// Return one challenge descriptor.
    fn describe_challenge(
        &self,
        snapshot_digest: Sha256Digest,
        challenge_ref: &str,
    ) -> Result<&ChallengeDescriptorV1, CorpusError>;
    /// Materialize one normalized challenge package.
    fn materialize_challenge(
        &self,
        snapshot_digest: Sha256Digest,
        challenge_ref: &str,
    ) -> Result<&ChallengePackageV1, CorpusError>;
    /// Return the explicit solver-view manifest.
    fn solver_view(
        &self,
        package_digest: Sha256Digest,
    ) -> Result<&SolverViewManifestV1, CorpusError>;
    /// Return item-level rights and provenance evidence.
    fn assess_license_and_provenance(
        &self,
        package_digest: Sha256Digest,
    ) -> Result<&RightsAndProvenanceAssessmentV1, CorpusError>;
    /// Return health without changing corpus state.
    fn health(&self) -> &AdapterHealthV1;
}

/// Hermetic single-challenge adapter used by contract and end-to-end tests.
#[derive(Debug, Clone)]
pub struct LocalFixtureCorpus {
    descriptor: CorpusDescriptorV1,
    snapshot: CorpusSnapshotV1,
    challenge: ChallengeDescriptorV1,
    package: ChallengePackageV1,
    solver_view: SolverViewManifestV1,
    rights: RightsAndProvenanceAssessmentV1,
    health: AdapterHealthV1,
}

impl LocalFixtureCorpus {
    /// Construct the deterministic local challenge package.
    ///
    /// # Errors
    ///
    /// Returns an error only if a built-in fixture value is not canonicalizable.
    #[allow(clippy::too_many_lines)]
    pub fn new(now: UtcTimestamp) -> Result<Self, CorpusError> {
        let public_digest =
            domain_separated_digest("hsai.fixture-artifact.v1", &"public challenge instructions")?;
        let verifier_digest =
            domain_separated_digest("hsai.fixture-artifact.v1", &"hidden verifier")?;
        let snapshot_digest = domain_separated_digest(
            "hsai.corpus-snapshot.v1",
            &(
                "local-fixture",
                "commit:0000000000000000000000000000000000000001",
            ),
        )?;
        let runtime_manifest_digest =
            domain_separated_digest("hsai.runtime-manifest.v1", &verifier_digest)?;
        let verifier_manifest_digest =
            domain_separated_digest("hsai.verifier-manifest.v1", &verifier_digest)?;
        let solver_view_manifest_digest =
            domain_separated_digest("hsai.solver-view.v1", &(public_digest, verifier_digest))?;
        let rights_assessment_digest =
            domain_separated_digest("hsai.rights-assessment.v1", &"fixture-permitted")?;
        let package_digest = domain_separated_digest(
            "hsai.challenge-package.v1",
            &(
                snapshot_digest,
                public_digest,
                verifier_digest,
                solver_view_manifest_digest,
            ),
        )?;
        let descriptor = CorpusDescriptorV1 {
            schema_version: "hsai.corpus-descriptor.v1".to_owned(),
            corpus_id: "corpus_local_fixture_v1".to_owned(),
            adapter_id: "local-fixture-corpus".to_owned(),
            upstream_uri: "fixture://local/computational-market".to_owned(),
            discovery_ref: "commit:0000000000000000000000000000000000000001".to_owned(),
            rights_status: RightsStatus::Permitted,
            created_at: now.clone(),
        };
        let snapshot = CorpusSnapshotV1 {
            schema_version: "hsai.corpus-snapshot.v1".to_owned(),
            corpus_id: descriptor.corpus_id.clone(),
            resolved_commit: descriptor.discovery_ref.clone(),
            snapshot_digest,
            submodule_digests: BTreeMap::new(),
            lfs_object_digests: Vec::new(),
            image_digests: Vec::new(),
            external_artifact_digests: Vec::new(),
            adapter_version: "1.0.0".to_owned(),
            received_at: now.clone(),
        };
        let challenge_spec_digest = domain_separated_digest(
            "hsai.challenge-spec.v1",
            &"candidate bytes must equal hsai-pass-v1",
        )?;
        let challenge = ChallengeDescriptorV1 {
            schema_version: outcomes_domain::CHALLENGE_DESCRIPTOR_SCHEMA_V1.to_owned(),
            challenge_id: "challenge_local_exact_bytes_v1".to_owned(),
            source_platform: "local-fixture".to_owned(),
            source_namespace: "local.demo".to_owned(),
            source_challenge_ref: "exact-bytes-v1".to_owned(),
            display_name: "Local exact bytes validator".to_owned(),
            canonical_spec_uri: "fixture://local/exact-bytes-v1/spec".to_owned(),
            canonical_spec_digest: challenge_spec_digest,
            source_terms_uri: "fixture://local/terms".to_owned(),
            source_timezone: "UTC".to_owned(),
            source_adapter_kind: "local-fixture-corpus".to_owned(),
            source_adapter_version: "1.0.0".to_owned(),
            created_at: now.clone(),
            display_metadata: BTreeMap::new(),
        };
        let package = ChallengePackageV1 {
            schema_version: "hsai.challenge-package.v1".to_owned(),
            package_id: "package_local_exact_bytes_v1".to_owned(),
            corpus_snapshot_digest: snapshot_digest,
            challenge_ref: challenge.source_challenge_ref.clone(),
            artifacts: vec![
                PackageArtifactV1 {
                    logical_path: "public/README.txt".to_owned(),
                    digest: public_digest,
                    byte_length: 29,
                    media_type: "text/plain".to_owned(),
                    visibility: PackageVisibility::SolverPublic,
                    executable: false,
                },
                PackageArtifactV1 {
                    logical_path: "verifier/checker".to_owned(),
                    digest: verifier_digest,
                    byte_length: 15,
                    media_type: "application/octet-stream".to_owned(),
                    visibility: PackageVisibility::VerifierOnly,
                    executable: true,
                },
            ],
            runtime_manifest_digest,
            verifier_manifest_digest,
            solver_view_manifest_digest,
            rights_assessment_digest,
            package_digest,
        };
        let solver_view = SolverViewManifestV1 {
            schema_version: "hsai.solver-view-manifest.v1".to_owned(),
            challenge_package_digest: package_digest,
            exposed_artifact_digests: vec![public_digest],
            excluded_secret_digests: vec![verifier_digest],
            manifest_digest: solver_view_manifest_digest,
        };
        let rights = RightsAndProvenanceAssessmentV1 {
            schema_version: "hsai.rights-and-provenance.v1".to_owned(),
            package_digest,
            upstream_attribution: vec!["HSAI local deterministic fixture".to_owned()],
            repository_license: None,
            item_license: Some("test-fixture-only".to_owned()),
            rights_status: RightsStatus::Permitted,
            redistribution_allowed: true,
            execution_allowed: true,
            commercial_market_use_allowed: false,
            exact_limitation: Some("local test and demonstration only".to_owned()),
            assessed_at: now.clone(),
        };
        let health = AdapterHealthV1 {
            adapter_id: descriptor.adapter_id.clone(),
            status: AdapterHealthStatus::Healthy,
            checked_at: now,
            exact_reason: "hermetic fixture available".to_owned(),
        };
        let adapter = Self {
            descriptor,
            snapshot,
            challenge,
            package,
            solver_view,
            rights,
            health,
        };
        verify_solver_view(&adapter.package, &adapter.solver_view)?;
        Ok(adapter)
    }
}

impl ChallengeCorpusAdapter for LocalFixtureCorpus {
    fn describe_corpus(&self) -> &CorpusDescriptorV1 {
        &self.descriptor
    }

    fn snapshot_corpus(&self, reference: &str) -> Result<&CorpusSnapshotV1, CorpusError> {
        if reference == self.snapshot.resolved_commit {
            Ok(&self.snapshot)
        } else if matches!(reference, "main" | "master" | "development" | "latest") {
            Err(CorpusError::MutableRef(reference.to_owned()))
        } else {
            Err(CorpusError::UnknownSnapshot(reference.to_owned()))
        }
    }

    fn list_challenges(&self, snapshot_digest: Sha256Digest) -> Result<Vec<String>, CorpusError> {
        if snapshot_digest == self.snapshot.snapshot_digest {
            Ok(vec![self.challenge.source_challenge_ref.clone()])
        } else {
            Err(CorpusError::UnknownSnapshot(snapshot_digest.to_string()))
        }
    }

    fn describe_challenge(
        &self,
        snapshot_digest: Sha256Digest,
        challenge_ref: &str,
    ) -> Result<&ChallengeDescriptorV1, CorpusError> {
        self.materialize_challenge(snapshot_digest, challenge_ref)?;
        Ok(&self.challenge)
    }

    fn materialize_challenge(
        &self,
        snapshot_digest: Sha256Digest,
        challenge_ref: &str,
    ) -> Result<&ChallengePackageV1, CorpusError> {
        if snapshot_digest != self.snapshot.snapshot_digest {
            return Err(CorpusError::UnknownSnapshot(snapshot_digest.to_string()));
        }
        if challenge_ref != self.challenge.source_challenge_ref {
            return Err(CorpusError::UnknownChallenge(challenge_ref.to_owned()));
        }
        Ok(&self.package)
    }

    fn solver_view(
        &self,
        package_digest: Sha256Digest,
    ) -> Result<&SolverViewManifestV1, CorpusError> {
        if package_digest == self.package.package_digest {
            Ok(&self.solver_view)
        } else {
            Err(CorpusError::UnknownChallenge(package_digest.to_string()))
        }
    }

    fn assess_license_and_provenance(
        &self,
        package_digest: Sha256Digest,
    ) -> Result<&RightsAndProvenanceAssessmentV1, CorpusError> {
        if package_digest == self.package.package_digest {
            Ok(&self.rights)
        } else {
            Err(CorpusError::UnknownChallenge(package_digest.to_string()))
        }
    }

    fn health(&self) -> &AdapterHealthV1 {
        &self.health
    }
}

/// Verify that a solver view cannot reach secret or non-public package artifacts.
///
/// # Errors
///
/// Returns an exposure or unknown-artifact error.
pub fn verify_solver_view(
    package: &ChallengePackageV1,
    view: &SolverViewManifestV1,
) -> Result<(), CorpusError> {
    for digest in &view.exposed_artifact_digests {
        let artifact = package
            .artifacts
            .iter()
            .find(|artifact| artifact.digest == *digest)
            .ok_or(CorpusError::UnknownSolverArtifact(*digest))?;
        if artifact.visibility != PackageVisibility::SolverPublic
            || view.excluded_secret_digests.contains(digest)
        {
            return Err(CorpusError::SecretExposure(*digest));
        }
    }
    for artifact in &package.artifacts {
        if artifact.visibility != PackageVisibility::SolverPublic
            && !view.excluded_secret_digests.contains(&artifact.digest)
        {
            return Err(CorpusError::SecretExposure(artifact.digest));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Result<UtcTimestamp, outcomes_domain::TimestampError> {
        "2026-07-10T12:00:00Z".parse()
    }

    #[test]
    fn contract_local_fixture_has_stable_snapshot_and_secret_isolation()
    -> Result<(), Box<dyn std::error::Error>> {
        let adapter = LocalFixtureCorpus::new(now()?)?;
        let snapshot = adapter.snapshot_corpus(&adapter.describe_corpus().discovery_ref)?;
        let package = adapter.materialize_challenge(snapshot.snapshot_digest, "exact-bytes-v1")?;
        verify_solver_view(package, adapter.solver_view(package.package_digest)?)?;
        assert_eq!(adapter.list_challenges(snapshot.snapshot_digest)?.len(), 1);
        Ok(())
    }

    #[test]
    fn contract_mutable_ref_fails_at_freeze() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = LocalFixtureCorpus::new(now()?)?;
        assert!(matches!(
            adapter.snapshot_corpus("main"),
            Err(CorpusError::MutableRef(_))
        ));
        Ok(())
    }

    #[test]
    fn contract_solver_view_exposure_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let adapter = LocalFixtureCorpus::new(now()?)?;
        let mut view = adapter.solver_view.clone();
        let secret = adapter.package.artifacts[1].digest;
        view.exposed_artifact_digests.push(secret);
        assert!(matches!(
            verify_solver_view(&adapter.package, &view),
            Err(CorpusError::SecretExposure(_))
        ));
        Ok(())
    }

    #[test]
    fn contract_seven_required_corpus_profiles_are_pinned_and_data_driven()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalog = bundled_corpus_profiles()?;
        assert_eq!(catalog.profiles.len(), 7);
        let author_profiles = catalog
            .profiles
            .iter()
            .filter(|profile| profile.profile_id.starts_with("public-author-"))
            .collect::<Vec<_>>();
        assert_eq!(author_profiles.len(), 3);
        assert!(
            author_profiles
                .iter()
                .all(|profile| profile.engine == "generic-git-corpus.v1")
        );
        assert!(catalog.profiles.iter().all(|profile| {
            profile.readiness != SettlementReadiness::SettlementReady && profile.commit.len() == 40
        }));
        Ok(())
    }
}
