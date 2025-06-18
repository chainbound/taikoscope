//! Validation functions for API query parameters

use crate::ErrorResponse;
use axum::http::StatusCode;
use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
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

/// Common query parameters for most endpoints
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct CommonQuery {
    /// Time range specification (e.g., "15m", "1h", "24h", "7d")
    pub range: Option<String>,
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

/// Convert a range string to `ChronoDuration` (e.g., "15m", "1h", "24h", "7d")
pub fn range_duration(range: &Option<String>) -> ChronoDuration {
    if let Some(r) = range.as_deref() {
        let r = r.trim().to_ascii_lowercase();

        if let Some(m) = r.strip_suffix('m') {
            if let Ok(mins) = m.parse::<i64>() {
                let mins = mins.max(0); // Only ensure non-negative, no upper limit
                return ChronoDuration::minutes(mins);
            }
        }

        if let Some(h) = r.strip_suffix('h') {
            if let Ok(hours) = h.parse::<i64>() {
                let hours = hours.max(0); // Only ensure non-negative, no upper limit
                return ChronoDuration::hours(hours);
            }
        }

        if let Some(d) = r.strip_suffix('d') {
            if let Ok(days) = d.parse::<i64>() {
                let hours = (days * 24).max(0); // Only ensure non-negative, no upper limit
                return ChronoDuration::hours(hours);
            }
        }
    }

    ChronoDuration::hours(1)
}

/// Resolve time range to `TimeRange` enum, prioritizing explicit time range params
pub fn resolve_time_range_enum(range: &Option<String>, time_params: &TimeRangeParams) -> TimeRange {
    // If explicit time range parameters are provided, derive the duration from them
    if has_time_range_params(time_params) {
        let now = Utc::now();

        let end = time_params
            .created_lt
            .or(time_params.created_lte)
            .and_then(|ms| Utc.timestamp_millis_opt(ms as i64).single())
            .unwrap_or(now);

        let start = time_params
            .created_gt
            .map(|v| v + 1)
            .or(time_params.created_gte)
            .and_then(|ms| Utc.timestamp_millis_opt(ms as i64).single())
            .unwrap_or_else(|| end - ChronoDuration::hours(1));

        let duration = end.signed_duration_since(start).max(ChronoDuration::zero());
        return TimeRange::from_duration(duration);
    }

    // Otherwise use the range parameter or default
    TimeRange::from_duration(range_duration(range))
}

/// Resolve time range to `DateTime` for endpoints that need since timestamps
pub fn resolve_time_range_since(
    range: &Option<String>,
    time_params: &TimeRangeParams,
) -> DateTime<Utc> {
    let now = Utc::now();

    // If explicit time range parameters are provided, use them
    let lower_bound = time_params.created_gt.map(|v| v + 1).or(time_params.created_gte);

    if let Some(timestamp_ms) = lower_bound {
        if let Some(dt) = Utc.timestamp_millis_opt(timestamp_ms as i64).single() {
            // No time limit enforcement - return the requested time
            return dt;
        }
    }

    // Fall back to range parameter or default - no time limit enforcement
    now - range_duration(range)
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
