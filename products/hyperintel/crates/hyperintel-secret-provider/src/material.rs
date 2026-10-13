use std::fmt;

use zeroize::{Zeroize, Zeroizing};

use crate::SecretProviderError;

/// Largest secret accepted from an injected resolver.
pub const MAX_SECRET_MATERIAL_BYTES: usize = 65_536;

/// Owned secret bytes with best-effort zeroization on drop.
///
/// This type is intentionally neither cloneable nor serializable. Zeroization
/// covers the current owned allocation, not prior reallocations, consumer
/// copies, allocator state, swap, core dumps, or physical memory.
pub struct SecretMaterialV1 {
    bytes: Zeroizing<Vec<u8>>,
}

impl SecretMaterialV1 {
    /// Move nonempty bounded bytes into a zeroize-on-drop container.
    ///
    /// # Errors
    ///
    /// Returns when `bytes` is empty or exceeds [`MAX_SECRET_MATERIAL_BYTES`].
    pub fn new(bytes: Vec<u8>) -> Result<Self, SecretProviderError> {
        let mut bytes = Zeroizing::new(bytes);
        if bytes.is_empty() || bytes.len() > MAX_SECRET_MATERIAL_BYTES {
            bytes.zeroize();
            return Err(SecretProviderError::InvalidMaterial);
        }
        Ok(Self { bytes })
    }

    /// Explicitly borrow secret bytes.
    ///
    /// The caller is responsible for avoiding copies, formatting, logging, or
    /// persistence of the returned bytes.
    #[must_use]
    pub fn expose_secret_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    /// Return the current byte count without exposing the bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Return whether explicit zeroization has emptied this container.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Zeroize the current allocation before ordinary drop.
    pub fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl fmt::Debug for SecretMaterialV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretMaterialV1([REDACTED])")
    }
}
