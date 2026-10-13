//! Offline verifier for caller-selected Simple Compute Market listing captures.
//!
//! The adapter implements only the pinned v2 read-signature contract. It does
//! not call a registry, contact a seller, select offers, or authorize a deal.

use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use data_encoding::HEXLOWER;
use ed25519_dalek::{Signature as Ed25519Signature, VerifyingKey};
use hyperai_foundation::parse_json_without_duplicate_keys;
use secp256k1::{
    Message, PublicKey, Secp256k1,
    ecdsa::{RecoverableSignature, RecoveryId},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest as Sha2Digest, Sha256};
use sha3::{Digest as Sha3Digest, Keccak256};

const REQUEST_PROTOCOL: &str = "arkhai.market-request-signature.v2";
const RESPONSE_PROTOCOL: &str = "arkhai.market-response-signature.v2";
const CAPTURE_SCHEMA: &str = "hyperai.simple-compute-market.listings-capture.v1";
const MAX_INPUT_BYTES: usize = 400_000;
const MAX_LISTINGS_PER_PAGE: usize = 100;
const MAX_SKEW_SECONDS: u64 = 300;
const MAX_CANONICAL_FIELD_BYTES: usize = 4_096;

/// Caller-selected capture and public trust pins, retained only in memory.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListingsCaptureInputV1 {
    /// Versioned request and response capture, with response body preserved as raw JSON text.
    pub capture_json: String,
    /// Caller-controlled buyer and registry identity pins.
    pub trust_json: String,
}

/// Verified result for one source-authenticated registry response.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ListingsCaptureVerificationV1 {
    /// `HyperAI` result schema identifier.
    pub schema_version: &'static str,
    /// Source capture schema identifier accepted by this verifier.
    pub source_capture_schema_version: String,
    /// Verification status under the caller-supplied identity pins.
    pub status: &'static str,
    /// Source product that owns the registry contract.
    pub source_product: &'static str,
    /// Buyer identity from the authenticated request.
    pub buyer_identity: IdentityViewV1,
    /// Registry authority identity from the authenticated response.
    pub registry_identity: IdentityViewV1,
    /// Request ID correlating the source request and response envelopes.
    pub request_id: String,
    /// Signed buyer request timestamp in Unix seconds.
    pub request_timestamp: i64,
    /// Source read operation accepted by this adapter.
    pub operation: &'static str,
    /// SHA-256 of the RFC 8785 canonical query body.
    pub query_body_sha256: String,
    /// Signed response timestamp in Unix seconds.
    pub response_timestamp: i64,
    /// HTTP status covered by the registry response signature.
    pub response_status: u16,
    /// SHA-256 of the RFC 8785 canonical response body.
    pub response_body_sha256: String,
    /// Number of caller-supplied buyer trust pins (one identity or a rotation pair).
    pub buyer_trust_pin_count: usize,
    /// Number of caller-supplied registry trust pins (one identity or a rotation pair).
    pub registry_trust_pin_count: usize,
    /// Number of source listing objects in the captured response page.
    pub listing_count: usize,
    /// Bounded views in source response order; unsupported resources stay opaque.
    pub listing_views: Vec<RegistryListingViewV1>,
    /// Exact parsed source response body, preserving unknown fields and resource schemas.
    pub registry_body: Value,
    /// Fixed claim ceiling for this offline import.
    pub claim_ceiling: &'static str,
    /// Limits that remain true after both signatures verify.
    pub explicit_nonclaims: Vec<&'static str>,
}

/// A display-only listing projection aligned with the original response order.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RegistryListingViewV1 {
    /// Zero-based position in the authenticated source response page.
    pub source_index: usize,
    /// Source listing identifier, retaining its original JSON type.
    pub listing_id: Option<Value>,
    /// Source status when it is a string.
    pub status: Option<String>,
    /// Source listing update value without a freshness interpretation.
    pub updated_at: Option<Value>,
    /// Bounded, display-only resource interpretation.
    pub resource: RegistryResourceViewV1,
}

/// A source-shape classification that never replaces the original payload.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RegistryResourceViewV1 {
    /// `compute_resource`, `api_credits_resource`, `token_resource_opaque`, or `unknown_opaque`.
    pub classification: &'static str,
    /// Source rule used for the classification, or why the value stays opaque.
    pub classification_basis: &'static str,
    /// Recognized display fields only; the complete source object remains unchanged.
    pub fields: Vec<RegistryResourceFieldV1>,
    /// Number of source object keys not included in this bounded display view.
    pub other_payload_field_count: usize,
}

/// One source-defined field shown in a resource summary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RegistryResourceFieldV1 {
    /// User-facing field label.
    pub label: &'static str,
    /// Exact source JSON value for this recognized field.
    pub value: Value,
}

/// A scheme-tagged identity shown with the capture result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IdentityViewV1 {
    /// Identity scheme declared by the source protocol.
    pub scheme: &'static str,
    /// Canonical scheme-specific public identity.
    pub identifier: String,
}

/// Closed input, trust, and verification failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ListingsCaptureError {
    /// Combined selected inputs exceed the local size limit.
    #[error("selected capture and trust inputs exceed the local limit")]
    InputTooLarge,
    /// The capture is malformed or outside the supported source contract.
    #[error("signed registry capture is invalid")]
    InvalidCapture,
    /// Caller-supplied public identity pins are malformed or inconsistent.
    #[error("caller trust pins are invalid")]
    InvalidTrust,
    /// Request or response authentication did not verify under the supplied pins.
    #[error("signed registry capture verification failed")]
    VerificationFailed,
    /// The authenticated response is not a supported listings page.
    #[error("registry response is not a supported listings page")]
    InvalidListingsResponse,
    /// The system clock is not available as a nonnegative Unix timestamp.
    #[error("local verification time is unavailable")]
    ClockUnavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
