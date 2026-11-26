//! Post-quantum, post-AI (PQAI) hash-only offline capture verifier.
//!
//! Replaces classical `eip191` (`secp256k1`) and `ed25519` authentication with
//! EIP-8141 Frame domain-separated, pure SHA-256 WOTS+ (`w = 16`, RFC 8391 style)
//! one-time hash-based signatures — zero elliptic-curve or lattice assumptions.

use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Fixed schema identifier for PQAI hash-signed compute-market captures.
pub const PQAI_CAPTURE_SCHEMA_V1: &str = "hyperai.scm.eip8141-wots-sha256-capture.v1";
/// Domain separator binding EIP-8141 Frame execution context to SHA-256 WOTS+ chains.
pub const PQAI_FRAME_DOMAIN_V1: &[u8] = b"hyperai:eip8141:wots-sha256:v1\0";

const WOTS_CHAINS: usize = 67; // 64 message nibbles + 3 checksum nibbles (w = 16)
const WOTS_W_MAX: u8 = 15;

/// Public verification key for a single-use WOTS+-SHA256 signer (32-byte commitment).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WotsPublicKeyV1 {
    /// Lowercase hex SHA-256 commitment over all 67 chain endpoints.
    pub root_hex: String,
    /// Lowercase hex 32-byte public seed for domain-separated chain hashing.
    pub pub_seed_hex: String,
}

/// Pure hash-based WOTS+-SHA256 signature (67 x 32-byte chain elements).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WotsSignatureV1 {
    /// 67 lowercase 64-char hex chain values.
    pub elements_hex: Vec<String>,
}

/// Offline PQAI capture envelope authenticated under EIP-8141 Frame + WOTS+-SHA256.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PqaiCaptureEnvelopeV1 {
    /// Must equal [`PQAI_CAPTURE_SCHEMA_V1`].
    pub schema_version: String,
    /// EIP-8141 Frame identity scheme (must be `"eip8141-wots-sha256"`).
    pub identity_scheme: String,
    /// Request identifier.
    pub request_id: String,
    /// Lowercase hex SHA-256 digest of the canonical response body.
    pub response_body_sha256_hex: String,
    /// Signer public key pinned by the caller.
    pub signer: WotsPublicKeyV1,
    /// Pure hash-based WOTS+-SHA256 signature.
    pub signature: WotsSignatureV1,
}

/// Closed errors for PQAI capture verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum PqaiCaptureError {
    /// Unsupported schema version.
    #[error("unsupported pqai capture schema version")]
    UnsupportedSchema,
    /// Classical or non-hash-only identity scheme refused fail-closed.
    #[error("non-pqai identity scheme rejected (require eip8141-wots-sha256)")]
    NonPqaiSchemeRejected,
    /// Signer key is not pinned by caller.
    #[error("signer public key not pinned")]
    UntrustedSigner,
    /// Malformed hex digest or wrong WOTS+ chain count.
    #[error("malformed pqai key, digest, or signature encoding")]
    MalformedEncoding,
    /// Recomputed WOTS+ root does not match pinned signer root.
    #[error("wots-sha256 signature verification failed")]
    SignatureInvalid,
}

fn decode_32(hex_str: &str) -> Result<[u8; 32], PqaiCaptureError> {
    if hex_str.len() != 64 {
        return Err(PqaiCaptureError::MalformedEncoding);
    }
    let bytes = HEXLOWER
        .decode(hex_str.as_bytes())
        .map_err(|_| PqaiCaptureError::MalformedEncoding)?;
    bytes
        .try_into()
        .map_err(|_| PqaiCaptureError::MalformedEncoding)
}

fn chain_step(seed: &[u8; 32], chain_idx: u16, step: u8, val: [u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(PQAI_FRAME_DOMAIN_V1);
    h.update(seed);
    h.update(chain_idx.to_be_bytes());
    h.update([step]);
    h.update(val);
    h.finalize().into()
}

fn chain_advance(
    seed: &[u8; 32],
    chain_idx: u16,
    mut val: [u8; 32],
    start: u8,
    steps: u8,
) -> [u8; 32] {
    for s in start..(start + steps) {
        val = chain_step(seed, chain_idx, s, val);
    }
    val
}

fn msg_nibbles_with_checksum(digest: &[u8; 32]) -> [u8; WOTS_CHAINS] {
    let mut out = [0u8; WOTS_CHAINS];
    let mut csum: u16 = 0;
    for (i, byte) in digest.iter().enumerate() {
        let hi = byte >> 4;
        let lo = byte & 0x0f;
        out[2 * i] = hi;
        out[2 * i + 1] = lo;
        csum += u16::from(WOTS_W_MAX - hi) + u16::from(WOTS_W_MAX - lo);
    }
    out[64] = ((csum >> 8) & 0x0f) as u8;
    out[65] = ((csum >> 4) & 0x0f) as u8;
    out[66] = (csum & 0x0f) as u8;
    out
}

fn frame_signing_digest(env: &PqaiCaptureEnvelopeV1) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(PQAI_FRAME_DOMAIN_V1);
    h.update(env.schema_version.as_bytes());
    h.update(b"\0");
    h.update(env.identity_scheme.as_bytes());
    h.update(b"\0");
    h.update(env.request_id.as_bytes());
    h.update(b"\0");
    h.update(env.response_body_sha256_hex.as_bytes());
    h.finalize().into()
}

fn hash_endpoints(seed: &[u8; 32], endpoints: &[[u8; 32]; WOTS_CHAINS]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(PQAI_FRAME_DOMAIN_V1);
    h.update(b"pk\0");
    h.update(seed);
    for ep in endpoints {
        h.update(ep);
    }
    h.finalize().into()
}

