use alloy::primitives::{Address, B256};
use derive_more::Deref;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Wrapper around `[u8; 20]` representing an address.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Default,
    Deref,
    ToSchema,
)]
#[schema(
    value_type = String,
    format = "hex",
    description = "20-byte address as hex string",
    example = "0x1234567890123456789012345678901234567890"
)]
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

impl From<Address> for AddressBytes {
    fn from(value: Address) -> Self {
        Self(value.into_array())
    }
}

impl From<AddressBytes> for Address {
    fn from(value: AddressBytes) -> Self {
        Self::from(value.0)
    }
}

impl AsRef<[u8]> for AddressBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AddressBytes {
    /// Returns the inner byte array.
    pub const fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }
}

/// Wrapper around `[u8; 32]` representing a hash.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Deref, ToSchema,
)]
#[schema(
    value_type = String,
    format = "hex",
    description = "32-byte hash as hex string",
    example = "0x1234567890123456789012345678901234567890123456789012345678901234"
)]
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

impl From<B256> for HashBytes {
    fn from(value: B256) -> Self {
        Self(*value.as_ref())
    }
}

impl From<HashBytes> for B256 {
    fn from(value: HashBytes) -> Self {
        Self::from(value.0)
    }
}

impl AsRef<[u8]> for HashBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl HashBytes {
    /// Returns the inner byte array.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