enum IdentityScheme {
    Eip191,
    Ed25519,
}

impl IdentityScheme {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Eip191 => "eip191",
            Self::Ed25519 => "ed25519",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
struct Identity {
    scheme: IdentityScheme,
    identifier: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentitySet {
    identities: Vec<Identity>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustInput {
    trusted_buyers: IdentitySet,
    trusted_registries: IdentitySet,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    schema_version: String,
    request: AuthenticatedRequest,
    request_body: Value,
    response: CapturedResponse,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthenticatedRequest {
    protocol: String,
    role: String,
    principal: Identity,
    method: String,
    operation: String,
    resource: String,
    request_id: String,
    timestamp: i64,
    body_hash: String,
    proof: SignatureProof,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignatureProof {
    scheme: IdentityScheme,
    value: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapturedResponse {
    status: u16,
    headers: ResponseHeaders,
    body: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseHeaders {
    #[serde(rename = "X-Market-Signature-Version")]
    signature_version: String,
    #[serde(rename = "X-Market-Identity-Scheme")]
    identity_scheme: IdentityScheme,
    #[serde(rename = "X-Market-Identity-Identifier")]
    identity_identifier: String,
    #[serde(rename = "X-Market-Role")]
    role: String,
    #[serde(rename = "X-Market-Request-ID")]
    request_id: String,
    #[serde(rename = "X-Market-Timestamp")]
    timestamp: String,
    #[serde(rename = "X-Market-Signature")]
    signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListingsQueryBody {
    query: Vec<[String; 2]>,
    #[serde(default)]
    if_match: Option<String>,
}

struct VerifiedRequest<'a> {
    envelope: &'a AuthenticatedRequest,
    buyer: Identity,
    query_body_sha256: String,
    buyer_pin_count: usize,
}

struct VerifiedListingsResponse {
    registry: Identity,
    timestamp: i64,
    status: u16,
    body: Value,
    body_sha256: String,
    listing_count: usize,
}

/// Verify one local Simple Compute Market v2 GET `/listings` capture.
///
/// The caller supplies the verification time so tests and offline callers can
/// make the source's 300-second timestamp-skew rule explicit. Both pins are
/// caller-controlled and are not promoted to HyperAI-governed trust.
///
/// # Errors
///
/// Returns a closed error category for size, JSON, schema, trust, context,
/// timestamp, digest, or signature failures.
pub fn verify_listings_capture_v1(
    input: &ListingsCaptureInputV1,
    now: i64,
) -> Result<ListingsCaptureVerificationV1, ListingsCaptureError> {
    if input
        .capture_json
        .len()
        .saturating_add(input.trust_json.len())
        > MAX_INPUT_BYTES
    {
        return Err(ListingsCaptureError::InputTooLarge);
    }
    if now < 0 {
        return Err(ListingsCaptureError::ClockUnavailable);
    }

    let capture = parse_strict::<Capture>(&input.capture_json)
        .map_err(|_| ListingsCaptureError::InvalidCapture)?;
    if capture.schema_version != CAPTURE_SCHEMA {
        return Err(ListingsCaptureError::InvalidCapture);
    }
    let source_capture_schema_version = capture.schema_version.clone();
    let trust = parse_strict::<TrustInput>(&input.trust_json)
        .map_err(|_| ListingsCaptureError::InvalidTrust)?;
    let buyer_pins =
        validate_identity_set(&trust.trusted_buyers).ok_or(ListingsCaptureError::InvalidTrust)?;
    let registry_pins = validate_identity_set(&trust.trusted_registries)
        .ok_or(ListingsCaptureError::InvalidTrust)?;

    let request = verify_request(&capture, &buyer_pins, now)?;
    let response = verify_response(&capture, &request, &registry_pins, now)?;
    let listing_views = listing_views(&response.body)?;

    Ok(ListingsCaptureVerificationV1 {
        schema_version: "hyperai.interface.scm-capture-verification.v1",
        source_capture_schema_version,
        status: "authenticated_under_caller_supplied_pins",
        source_product: "Simple Compute Market",
        buyer_identity: IdentityViewV1 {
            scheme: request.buyer.scheme.as_str(),
            identifier: request.buyer.identifier,
        },
        registry_identity: IdentityViewV1 {
            scheme: response.registry.scheme.as_str(),
            identifier: response.registry.identifier,
        },
        request_id: request.envelope.request_id.clone(),
        request_timestamp: request.envelope.timestamp,
        operation: "listing.list",
        query_body_sha256: request.query_body_sha256,
        response_timestamp: response.timestamp,
        response_status: response.status,
        response_body_sha256: response.body_sha256,
        buyer_trust_pin_count: request.buyer_pin_count,
        registry_trust_pin_count: registry_pins.len(),
        listing_count: response.listing_count,
        listing_views,
        registry_body: response.body,
        claim_ceiling: "offline Simple Compute Market v2 request and registry-response signature verification under caller-supplied identity pins; no HyperAI-governed trust or offer/deal authority",
        explicit_nonclaims: vec![
            "the response signature does not bind the request query-body hash",
            "listing freshness is not established by signature timestamp skew",
            "a still-fresh capture can be verified more than once",
            "caller-supplied pins are not HyperAI-governed trust roots",
            "registry attribution does not prove seller identity, hardware, availability, or correct computation",
            "the imported body is not validated against every source listing schema",
            "the imported listing cannot enter offer selection, buyer approval, or settlement",
        ],
    })
}

fn verify_request<'a>(
    capture: &'a Capture,
    buyer_pins: &BTreeSet<Identity>,
    now: i64,
) -> Result<VerifiedRequest<'a>, ListingsCaptureError> {
    let request = &capture.request;
    let buyer =
        canonical_identity(&request.principal).ok_or(ListingsCaptureError::InvalidCapture)?;
    if request.protocol != REQUEST_PROTOCOL
        || request.role != "buyer"
        || request.method != "GET"
        || request.operation != "listing.list"
        || request.resource != "listings"
        || !is_source_token(&request.role, 64)
        || !is_source_token(&request.operation, 128)
        || !is_request_id(&request.request_id)
        || request.method.len() > 32
        || !request
            .method
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic())
        || request.resource.len() > 1_024
        || request.resource.as_bytes().contains(&0)
        || request.proof.scheme != buyer.scheme
    {
        return Err(ListingsCaptureError::InvalidCapture);
    }
    if !buyer_pins.contains(&buyer) {
        return Err(ListingsCaptureError::VerificationFailed);
    }
    verify_query_shape(&capture.request_body)?;
    if request.timestamp < 0 || now.abs_diff(request.timestamp) > MAX_SKEW_SECONDS {
        return Err(ListingsCaptureError::VerificationFailed);
    }

    let request_body_hash = canonical_body_hash(&capture.request_body)?;
    if request.body_hash != request_body_hash {
        return Err(ListingsCaptureError::VerificationFailed);
    }
    let request_message = frame(&[
        REQUEST_PROTOCOL,
        &request.role,
        buyer.scheme.as_str(),
        &buyer.identifier,
        &request.method,
        &request.operation,
        &request.resource,
        &request.request_id,
        &request.timestamp.to_string(),
        &request_body_hash,
    ])?;
    if !verify_signature(&buyer, &request_message, &request.proof.value) {
        return Err(ListingsCaptureError::VerificationFailed);
    }

    Ok(VerifiedRequest {
        envelope: request,
        buyer,
        query_body_sha256: request_body_hash,
        buyer_pin_count: buyer_pins.len(),
    })
}

fn verify_response(
    capture: &Capture,
    request: &VerifiedRequest<'_>,
    registry_pins: &BTreeSet<Identity>,
    now: i64,
) -> Result<VerifiedListingsResponse, ListingsCaptureError> {
    let response = &capture.response;
    if response.status != 200 {
        return Err(ListingsCaptureError::InvalidListingsResponse);
    }
    let registry = canonical_identity(&Identity {
        scheme: response.headers.identity_scheme,
        identifier: response.headers.identity_identifier.clone(),
    })
    .ok_or(ListingsCaptureError::InvalidCapture)?;
    if !registry_pins.contains(&registry)
        || response.headers.signature_version != RESPONSE_PROTOCOL
        || response.headers.role != "registry"
        || response.headers.request_id != request.envelope.request_id
    {
        return Err(ListingsCaptureError::VerificationFailed);
    }
    let response_timestamp = response
        .headers
        .timestamp
        .parse::<i64>()
        .map_err(|_| ListingsCaptureError::InvalidCapture)?;
    if response_timestamp < 0 || now.abs_diff(response_timestamp) > MAX_SKEW_SECONDS {
        return Err(ListingsCaptureError::VerificationFailed);
    }

    let registry_body = parse_json_without_duplicate_keys(&response.body)
        .map_err(|_| ListingsCaptureError::InvalidListingsResponse)?;
    let listing_count = listing_page(&registry_body)?;
    let response_body_hash = canonical_body_hash(&registry_body)?;
    let response_message = frame(&[
        RESPONSE_PROTOCOL,
        &response.headers.role,
        registry.scheme.as_str(),
        &registry.identifier,
        &request.envelope.method,
        &request.envelope.operation,
        &request.envelope.resource,
        &request.envelope.request_id,
        &response_timestamp.to_string(),
        &response.status.to_string(),
        &response_body_hash,
    ])?;
    if !verify_signature(&registry, &response_message, &response.headers.signature) {
        return Err(ListingsCaptureError::VerificationFailed);
    }

    Ok(VerifiedListingsResponse {
        registry,
        timestamp: response_timestamp,
        status: response.status,
        body: registry_body,
        body_sha256: response_body_hash,
        listing_count,
    })
}

fn verify_query_shape(body: &Value) -> Result<(), ListingsCaptureError> {
    let query: ListingsQueryBody =
        serde_json::from_value(body.clone()).map_err(|_| ListingsCaptureError::InvalidCapture)?;
    let pairs_are_sorted = !query
        .query
        .windows(2)
        .any(|pair| pair[0].as_slice() > pair[1].as_slice());
    let if_match_is_bounded = query
        .if_match
        .is_none_or(|if_match| if_match.len() <= MAX_CANONICAL_FIELD_BYTES);
    if pairs_are_sorted && if_match_is_bounded {
        Ok(())
    } else {
        Err(ListingsCaptureError::InvalidCapture)
    }
}

/// Verify one capture using the current local Unix time.
///
/// # Errors
///
/// Returns the same closed error categories as [`verify_listings_capture_v1`].
pub fn verify_listings_capture_now_v1(
    input: &ListingsCaptureInputV1,
) -> Result<ListingsCaptureVerificationV1, ListingsCaptureError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .ok_or(ListingsCaptureError::ClockUnavailable)?;
    verify_listings_capture_v1(input, now)
}

fn parse_strict<T: for<'de> Deserialize<'de>>(input: &str) -> Result<T, ListingsCaptureError> {
    let value = parse_json_without_duplicate_keys(input)
        .map_err(|_| ListingsCaptureError::InvalidCapture)?;
    serde_json::from_value(value).map_err(|_| ListingsCaptureError::InvalidCapture)
}

fn is_source_token(value: &str, max_bytes: usize) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= max_bytes
        && bytes[0].is_ascii_alphabetic()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(byte))
}

fn is_request_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 128
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(byte))
}