/// Derive a deterministic WOTS+-SHA256 public key and signature from a 32-byte secret seed.
#[must_use]
pub fn sign_pqai_capture_v1(
    secret_seed: &[u8; 32],
    pub_seed: &[u8; 32],
    request_id: &str,
    response_body_sha256_hex: &str,
) -> PqaiCaptureEnvelopeV1 {
    let mut sk_chains = [[0u8; 32]; WOTS_CHAINS];
    let mut pk_endpoints = [[0u8; 32]; WOTS_CHAINS];
    for (idx, (sk_slot, pk_slot)) in sk_chains.iter_mut().zip(pk_endpoints.iter_mut()).enumerate() {
        let mut h = Sha256::new();
        h.update(PQAI_FRAME_DOMAIN_V1);
        h.update(b"sk\0");
        h.update(secret_seed);
        h.update((idx as u16).to_be_bytes());
        let sk: [u8; 32] = h.finalize().into();
        *sk_slot = sk;
        *pk_slot = chain_advance(pub_seed, idx as u16, sk, 0, WOTS_W_MAX);
    }
    let pk_root = hash_endpoints(pub_seed, &pk_endpoints);
    let mut env = PqaiCaptureEnvelopeV1 {
        schema_version: PQAI_CAPTURE_SCHEMA_V1.to_string(),
        identity_scheme: "eip8141-wots-sha256".to_string(),
        request_id: request_id.to_string(),
        response_body_sha256_hex: response_body_sha256_hex.to_string(),
        signer: WotsPublicKeyV1 {
            root_hex: HEXLOWER.encode(&pk_root),
            pub_seed_hex: HEXLOWER.encode(pub_seed),
        },
        signature: WotsSignatureV1 {
            elements_hex: Vec::with_capacity(WOTS_CHAINS),
        },
    };
    let digest = frame_signing_digest(&env);
    let nibbles = msg_nibbles_with_checksum(&digest);
    for (idx, (&sk, &nibble)) in sk_chains.iter().zip(nibbles.iter()).enumerate() {
        let sig_elem = chain_advance(pub_seed, idx as u16, sk, 0, nibble);
        env.signature.elements_hex.push(HEXLOWER.encode(&sig_elem));
    }
    env
}

/// Fail-closed offline verifier for PQAI EIP-8141 + WOTS+-SHA256 capture envelopes.
///
/// Rejects classical (`eip191`/`secp256k1`/`ed25519`) schemes, unpinned signers,
/// malformed digests, and tampered messages or signature chains.
///
/// # Errors
///
/// Returns a closed [`PqaiCaptureError`] on any policy or cryptographic failure.
pub fn verify_pqai_capture_v1(
    env: &PqaiCaptureEnvelopeV1,
    pinned_signers: &[WotsPublicKeyV1],
) -> Result<[u8; 32], PqaiCaptureError> {
    if env.schema_version != PQAI_CAPTURE_SCHEMA_V1 {
        return Err(PqaiCaptureError::UnsupportedSchema);
    }
    if env.identity_scheme != "eip8141-wots-sha256" {
        return Err(PqaiCaptureError::NonPqaiSchemeRejected);
    }
    if !pinned_signers.contains(&env.signer) {
        return Err(PqaiCaptureError::UntrustedSigner);
    }
    let expected_root = decode_32(&env.signer.root_hex)?;
    let pub_seed = decode_32(&env.signer.pub_seed_hex)?;
    let _ = decode_32(&env.response_body_sha256_hex)?;
    if env.signature.elements_hex.len() != WOTS_CHAINS {
        return Err(PqaiCaptureError::MalformedEncoding);
    }

    let digest = frame_signing_digest(env);
    let nibbles = msg_nibbles_with_checksum(&digest);
    let mut recovered = [[0u8; 32]; WOTS_CHAINS];
    for (idx, (elem_hex, &nibble)) in env
        .signature
        .elements_hex
        .iter()
        .zip(nibbles.iter())
        .enumerate()
    {
        let elem = decode_32(elem_hex)?;
        recovered[idx] = chain_advance(&pub_seed, idx as u16, elem, nibble, WOTS_W_MAX - nibble);
    }

    let recomputed_root = hash_endpoints(&pub_seed, &recovered);
    if recomputed_root != expected_root {
        return Err(PqaiCaptureError::SignatureInvalid);
    }
    Ok(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pqai_wots_capture_verifies_and_rejects_tamper_or_classical_scheme() {
        let sk = [0x42u8; 32];
        let seed = [0x24u8; 32];
        let body_hash = "c".repeat(64);
        let env = sign_pqai_capture_v1(&sk, &seed, "req-pqai-001", &body_hash);
        let pins = vec![env.signer.clone()];

        let digest = verify_pqai_capture_v1(&env, &pins).expect("valid pqai signature");
        assert_eq!(digest, frame_signing_digest(&env));

        let mut classical = env.clone();
        classical.identity_scheme = "eip191".to_string();
        assert_eq!(
            verify_pqai_capture_v1(&classical, &pins),
            Err(PqaiCaptureError::NonPqaiSchemeRejected)
        );

        let mut tampered_body = env.clone();
        tampered_body.response_body_sha256_hex = "d".repeat(64);
        assert_eq!(
            verify_pqai_capture_v1(&tampered_body, &pins),
            Err(PqaiCaptureError::SignatureInvalid)
        );

        let mut tampered_sig = env.clone();
        tampered_sig.signature.elements_hex[0] = "0".repeat(64);
        assert_eq!(
            verify_pqai_capture_v1(&tampered_sig, &pins),
            Err(PqaiCaptureError::SignatureInvalid)
        );

        assert_eq!(
            verify_pqai_capture_v1(&env, &[]),
            Err(PqaiCaptureError::UntrustedSigner)
        );
    }
}
