//! Validation functions for API query parameters

use crate::ErrorResponse;
use axum::http::StatusCode;
use chrono::{Duration as ChronoDuration, TimeZone};
use clickhouse_lib::TimeRange;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

/// Maximum allowed timestamp (reasonable upper bound to prevent overflow)
const MAX_TIMESTAMP_MS: u64 = 4_102_444_800_000; // Year 2100

/// Base time range filtering parameters
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct TimeRangeParams {
    /// Filter for timestamps greater than this value (exclusive)
    #[serde(rename = "created[gt]", deserialize_with = "crate::validation::de_u64_opt", default)]
    pub created_gt: Option<u64>,
    /// Filter for timestamps greater than or equal to this value (inclusive)
    #[serde(rename = "created[gte]", deserialize_with = "crate::validation::de_u64_opt", default)]
    pub created_gte: Option<u64>,
    /// Filter for timestamps less than this value (exclusive)
    #[serde(rename = "created[lt]", deserialize_with = "crate::validation::de_u64_opt", default)]
    pub created_lt: Option<u64>,
    /// Filter for timestamps less than or equal to this value (inclusive)
    #[serde(rename = "created[lte]", deserialize_with = "crate::validation::de_u64_opt", default)]
    pub created_lte: Option<u64>,
}

/// Base block range filtering parameters
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BlockRangeParams {
    /// Filter for block numbers greater than this value (exclusive)
    #[serde(rename = "block[gt]", deserialize_with = "crate::validation::de_u64_opt", default)]
    pub block_gt: Option<u64>,
    /// Filter for block numbers greater than or equal to this value (inclusive)
    #[serde(rename = "block[gte]", deserialize_with = "crate::validation::de_u64_opt", default)]
    pub block_gte: Option<u64>,
    /// Filter for block numbers less than this value (exclusive)
    #[serde(rename = "block[lt]", deserialize_with = "crate::validation::de_u64_opt", default)]
    pub block_lt: Option<u64>,
    /// Filter for block numbers less than or equal to this value (inclusive)
    #[serde(rename = "block[lte]", deserialize_with = "crate::validation::de_u64_opt", default)]
    pub block_lte: Option<u64>,
}

/// Common query parameters for most endpoints
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct CommonQuery {
    /// Filter by specific address
    pub address: Option<String>,
    /// Time range filtering parameters
    #[serde(flatten)]
    pub time_range: TimeRangeParams,
}

/// Extended query with pagination for endpoints that need it
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct PaginatedQuery {
    /// Common query parameters
    #[serde(flatten)]
    pub common: CommonQuery,
    /// Maximum number of items to return
    pub limit: Option<u64>,
    /// Return items after this cursor (exclusive)
    pub starting_after: Option<u64>,
    /// Return items before this cursor (exclusive)
    pub ending_before: Option<u64>,
}

/// Query parameters that combine block range filters with pagination
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BlockPaginatedQuery {
    /// Block range filtering parameters
    #[serde(flatten)]
    pub block_range: BlockRangeParams,
    /// Maximum number of items to return
    pub limit: Option<u64>,
    /// Return items after this cursor (exclusive)
    pub starting_after: Option<u64>,
    /// Return items before this cursor (exclusive)
    pub ending_before: Option<u64>,
}

/// Query parameters for block profit ranking endpoints
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ProfitQuery {
    /// Common query parameters
    #[serde(flatten)]
    pub common: CommonQuery,
    /// Maximum number of items to return
    pub limit: Option<u64>,
    /// Sort order for profits ("asc" or "desc")
    pub order: Option<String>,
}

/// Unified query parameters that support both regular and aggregated modes
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct UnifiedQuery {
    /// Common query parameters
    #[serde(flatten)]
    pub common: CommonQuery,
    /// Enable aggregated mode (presence of this parameter triggers aggregation)
    pub aggregated: Option<String>,
    /// Maximum number of items to return (only for regular mode)
    pub limit: Option<u64>,
    /// Return items after this cursor (only for regular mode)
    pub starting_after: Option<u64>,
    /// Return items before this cursor (only for regular mode)
    pub ending_before: Option<u64>,
}