fn validate_identity_set(set: &IdentitySet) -> Option<BTreeSet<Identity>> {
    if !(1..=2).contains(&set.identities.len()) {
        return None;
    }
    let mut identities = BTreeSet::new();
    for identity in &set.identities {
        let canonical = canonical_identity(identity)?;
        if !identities.insert(canonical) {
            return None;
        }
    }
    Some(identities)
}

fn canonical_identity(identity: &Identity) -> Option<Identity> {
    match identity.scheme {
        IdentityScheme::Eip191 => {
            let value = identity.identifier.as_bytes();
            if value.len() != 42
                || !value.starts_with(b"0x")
                || value[2..].iter().any(|byte| !byte.is_ascii_hexdigit())
            {
                return None;
            }
            Some(Identity {
                scheme: identity.scheme,
                identifier: identity.identifier.to_ascii_lowercase(),
            })
        }
        IdentityScheme::Ed25519 => {
            let decoded = URL_SAFE_NO_PAD
                .decode(identity.identifier.as_bytes())
                .ok()?;
            if decoded.len() != 32 || URL_SAFE_NO_PAD.encode(decoded) != identity.identifier {
                return None;
            }
            Some(identity.clone())
        }
    }
}

fn canonical_body_hash(body: &Value) -> Result<String, ListingsCaptureError> {
    let canonical =
        serde_json_canonicalizer::to_vec(body).map_err(|_| ListingsCaptureError::InvalidCapture)?;
    Ok(hex_digest(Sha256::digest(canonical).as_slice()))
}

