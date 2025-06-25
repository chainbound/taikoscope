use serde::{Deserialize, Serialize};

/// Wrapper around `[u8; 20]` representing an address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct AddressBytes(pub [u8; 20]);

impl From<[u8; 20]> for AddressBytes {
    fn from(value: [u8; 20]) -> Self {
        Self(value)
    }
}

impl From<AddressBytes> for [u8; 20] {
    fn from(value: AddressBytes) -> Self {
        value.0
    }
}

impl AsRef<[u8]> for AddressBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AddressBytes {
    /// Returns the inner byte array.
    pub const fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

/// Wrapper around `[u8; 32]` representing a hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct HashBytes(pub [u8; 32]);

impl From<[u8; 32]> for HashBytes {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

impl From<HashBytes> for [u8; 32] {
    fn from(value: HashBytes) -> Self {
        value.0
    }
}

impl AsRef<[u8]> for HashBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl HashBytes {
    /// Returns the inner byte array.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