/// Query mode determined from parameters
#[derive(Debug, Clone)]
pub enum QueryMode {
    /// Regular paginated mode with specified limit
    Regular {
        /// Maximum number of items to return
        limit: u64,
    },
    /// Aggregated mode with automatic bucketing
    Aggregated,
}

/// Validate unified query parameters and determine query mode
pub fn validate_unified_query(
    params: &UnifiedQuery,
    max_limit: u64,
) -> Result<QueryMode, ErrorResponse> {
    // Validate common time range parameters
    validate_time_range(&params.common.time_range)?;

    // Check if aggregated mode is enabled (parameter present)
    let is_aggregated = params.aggregated.is_some();

    if is_aggregated {
        // In aggregated mode, pagination parameters are not allowed
        if params.limit.is_some() ||
            params.starting_after.is_some() ||
            params.ending_before.is_some()
        {
            return Err(ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                "Pagination parameters (limit, starting_after, ending_before) cannot be used with aggregated mode",
            ));
        }
        Ok(QueryMode::Aggregated)
    } else {
        // In regular mode, validate pagination parameters
        let limit = validate_pagination(
            params.starting_after.as_ref(),
            params.ending_before.as_ref(),
            params.limit.as_ref(),
            max_limit,
        )?;
        Ok(QueryMode::Regular { limit })
    }
}

/// Validate time range parameters for logical consistency
pub fn validate_time_range(params: &TimeRangeParams) -> Result<(), ErrorResponse> {
    // Check for mutually exclusive parameters
    if let (Some(_), Some(_)) = (params.created_gt, params.created_gte) {
        return Err(ErrorResponse::new(
            "invalid-params",
            "Bad Request",
            StatusCode::BAD_REQUEST,
            "created[gt] and created[gte] cannot be used together",
        ));
    }

    if let (Some(_), Some(_)) = (params.created_lt, params.created_lte) {
        return Err(ErrorResponse::new(
            "invalid-params",
            "Bad Request",
            StatusCode::BAD_REQUEST,
            "created[lt] and created[lte] cannot be used together",
        ));
    }

    // Validate timestamp bounds
    for &timestamp in [params.created_gt, params.created_gte, params.created_lt, params.created_lte]
        .iter()
        .flatten()
    {
        if timestamp > MAX_TIMESTAMP_MS {
            return Err(ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                format!("Timestamp {} is too large (max: {})", timestamp, MAX_TIMESTAMP_MS),
            ));
        }
    }

    // Validate logical ranges
    let lower_bound = params.created_gt.map(|v| v + 1).or(params.created_gte);
    let upper_bound = params.created_lt.or(params.created_lte);

    if let (Some(lower), Some(upper)) = (lower_bound, upper_bound) {
        let is_inclusive = params.created_lte.is_some();
        if (is_inclusive && lower > upper) || (!is_inclusive && lower >= upper) {
            return Err(ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                "Invalid time range: start time must be before end time",
            ));
        }
    }

    Ok(())
}

/// Validate block range parameters for logical consistency
pub fn validate_block_range(params: &BlockRangeParams) -> Result<(), ErrorResponse> {
    if let (Some(_), Some(_)) = (params.block_gt, params.block_gte) {
        return Err(ErrorResponse::new(
            "invalid-params",
            "Bad Request",
            StatusCode::BAD_REQUEST,
            "block[gt] and block[gte] cannot be used together",
        ));
    }

    if let (Some(_), Some(_)) = (params.block_lt, params.block_lte) {
        return Err(ErrorResponse::new(
            "invalid-params",
            "Bad Request",
            StatusCode::BAD_REQUEST,
            "block[lt] and block[lte] cannot be used together",
        ));
    }

    let lower_bound = if let Some(gt) = params.block_gt {
        match gt.checked_add(1) {
            Some(v) => Some(v),
            None => {
                return Err(ErrorResponse::new(
                    "invalid-params",
                    "Bad Request",
                    StatusCode::BAD_REQUEST,
                    "block[gt] value is too large",
                ))
            }
        }
    } else {
        params.block_gte
    };
    let upper_bound = params.block_lt.or(params.block_lte);

    if let (Some(lower), Some(upper)) = (lower_bound, upper_bound) {
        let is_inclusive = params.block_lte.is_some();
        if (is_inclusive && lower > upper) || (!is_inclusive && lower >= upper) {
            return Err(ErrorResponse::new(
                "invalid-params",
                "Bad Request",
                StatusCode::BAD_REQUEST,
                "Invalid block range: start block must be before end block",
            ));
        }
    }

    Ok(())
}