fn hex_digest(bytes: &[u8]) -> String {
    HEXLOWER.encode(bytes)
}

fn frame(fields: &[&str]) -> Result<Vec<u8>, ListingsCaptureError> {
    let mut message = Vec::new();
    for field in fields {
        let bytes = field.as_bytes();
        if bytes.len() > MAX_CANONICAL_FIELD_BYTES {
            return Err(ListingsCaptureError::InvalidCapture);
        }
        let length =
            u32::try_from(bytes.len()).map_err(|_| ListingsCaptureError::InvalidCapture)?;
        message.extend_from_slice(&length.to_be_bytes());
        message.extend_from_slice(bytes);
    }
    Ok(message)
}

fn verify_signature(identity: &Identity, message: &[u8], encoded_signature: &str) -> bool {
    match identity.scheme {
        IdentityScheme::Ed25519 => verify_ed25519(identity, message, encoded_signature),
        IdentityScheme::Eip191 => verify_eip191(identity, message, encoded_signature),
    }
}

fn verify_ed25519(identity: &Identity, message: &[u8], encoded_signature: &str) -> bool {
    let Some(public_bytes) = URL_SAFE_NO_PAD
        .decode(identity.identifier.as_bytes())
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
    else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_bytes(&public_bytes) else {
        return false;
    };
    let Ok(signature_bytes) = URL_SAFE_NO_PAD.decode(encoded_signature.as_bytes()) else {
        return false;
    };
    let Ok(signature) = Ed25519Signature::from_slice(&signature_bytes) else {
        return false;
    };
    URL_SAFE_NO_PAD.encode(signature.to_bytes()) == encoded_signature
        && key.verify_strict(message, &signature).is_ok()
}

fn verify_eip191(identity: &Identity, message: &[u8], encoded_signature: &str) -> bool {
    if encoded_signature.len() != 132 || !encoded_signature.starts_with("0x") {
        return false;
    }
    let Ok(proof) = HEXLOWER.decode(&encoded_signature.as_bytes()[2..]) else {
        return false;
    };
    let Some(signature) = proof.get(..64) else {
        return false;
    };
    let Some(recovery_byte) = proof.get(64).copied() else {
        return false;
    };
    let recovery_id = match recovery_byte {
        0 | 27 => 0,
        1 | 28 => 1,
        _ => return false,
    };
    let recovery_id = RecoveryId::from_u8_masked(recovery_id);
    let Ok(signature) = RecoverableSignature::from_compact(signature, recovery_id) else {
        return false;
    };
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let mut personal_message = Vec::with_capacity(prefix.len().saturating_add(message.len()));
    personal_message.extend_from_slice(prefix.as_bytes());
    personal_message.extend_from_slice(message);
    let digest = Keccak256::digest(personal_message);
    let digest: [u8; 32] = digest.into();
    let message = Message::from_digest(digest);
    let secp = Secp256k1::verification_only();
    let Ok(public_key) = secp.recover_ecdsa(message, &signature) else {
        return false;
    };
    let recovered = ethereum_address(&public_key);
    recovered == identity.identifier
}

fn ethereum_address(public_key: &PublicKey) -> String {
    let uncompressed = public_key.serialize_uncompressed();
    let digest = Keccak256::digest(&uncompressed[1..]);
    let address = &digest[12..];
    format!("0x{}", HEXLOWER.encode(address))
}

