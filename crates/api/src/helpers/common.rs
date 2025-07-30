//! Common helper functions used across API endpoints

use crate::ErrorResponse;
use alloy_primitives::Address;
use axum::http::StatusCode;
use clickhouse_lib::AddressBytes;
use hex::encode;
use primitives::WEI_PER_GWEI;

/// Parse and validate an Ethereum address from a string
pub fn parse_address(addr_str: &str) -> Result<AddressBytes, ErrorResponse> {
    match addr_str.parse::<Address>() {
        Ok(a) => Ok(AddressBytes::from(a)),
        Err(e) => {
            tracing::warn!(error = %e, address = addr_str, "Failed to parse address");
            Err(ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                format!("Invalid address format: {}", e),
            ))
        }
    }
}

/// Parse an optional address string
pub fn parse_optional_address(
    addr_str: Option<&String>,
) -> Result<Option<AddressBytes>, ErrorResponse> {
    match addr_str {
        Some(addr) => parse_address(addr).map(Some),
        None => Ok(None),
    }
}

/// Format an address as a hex string with 0x prefix
pub fn format_address(addr: AddressBytes) -> String {
    Address::from(addr).to_string()
}

/// Format an address bytes as a hex string with 0x prefix
pub fn format_address_bytes(bytes: &[u8]) -> String {
    format!("0x{}", encode(bytes))
}

/// Format `AddressBytes` as a hex string with 0x prefix
pub fn format_address_bytes_type(addr: &AddressBytes) -> String {
    format_address(*addr)
}

/// Convert Wei to Gwei
pub const fn wei_to_gwei(wei: u128) -> u128 {
    wei / WEI_PER_GWEI
}

/// Convert optional Wei to Gwei
pub fn wei_to_gwei_opt(wei: Option<u128>) -> Option<u128> {
    wei.map(wei_to_gwei)
}

/// Create a database error response with logging
pub fn database_error(operation: &str, error: impl std::fmt::Display) -> ErrorResponse {
    tracing::error!(operation = operation, error = %error, "Database operation failed");
    ErrorResponse::database_error()
}

/// Create a database error response for a specific query type
pub fn query_error(query_type: &str, error: impl std::fmt::Display) -> ErrorResponse {
    database_error(&format!("get {}", query_type), error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address_valid() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f8e3A1";
        let result = parse_address(addr).unwrap();
        let formatted = format_address(result);
        assert_eq!(formatted.to_lowercase(), addr.to_lowercase());
    }

    #[test]
    fn test_parse_address_invalid() {
        let addr = "invalid_address";
        let result = parse_address(addr);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.r#type, "invalid-params");
        assert!(err.detail.contains("Invalid address format"));
    }

    #[test]
    fn test_parse_optional_address_some() {
        let addr = String::from("0x742d35Cc6634C0532925a3b844Bc9e7595f8e3A1");
        let result = parse_optional_address(Some(&addr)).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_optional_address_none() {
        let result = parse_optional_address(None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_wei_to_gwei_conversion() {
        assert_eq!(wei_to_gwei(1_000_000_000), 1);
        assert_eq!(wei_to_gwei(5_500_000_000), 5);
        assert_eq!(wei_to_gwei(999_999_999), 0);
    }

    #[test]
    fn test_wei_to_gwei_opt() {
        assert_eq!(wei_to_gwei_opt(Some(1_000_000_000)), Some(1));
        assert_eq!(wei_to_gwei_opt(None), None);
    }

    #[test]
    fn test_format_address_bytes() {
        let bytes = vec![0x74, 0x2d, 0x35];
        assert_eq!(format_address_bytes(&bytes), "0x742d35");
    }
}