/// Validate pagination parameters
pub fn validate_pagination(
    starting_after: Option<&u64>,
    ending_before: Option<&u64>,
    limit: Option<&u64>,
    max_limit: u64,
) -> Result<u64, ErrorResponse> {
    if starting_after.is_some() && ending_before.is_some() {
        return Err(ErrorResponse::new(
            "invalid-params",
            "Bad Request",
            StatusCode::BAD_REQUEST,
            "starting_after and ending_before parameters are mutually exclusive",
        ));
    }

    let effective_limit = limit.copied().unwrap_or(max_limit).min(max_limit);

    Ok(effective_limit)
}

/// Validate that time range and slot range parameters are not mixed
pub fn validate_range_exclusivity(
    has_time_range: bool,
    has_slot_range: bool,
) -> Result<(), ErrorResponse> {
    if has_time_range && has_slot_range {
        return Err(ErrorResponse::new(
            "invalid-params",
            "Bad Request",
            StatusCode::BAD_REQUEST,
            "Time range params cannot be combined with slot range params",
        ));
    }
    Ok(())
}

/// Check if `TimeRangeParams` has any values set
pub const fn has_time_range_params(params: &TimeRangeParams) -> bool {
    params.created_gt.is_some() ||
        params.created_gte.is_some() ||
        params.created_lt.is_some() ||
        params.created_lte.is_some()
}

/// Check if `BlockRangeParams` has any values set
pub const fn has_block_range_params(params: &BlockRangeParams) -> bool {
    params.block_gt.is_some() ||
        params.block_gte.is_some() ||
        params.block_lt.is_some() ||
        params.block_lte.is_some()
}

/// Resolve time range to `TimeRange` enum from explicit time range params
pub fn resolve_time_range_enum(time_params: &TimeRangeParams) -> TimeRange {
    // If explicit time range parameters are provided, derive the duration from them
    if has_time_range_params(time_params) {
        let now = chrono::Utc::now();

        let start = time_params
            .created_gt
            .map(|v| v + 1)
            .or(time_params.created_gte)
            .and_then(|ms| chrono::Utc.timestamp_millis_opt(ms as i64).single())
            .unwrap_or_else(|| now - chrono::Duration::hours(1));

        let end = time_params
            .created_lt
            .or(time_params.created_lte)
            .and_then(|ms| chrono::Utc.timestamp_millis_opt(ms as i64).single())
            .unwrap_or(now);

        let duration = end.signed_duration_since(start).max(chrono::Duration::zero());
        return TimeRange::from_duration(duration);
    }

    // Default to 1 hour when no time range parameters are provided
    TimeRange::from_duration(ChronoDuration::hours(1))
}

/// Resolve time range to `DateTime` for endpoints that need since timestamps
pub fn resolve_time_range_since(time_params: &TimeRangeParams) -> chrono::DateTime<chrono::Utc> {
    let now = chrono::Utc::now();

    // If explicit time range parameters are provided, use them
    let lower_bound = time_params.created_gt.map(|v| v + 1).or(time_params.created_gte);

    if let Some(timestamp_ms) = lower_bound {
        if let Some(dt) = chrono::Utc.timestamp_millis_opt(timestamp_ms as i64).single() {
            return dt;
        }
    }

    // Default to 1 hour ago when no time range parameters are provided
    now - ChronoDuration::hours(1)
}

