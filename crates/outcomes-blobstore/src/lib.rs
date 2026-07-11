//! Filesystem content-addressed storage for the local development profile.
//!
//! Locators are non-authoritative. Every read recomputes the SHA-256 digest and
//! validates the byte length before returning content.

use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use outcomes_domain::{ContentBlobRefV1, EvidenceClassification, Sha256Digest};
use sha2::{Digest as _, Sha256};
use tempfile::NamedTempFile;
use thiserror::Error;

/// Errors returned by the local filesystem content-addressed store.
#[derive(Debug, Error)]
pub enum BlobStoreError {
    /// Filesystem operation failed.
    #[error("blob-store IO failed: {0}")]
    Io(#[from] io::Error),
    /// The caller requested an unsupported digest algorithm.
    #[error("unsupported digest algorithm {0}")]
    UnsupportedDigestAlgorithm(String),
    /// Stored bytes no longer match their committed digest.
    #[error("stored blob digest mismatch: expected {expected}, computed {computed}")]
    DigestMismatch {
        /// Digest committed by the manifest.
        expected: Sha256Digest,
        /// Digest recomputed from stored bytes.
        computed: Sha256Digest,
    },
    /// Stored bytes no longer match their committed length.
    #[error("stored blob length mismatch: expected {expected}, found {actual}")]
    LengthMismatch {
        /// Length committed by the manifest.
        expected: u64,
        /// Length read from storage.
        actual: u64,
    },
    /// Media type was absent.
    #[error("media type must be non-empty")]
    EmptyMediaType,
    /// A temporary file could not be committed without overwriting content.
    #[error("failed to persist content-addressed blob: {0}")]
    Persist(#[from] tempfile::PersistError),
}

/// Immutable filesystem-backed content-addressed storage.
#[derive(Debug, Clone)]
pub struct FileBlobStore {
    root: PathBuf,
}

impl FileBlobStore {
    /// Create a local store rooted at `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Store bytes under their SHA-256 identity and return a portable reference.
    ///
    /// Existing identical content is reused. The API exposes no mutation or
    /// deletion operation.
    ///
    /// # Errors
    ///
    /// Returns an IO or integrity error if the content cannot be written or an
    /// existing object does not match its digest and length.
    pub fn put(
        &self,
        bytes: &[u8],
        media_type: &str,
        classification: EvidenceClassification,
    ) -> Result<ContentBlobRefV1, BlobStoreError> {
        if media_type.trim().is_empty() {
            return Err(BlobStoreError::EmptyMediaType);
        }
        let digest = digest_bytes(bytes);
        let path = self.path_for(digest);
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "digest path has no parent")
        })?;
        fs::create_dir_all(parent)?;
        if path.exists() {
            verify_path(&path, digest, bytes.len() as u64)?;
        } else {
            let mut temporary = NamedTempFile::new_in(parent)?;
            temporary.write_all(bytes)?;
            temporary.flush()?;
            temporary.as_file().sync_all()?;
            match temporary.persist_noclobber(&path) {
                Ok(file) => sync_file(&file)?,
                Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
                    verify_path(&path, digest, bytes.len() as u64)?;
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(ContentBlobRefV1 {
            digest_algorithm: "sha256".to_owned(),
            digest,
            byte_length: bytes.len() as u64,
            media_type: media_type.to_owned(),
            classification,
            storage_locator: format!("cas://sha256/{digest}"),
        })
    }

    /// Read and verify bytes referenced by a content manifest.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported algorithm, missing object, digest
    /// mismatch, length mismatch, or filesystem failure.
    pub fn get(&self, reference: &ContentBlobRefV1) -> Result<Vec<u8>, BlobStoreError> {
        if reference.digest_algorithm != "sha256" {
            return Err(BlobStoreError::UnsupportedDigestAlgorithm(
                reference.digest_algorithm.clone(),
            ));
        }
        let path = self.path_for(reference.digest);
        let bytes = fs::read(path)?;
        let actual_length = bytes.len() as u64;
        if actual_length != reference.byte_length {
            return Err(BlobStoreError::LengthMismatch {
                expected: reference.byte_length,
                actual: actual_length,
            });
        }
        let computed = digest_bytes(&bytes);
        if computed != reference.digest {
            return Err(BlobStoreError::DigestMismatch {
                expected: reference.digest,
                computed,
            });
        }
        Ok(bytes)
    }

    fn path_for(&self, digest: Sha256Digest) -> PathBuf {
        let hex = digest.to_hex();
        self.root.join("sha256").join(&hex[..2]).join(hex)
    }
}

fn digest_bytes(bytes: &[u8]) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Sha256Digest::from_bytes(hasher.finalize().into())
}

fn verify_path(
    path: &Path,
    expected_digest: Sha256Digest,
    expected_length: u64,
) -> Result<(), BlobStoreError> {
    let bytes = fs::read(path)?;
    let actual_length = bytes.len() as u64;
    if actual_length != expected_length {
        return Err(BlobStoreError::LengthMismatch {
            expected: expected_length,
            actual: actual_length,
        });
    }
    let computed = digest_bytes(&bytes);
    if computed != expected_digest {
        return Err(BlobStoreError::DigestMismatch {
            expected: expected_digest,
            computed,
        });
    }
    Ok(())
}

fn sync_file(file: &File) -> Result<(), BlobStoreError> {
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_round_trip_and_duplicate_are_content_addressed() -> Result<(), BlobStoreError> {
        let directory = tempfile::tempdir()?;
        let store = FileBlobStore::new(directory.path());
        let first = store.put(
            b"portable evidence",
            "application/octet-stream",
            EvidenceClassification::Public,
        )?;
        let second = store.put(
            b"portable evidence",
            "application/octet-stream",
            EvidenceClassification::Public,
        )?;
        assert_eq!(first.digest, second.digest);
        assert_eq!(store.get(&first)?, b"portable evidence");
        Ok(())
    }

    #[test]
    fn contract_tampered_blob_is_rejected() -> Result<(), BlobStoreError> {
        let directory = tempfile::tempdir()?;
        let store = FileBlobStore::new(directory.path());
        let reference = store.put(
            b"original",
            "application/octet-stream",
            EvidenceClassification::Public,
        )?;
        fs::write(store.path_for(reference.digest), b"tampered")?;
        assert!(matches!(
            store.get(&reference),
            Err(BlobStoreError::DigestMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn contract_locator_never_contains_host_path() -> Result<(), BlobStoreError> {
        let directory = tempfile::tempdir()?;
        let store = FileBlobStore::new(directory.path());
        let reference = store.put(
            b"hidden host path",
            "text/plain",
            EvidenceClassification::OperatorOnly,
        )?;
        assert!(reference.storage_locator.starts_with("cas://sha256/"));
        assert!(
            !reference
                .storage_locator
                .contains(&directory.path().to_string_lossy()[..])
        );
        Ok(())
    }
}