fn listing_page(body: &Value) -> Result<usize, ListingsCaptureError> {
    Ok(listing_entries(body)?.len())
}

fn listing_entries(body: &Value) -> Result<Vec<&Value>, ListingsCaptureError> {
    let entries = match body {
        Value::Array(entries) => entries,
        Value::Object(object) => {
            let entries = ["items", "listings", "data"]
                .iter()
                .filter_map(|key| object.get(*key))
                .find(|value| match value {
                    Value::Array(entries) => !entries.is_empty(),
                    Value::Null | Value::Bool(false) => false,
                    Value::String(value) => !value.is_empty(),
                    _ => true,
                });
            match entries {
                Some(Value::Array(entries)) => entries,
                Some(_) => return Err(ListingsCaptureError::InvalidListingsResponse),
                None => return Ok(Vec::new()),
            }
        }
        _ => return Err(ListingsCaptureError::InvalidListingsResponse),
    };
    if entries.len() > MAX_LISTINGS_PER_PAGE
        || entries.iter().any(|entry| entry.as_object().is_none())
    {
        return Err(ListingsCaptureError::InvalidListingsResponse);
    }
    Ok(entries.iter().collect())
}

fn listing_views(body: &Value) -> Result<Vec<RegistryListingViewV1>, ListingsCaptureError> {
    listing_entries(body)?
        .into_iter()
        .enumerate()
        .map(|(source_index, entry)| {
            let resource = entry.get("offer_resource").or_else(|| entry.get("offer"));
            Ok(RegistryListingViewV1 {
                source_index,
                listing_id: entry.get("listing_id").or_else(|| entry.get("id")).cloned(),
                status: entry
                    .get("status")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                updated_at: entry.get("updated_at").cloned(),
                resource: summarize_resource(resource),
            })
        })
        .collect()
}

fn summarize_resource(resource: Option<&Value>) -> RegistryResourceViewV1 {
    let Some(object) = resource.and_then(Value::as_object) else {
        return opaque_resource("The source resource is not a JSON object.", 0);
    };

    if object.get("kind").and_then(Value::as_str) == Some("api_credits.v1") {
        let service_name = object
            .get("service_name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        let capacity_site_id = object
            .get("capacity_site_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        if let (Some(service_name), Some(capacity_site_id)) = (service_name, capacity_site_id) {
            return RegistryResourceViewV1 {
                classification: "api_credits_resource",
                classification_basis: "kind=api_credits.v1 with required service_name and capacity_site_id",
                fields: vec![
                    resource_field("Service", Value::String(service_name.to_owned())),
                    resource_field("Capacity site", Value::String(capacity_site_id.to_owned())),
                ],
                other_payload_field_count: object.len().saturating_sub(3),
            };
        }
        return opaque_resource(
            "The api_credits.v1 marker is present, but required fields do not match the source shape.",
            object.len(),
        );
    }

    // The pinned VM resource parser gives `token` precedence over `gpu_model`.
    // Token values stay opaque here; amounts are never interpreted as fiat.
    if object.contains_key("token") {
        return RegistryResourceViewV1 {
            classification: "token_resource_opaque",
            classification_basis: "token field takes precedence in the pinned VM resource parser; token metadata and amount remain opaque",
            fields: Vec::new(),
            other_payload_field_count: object.len(),
        };
    }

    if object.contains_key("gpu_model") {
        let gpu_model = object
            .get("gpu_model")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        let gpu_count = object
            .get("gpu_count")
            .filter(|value| value.as_i64().is_some() || value.as_u64().is_some());
        let sla = object.get("sla").filter(|value| value.as_f64().is_some());
        let region = object
            .get("region")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty());
        if let (Some(gpu_model), Some(gpu_count), Some(sla), Some(region)) =
            (gpu_model, gpu_count, sla, region)
        {
            return RegistryResourceViewV1 {
                classification: "compute_resource",
                classification_basis: "gpu_model dispatch with required gpu_count, sla, and region fields",
                fields: vec![
                    resource_field("GPU model", Value::String(gpu_model.to_owned())),
                    resource_field("GPU count", gpu_count.clone()),
                    resource_field("SLA", sla.clone()),
                    resource_field("Region", Value::String(region.to_owned())),
                ],
                other_payload_field_count: object.len().saturating_sub(4),
            };
        }
        return opaque_resource(
            "The gpu_model dispatch field is present, but required compute fields do not match the source shape.",
            object.len(),
        );
    }

    opaque_resource(
        "No supported source resource shape matched; the original payload is preserved.",
        object.len(),
    )
}

fn resource_field(label: &'static str, value: Value) -> RegistryResourceFieldV1 {
    RegistryResourceFieldV1 { label, value }
}