/// Resolve time range to start and end `DateTime` values
pub fn resolve_time_range_bounds(
    time_params: &TimeRangeParams,
) -> (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) {
    let now = chrono::Utc::now();

    let start = time_params
        .created_gt
        .map(|v| v + 1)
        .or(time_params.created_gte)
        .and_then(|ms| chrono::Utc.timestamp_millis_opt(ms as i64).single())
        .unwrap_or_else(|| now - ChronoDuration::hours(1));

    // Preserve exclusivity semantics for created[lt] by subtracting 1 ms
    // from the computed upper bound, since downstream queries use
    // "inserted_at <= end". This makes the effective condition
    // "inserted_at < created[lt]" when created[lt] is provided.
    let end = if let Some(ms_lt) = time_params.created_lt {
        let adjusted = ms_lt.saturating_sub(1);
        chrono::Utc.timestamp_millis_opt(adjusted as i64).single().unwrap_or(now)
    } else if let Some(ms_lte) = time_params.created_lte {
        chrono::Utc.timestamp_millis_opt(ms_lte as i64).single().unwrap_or(now)
    } else {
        now
    };

    (start, end)
}

/// Custom deserializer that converts a URL-encoded form value into a `u64`.
/// This accepts both bare numbers (e.g. `1750000`) and quoted numbers (e.g.
/// `"1750000"`) to be tolerant of over-encoded clients.
pub fn de_u64_opt<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    // `serde_urlencoded` always yields strings, so deserialize as `Option<String>`
    // and trim any stray quotes before parsing.
    Option::<String>::deserialize(deserializer)?
        .map(|raw| {
            let trimmed = raw.trim_matches('"');
            let value: i64 = trimmed
                .parse()
                .map_err(|e| Error::custom(format!("invalid integer '{}': {}", raw, e)))?;
            u64::try_from(value)
                .map_err(|_| Error::custom(format!("negative value '{}' not allowed", raw)))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range_validation_mutually_exclusive_gt_gte() {
        let params = TimeRangeParams {
            created_gt: Some(100),
            created_gte: Some(200),
            created_lt: None,
            created_lte: None,
        };

        let result = validate_time_range(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.r#type, "invalid-params");
        assert!(err.detail.contains("created[gt] and created[gte] cannot be used together"));
    }

    #[test]
    fn test_time_range_validation_mutually_exclusive_lt_lte() {
        let params = TimeRangeParams {
            created_gt: None,
            created_gte: None,
            created_lt: Some(100),
            created_lte: Some(200),
        };

        let result = validate_time_range(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.r#type, "invalid-params");
        assert!(err.detail.contains("created[lt] and created[lte] cannot be used together"));
    }

    #[test]
    fn test_time_range_validation_invalid_range_gt_lt() {
        let params = TimeRangeParams {
            created_gt: Some(200),
            created_gte: None,
            created_lt: Some(100),
            created_lte: None,
        };

        let result = validate_time_range(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.r#type, "invalid-params");
        assert!(err.detail.contains("Invalid time range: start time must be before end time"));
    }

    #[test]
    fn test_time_range_validation_invalid_range_gte_lte() {
        let params = TimeRangeParams {
            created_gt: None,
            created_gte: Some(200),
            created_lt: None,
            created_lte: Some(100),
        };

        let result = validate_time_range(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.r#type, "invalid-params");
        assert!(err.detail.contains("Invalid time range: start time must be before end time"));
    }

    #[test]
    fn test_time_range_validation_valid_range() {
        let params = TimeRangeParams {
            created_gt: Some(100),
            created_gte: None,
            created_lt: Some(200),
            created_lte: None,
        };

        let result = validate_time_range(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_time_range_validation_equal_boundary_with_lte() {
        let params = TimeRangeParams {
            created_gt: None,
            created_gte: Some(100),
            created_lt: None,
            created_lte: Some(100),
        };

        let result = validate_time_range(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_time_range_validation_too_large_timestamp() {
        let params = TimeRangeParams {
            created_gt: Some(MAX_TIMESTAMP_MS + 1),
            created_gte: None,
            created_lt: None,
            created_lte: None,
        };

        let result = validate_time_range(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.r#type, "invalid-params");
        assert!(err.detail.contains("Timestamp"));
        assert!(err.detail.contains("is too large"));
    }

    #[test]
    fn test_pagination_validation_mutually_exclusive() {
        let result = validate_pagination(Some(&100), Some(&200), None, 10000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.r#type, "invalid-params");
        assert!(
            err.detail
                .contains("starting_after and ending_before parameters are mutually exclusive")
        );
    }

    #[test]
    fn test_pagination_validation_limit_clamped() {
        // Zero limit should remain zero
        let result = validate_pagination(None, None, Some(&0), 10000).unwrap();
        assert_eq!(result, 0);

        // Large limit should be clamped to the maximum
        let result = validate_pagination(None, None, Some(&20000), 10000).unwrap();
        assert_eq!(result, 10000);
    }

    #[test]
    fn test_pagination_validation_valid() {
        let result = validate_pagination(Some(&100), None, Some(&50), 10000).unwrap();
        assert_eq!(result, 50);
    }

    #[test]
    fn test_range_exclusivity_validation() {
        let result = validate_range_exclusivity(true, true);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.r#type, "invalid-params");
        assert!(err.detail.contains("Time range params cannot be combined with slot range params"));
    }

    #[test]
    fn test_has_time_range_params() {
        let empty_params = TimeRangeParams {
            created_gt: None,
            created_gte: None,
            created_lt: None,
            created_lte: None,
        };
        assert!(!has_time_range_params(&empty_params));

        let with_gt = TimeRangeParams {
            created_gt: Some(100),
            created_gte: None,
            created_lt: None,
            created_lte: None,
        };
        assert!(has_time_range_params(&with_gt));
    }

    #[test]
    fn test_block_range_validation_mutually_exclusive_gt_gte() {
        let params = BlockRangeParams {
            block_gt: Some(10),
            block_gte: Some(20),
            block_lt: None,
            block_lte: None,
        };

        let result = validate_block_range(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.detail.contains("block[gt] and block[gte] cannot be used together"));
    }

    #[test]
    fn test_block_range_validation_invalid_range() {
        let params = BlockRangeParams {
            block_gt: Some(200),
            block_gte: None,
            block_lt: Some(100),
            block_lte: None,
        };

        let result = validate_block_range(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_block_range_validation_invalid_range_gte_lte() {
        let params = BlockRangeParams {
            block_gt: None,
            block_gte: Some(200),
            block_lt: None,
            block_lte: Some(100),
        };

        let result = validate_block_range(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_block_range_validation_valid_range() {
        let params = BlockRangeParams {
            block_gt: Some(100),
            block_gte: None,
            block_lt: Some(200),
            block_lte: None,
        };

        let result = validate_block_range(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_block_range_validation_mutually_exclusive_lt_lte() {
        let params = BlockRangeParams {
            block_gt: None,
            block_gte: None,
            block_lt: Some(50),
            block_lte: Some(100),
        };

        let result = validate_block_range(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.detail.contains("block[lt] and block[lte] cannot be used together"));
    }

    #[test]
    fn test_block_range_validation_equal_boundary_with_lte() {
        let params = BlockRangeParams {
            block_gt: None,
            block_gte: Some(100),
            block_lt: None,
            block_lte: Some(100),
        };

        let result = validate_block_range(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_has_block_range_params() {
        let empty_params =
            BlockRangeParams { block_gt: None, block_gte: None, block_lt: None, block_lte: None };
        assert!(!has_block_range_params(&empty_params));

        let with_gt = BlockRangeParams {
            block_gt: Some(1),
            block_gte: None,
            block_lt: None,
            block_lte: None,
        };
        assert!(has_block_range_params(&with_gt));
    }

    #[test]
    fn test_block_range_validation_block_gt_overflow() {
        let params = BlockRangeParams {
            block_gt: Some(u64::MAX),
            block_gte: None,
            block_lt: None,
            block_lte: None,
        };

        let result = validate_block_range(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.detail.contains("too large"));
    }

    #[test]
    fn test_de_u64_opt_rejects_negative() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Wrapper {
            #[serde(deserialize_with = "crate::validation::de_u64_opt")]
            value: Option<u64>,
        }

        let res: Result<Wrapper, _> = serde_urlencoded::from_str("value=-5");
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(err.contains("negative value"));
    }

    #[test]
    fn test_de_u64_opt_accepts_positive() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Wrapper {
            #[serde(deserialize_with = "crate::validation::de_u64_opt")]
            value: Option<u64>,
        }

        let res: Wrapper = serde_urlencoded::from_str("value=42").unwrap();
        assert_eq!(res.value, Some(42));
    }
}