fn opaque_resource(
    reason: &'static str,
    other_payload_field_count: usize,
) -> RegistryResourceViewV1 {
    RegistryResourceViewV1 {
        classification: "unknown_opaque",
        classification_basis: reason,
        fields: Vec::new(),
        other_payload_field_count,
    }
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use data_encoding::HEXLOWER;
    use ed25519_dalek::{Signer as _, SigningKey};
    use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
    use serde_json::{Value, json};
    use sha2::{Digest as Sha2Digest, Sha256};
    use sha3::{Digest as Sha3Digest, Keccak256};

    use super::{
        CAPTURE_SCHEMA, IdentityScheme, ListingsCaptureError, ListingsCaptureInputV1,
        canonical_body_hash, ethereum_address, frame, summarize_resource,
        verify_listings_capture_v1,
    };

    const NOW: i64 = 1_790_281_201;

    struct TestPrincipal {
        scheme: IdentityScheme,
        identifier: String,
        secret: [u8; 32],
    }

    fn principal(
        scheme: IdentityScheme,
        seed: u8,
    ) -> Result<TestPrincipal, Box<dyn std::error::Error>> {
        let secret = [seed; 32];
        let identifier = match scheme {
            IdentityScheme::Ed25519 => {
                let key = SigningKey::from_bytes(&secret);
                URL_SAFE_NO_PAD.encode(key.verifying_key().to_bytes())
            }
            IdentityScheme::Eip191 => {
                let key = SecretKey::from_byte_array(secret)?;
                let public_key = PublicKey::from_secret_key(&Secp256k1::new(), &key);
                ethereum_address(&public_key)
            }
        };
        Ok(TestPrincipal {
            scheme,
            identifier,
            secret,
        })
    }

    fn sign(
        principal: &TestPrincipal,
        message: &[u8],
    ) -> Result<String, Box<dyn std::error::Error>> {
        match principal.scheme {
            IdentityScheme::Ed25519 => {
                let key = SigningKey::from_bytes(&principal.secret);
                Ok(URL_SAFE_NO_PAD.encode(key.sign(message).to_bytes()))
            }
            IdentityScheme::Eip191 => {
                let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
                let mut personal_message = Vec::new();
                personal_message.extend_from_slice(prefix.as_bytes());
                personal_message.extend_from_slice(message);
                let digest: [u8; 32] = Keccak256::digest(personal_message).into();
                let signature = Secp256k1::new()
                    .sign_ecdsa_recoverable(
                        Message::from_digest(digest),
                        &SecretKey::from_byte_array(principal.secret)?,
                    )
                    .serialize_compact();
                let mut raw = signature.1.to_vec();
                let recovery_id = match signature.0 {
                    secp256k1::ecdsa::RecoveryId::Zero => 27,
                    secp256k1::ecdsa::RecoveryId::One => 28,
                    secp256k1::ecdsa::RecoveryId::Two | secp256k1::ecdsa::RecoveryId::Three => {
                        return Err("unsupported Ethereum recovery id".into());
                    }
                };
                raw.push(recovery_id);
                Ok(format!("0x{}", HEXLOWER.encode(&raw)))
            }
        }
    }

    fn identity_json(principal: &TestPrincipal) -> Value {
        json!({
            "scheme": principal.scheme.as_str(),
            "identifier": principal.identifier,
        })
    }

    fn make_input(
        scheme: IdentityScheme,
        query: &Value,
        response_request_id: Option<&str>,
        request_timestamp: i64,
        response_timestamp: i64,
        response_body: &str,
    ) -> Result<ListingsCaptureInputV1, Box<dyn std::error::Error>> {
        let buyer = principal(scheme, 7)?;
        let registry = principal(scheme, 9)?;
        let query_hash = canonical_body_hash(query)?;
        let request_id = "capture-test-1";
        let request_fields = frame(&[
            "arkhai.market-request-signature.v2",
            "buyer",
            buyer.scheme.as_str(),
            &buyer.identifier,
            "GET",
            "listing.list",
            "listings",
            request_id,
            &request_timestamp.to_string(),
            &query_hash,
        ])?;
        let request_signature = sign(&buyer, &request_fields)?;
        let parsed_body: Value = serde_json::from_str(response_body)?;
        let response_body_hash = canonical_body_hash(&parsed_body)?;
        let response_fields = frame(&[
            "arkhai.market-response-signature.v2",
            "registry",
            registry.scheme.as_str(),
            &registry.identifier,
            "GET",
            "listing.list",
            "listings",
            response_request_id.unwrap_or(request_id),
            &response_timestamp.to_string(),
            "200",
            &response_body_hash,
        ])?;
        let response_signature = sign(&registry, &response_fields)?;

        let capture = json!({
            "schema_version": CAPTURE_SCHEMA,
            "request": {
                "protocol": "arkhai.market-request-signature.v2",
                "role": "buyer",
                "principal": identity_json(&buyer),
                "method": "GET",
                "operation": "listing.list",
                "resource": "listings",
                "request_id": request_id,
                "timestamp": request_timestamp,
                "body_hash": query_hash,
                "proof": {"scheme": buyer.scheme.as_str(), "value": request_signature},
            },
            "request_body": query.clone(),
            "response": {
                "status": 200,
                "headers": {
                    "X-Market-Signature-Version": "arkhai.market-response-signature.v2",
                    "X-Market-Identity-Scheme": registry.scheme.as_str(),
                    "X-Market-Identity-Identifier": registry.identifier,
                    "X-Market-Role": "registry",
                    "X-Market-Request-ID": response_request_id.unwrap_or(request_id),
                    "X-Market-Timestamp": response_timestamp.to_string(),
                    "X-Market-Signature": response_signature,
                },
                "body": response_body,
            },
        });
        let trust = json!({
            "trusted_buyers": {"identities": [identity_json(&buyer)]},
            "trusted_registries": {"identities": [identity_json(&registry)]},
        });
        Ok(ListingsCaptureInputV1 {
            capture_json: serde_json::to_string(&capture)?,
            trust_json: serde_json::to_string(&trust)?,
        })
    }

    const LISTINGS_BODY: &str = r#"{"items":[{"listing_id":"listing-1","publisher_principals":{"identities":[{"scheme":"ed25519","identifier":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}]},"offer_resource":{"gpu_model":"H100","gpu_count":2,"sla":0.99,"region":"us-east","ram_gb":80},"settlement_options":[{"scheme":"opaque.v1"}]}],"total":1,"limit":2,"offset":0}"#;

    #[test]
    fn contract_both_identity_schemes_verify_the_source_v2_capture()
    -> Result<(), Box<dyn std::error::Error>> {
        for scheme in [IdentityScheme::Ed25519, IdentityScheme::Eip191] {
            let input = make_input(
                scheme,
                &json!({"query":[["limit","2"],["offset","0"],["status","open"]]}),
                None,
                NOW,
                NOW,
                LISTINGS_BODY,
            )?;
            let result = verify_listings_capture_v1(&input, NOW)?;
            assert_eq!(result.status, "authenticated_under_caller_supplied_pins");
            assert_eq!(result.listing_count, 1);
            assert_eq!(
                result.listing_views[0].resource.classification,
                "compute_resource"
            );
            assert_eq!(result.listing_views[0].resource.fields.len(), 4);
            assert_eq!(
                result.listing_views[0].resource.other_payload_field_count,
                1
            );
            assert_eq!(
                result.registry_body["items"][0]["offer_resource"]["gpu_count"],
                2
            );
            assert!(
                result
                    .explicit_nonclaims
                    .contains(&"the response signature does not bind the request query-body hash")
            );
            assert!(
                result
                    .explicit_nonclaims
                    .contains(&"a still-fresh capture can be verified more than once")
            );
        }
        Ok(())
    }

    #[test]
    fn contract_source_generated_v2_vectors_verify_cross_language_signatures()
    -> Result<(), Box<dyn std::error::Error>> {
        let fixture = super::parse_json_without_duplicate_keys(include_str!(
            "../tests/fixtures/source-v2-captures.json"
        ))?;
        assert_eq!(
            fixture["source_revision"],
            "a1c46d20b1b2f1aabe53e1b052921a87cf9d7360"
        );
        assert_eq!(fixture["generated_at_unix_seconds"], NOW);
        let vectors = fixture["vectors"]
            .as_array()
            .ok_or("source vector list is missing")?;
        assert_eq!(vectors.len(), 2);

        for vector in vectors {
            let scheme = vector["scheme"]
                .as_str()
                .ok_or("source scheme is missing")?;
            let input = ListingsCaptureInputV1 {
                capture_json: serde_json::to_string(&vector["capture"])?,
                trust_json: serde_json::to_string(&vector["trust"])?,
            };
            let result = verify_listings_capture_v1(&input, NOW)?;
            assert_eq!(result.status, "authenticated_under_caller_supplied_pins");
            assert_eq!(result.buyer_identity.scheme, scheme);
            assert_eq!(result.registry_identity.scheme, scheme);
            assert_eq!(result.listing_count, 1);
            assert_eq!(
                result.listing_views[0].resource.classification,
                "unknown_opaque"
            );
            assert_eq!(
                result.registry_body["items"][0]["offer_resource"]["resource_schema"],
                "vm.v1"
            );
            assert!(
                result
                    .explicit_nonclaims
                    .contains(&"the response signature does not bind the request query-body hash")
            );
        }
        Ok(())
    }

    #[test]
    fn contract_domain_views_require_source_markers_and_required_fields() {
        let compute = summarize_resource(Some(&json!({
            "gpu_model": "H100",
            "gpu_count": 8,
            "sla": 0.99,
            "region": "us-east",
            "ram_gb": 640
        })));
        assert_eq!(compute.classification, "compute_resource");
        assert_eq!(compute.fields.len(), 4);
        assert_eq!(compute.other_payload_field_count, 1);

        let api_credits = summarize_resource(Some(&json!({
            "kind": "api_credits.v1",
            "service_name": "weather",
            "capacity_site_id": "site-1",
            "base_url": "https://example.invalid"
        })));
        assert_eq!(api_credits.classification, "api_credits_resource");
        assert_eq!(api_credits.fields.len(), 2);
        assert_eq!(api_credits.other_payload_field_count, 1);

        let malformed_compute = summarize_resource(Some(&json!({
            "gpu_model": "H100",
            "gpu_count": true,
            "sla": "0.99",
            "region": "us-east"
        })));
        assert_eq!(malformed_compute.classification, "unknown_opaque");
        assert!(malformed_compute.fields.is_empty());

        let token_and_gpu = summarize_resource(Some(&json!({
            "token": "0x0000000000000000000000000000000000000001",
            "gpu_model": "H100",
            "gpu_count": 1,
            "sla": 1.0,
            "region": "us-east"
        })));
        assert_eq!(token_and_gpu.classification, "token_resource_opaque");
        assert!(token_and_gpu.fields.is_empty());
    }

    #[test]
    fn contract_query_body_may_change_without_invalidating_the_signed_response()
    -> Result<(), Box<dyn std::error::Error>> {
        let changed_query = json!({"query":[["limit","2"],["offset","0"],["status","closed"]]});
        let input = make_input(
            IdentityScheme::Ed25519,
            &changed_query,
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        let result = verify_listings_capture_v1(&input, NOW)?;
        assert!(
            result
                .explicit_nonclaims
                .contains(&"the response signature does not bind the request query-body hash")
        );
        Ok(())
    }

    #[test]
    fn contract_wrong_response_request_id_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            Some("another-request"),
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        assert_eq!(
            verify_listings_capture_v1(&input, NOW),
            Err(ListingsCaptureError::VerificationFailed)
        );
        Ok(())
    }

    #[test]
    fn contract_untrusted_buyer_pin_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let mut input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        let mut trust: Value = serde_json::from_str(&input.trust_json)?;
        trust["trusted_buyers"]["identities"][0]["identifier"] =
            json!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        input.trust_json = serde_json::to_string(&trust)?;
        assert_eq!(
            verify_listings_capture_v1(&input, NOW),
            Err(ListingsCaptureError::VerificationFailed)
        );
        Ok(())
    }

    #[test]
    fn contract_changed_query_body_hash_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let mut input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        let mut capture: Value = serde_json::from_str(&input.capture_json)?;
        capture["request_body"]["query"][0][1] = json!("3");
        input.capture_json = serde_json::to_string(&capture)?;
        assert_eq!(
            verify_listings_capture_v1(&input, NOW),
            Err(ListingsCaptureError::VerificationFailed)
        );
        Ok(())
    }

    #[test]
    fn contract_changed_request_signature_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let mut input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        let mut capture: Value = serde_json::from_str(&input.capture_json)?;
        let signature = capture["request"]["proof"]["value"]
            .as_str()
            .ok_or("request proof is missing")?;
        let mut changed_signature = signature.to_owned();
        changed_signature.replace_range(..1, "A");
        capture["request"]["proof"]["value"] = json!(changed_signature);
        input.capture_json = serde_json::to_string(&capture)?;
        assert_eq!(
            verify_listings_capture_v1(&input, NOW),
            Err(ListingsCaptureError::VerificationFailed)
        );
        Ok(())
    }

    #[test]
    fn contract_duplicate_fresh_capture_is_not_claimed_as_replay_protected()
    -> Result<(), Box<dyn std::error::Error>> {
        let input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        let first = verify_listings_capture_v1(&input, NOW)?;
        let second = verify_listings_capture_v1(&input, NOW)?;
        assert_eq!(first.response_body_sha256, second.response_body_sha256);
        assert!(
            first
                .explicit_nonclaims
                .contains(&"a still-fresh capture can be verified more than once")
        );
        Ok(())
    }

    #[test]
    fn contract_expired_capture_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            None,
            NOW - 301,
            NOW - 301,
            LISTINGS_BODY,
        )?;
        assert_eq!(
            verify_listings_capture_v1(&input, NOW),
            Err(ListingsCaptureError::VerificationFailed)
        );
        Ok(())
    }

    #[test]
    fn contract_tampered_response_body_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let mut input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        let mut capture: Value = serde_json::from_str(&input.capture_json)?;
        capture["response"]["body"] = json!(r#"{"items":[],"total":0,"limit":2,"offset":0}"#);
        input.capture_json = serde_json::to_string(&capture)?;
        assert_eq!(
            verify_listings_capture_v1(&input, NOW),
            Err(ListingsCaptureError::VerificationFailed)
        );
        Ok(())
    }

    #[test]
    fn contract_duplicate_keys_in_all_raw_json_inputs_are_rejected()
    -> Result<(), Box<dyn std::error::Error>> {
        let input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        let mut duplicate_capture = input.clone();
        duplicate_capture.capture_json = duplicate_capture.capture_json.replacen(
            "\"schema_version\":",
            "\"schema_version\":\"ignored\",\"schema_version\":",
            1,
        );
        assert_eq!(
            verify_listings_capture_v1(&duplicate_capture, NOW),
            Err(ListingsCaptureError::InvalidCapture)
        );

        let mut duplicate_trust = input.clone();
        duplicate_trust.trust_json = r#"{"trusted_buyers":{"identities":[]},"trusted_buyers":{"identities":[]},"trusted_registries":{"identities":[]}}"#.to_owned();
        assert_eq!(
            verify_listings_capture_v1(&duplicate_trust, NOW),
            Err(ListingsCaptureError::InvalidTrust)
        );

        let mut duplicate_body = input;
        let mut capture: Value = serde_json::from_str(&duplicate_body.capture_json)?;
        capture["response"]["body"] = json!(r#"{"items":[],"items":[]}"#);
        duplicate_body.capture_json = serde_json::to_string(&capture)?;
        assert_eq!(
            verify_listings_capture_v1(&duplicate_body, NOW),
            Err(ListingsCaptureError::InvalidListingsResponse)
        );
        Ok(())
    }

    #[test]
    fn contract_unsorted_query_pairs_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["status","open"],["limit","2"]]}),
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        assert_eq!(
            verify_listings_capture_v1(&input, NOW),
            Err(ListingsCaptureError::InvalidCapture)
        );
        Ok(())
    }

    #[test]
    fn contract_capture_size_is_bounded() -> Result<(), Box<dyn std::error::Error>> {
        let mut input = make_input(
            IdentityScheme::Ed25519,
            &json!({"query":[["limit","2"]]}),
            None,
            NOW,
            NOW,
            LISTINGS_BODY,
        )?;
        input.capture_json = " ".repeat(super::MAX_INPUT_BYTES);
        assert_eq!(
            verify_listings_capture_v1(&input, NOW),
            Err(ListingsCaptureError::InputTooLarge)
        );
        Ok(())
    }

    #[test]
    fn contract_sha256_helper_matches_the_source_body_hash_shape()
    -> Result<(), Box<dyn std::error::Error>> {
        let body = json!({"z": 2, "a": 1});
        let canonical = serde_json_canonicalizer::to_vec(&body)?;
        assert_eq!(
            canonical_body_hash(&body)?,
            HEXLOWER.encode(&Sha256::digest(canonical))
        );
        Ok(())
    }
}
