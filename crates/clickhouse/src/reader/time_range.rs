/// Supported time ranges for analytics queries
#[derive(Copy, Clone, Debug)]
pub enum TimeRange {
    /// Data from the last 15 minutes
    Last15Min,
    /// Data from the last hour
    LastHour,
    /// Data from the last 24 hours
    Last24Hours,
    /// Data from the last 7 days
    Last7Days,
    /// Data from a custom duration in seconds (clamped to 7 days)
    Custom(u64),
}

impl TimeRange {
    /// Maximum allowed range in seconds (30 days).
    const MAX_SECONDS: u64 = 30 * 24 * 3600;

    /// Create a [`TimeRange`] from a [`chrono::Duration`], clamping to the
    /// allowed maximum of thirty days.
    pub fn from_duration(duration: chrono::Duration) -> Self {
        let secs = duration.num_seconds().clamp(0, Self::MAX_SECONDS as i64) as u64;
        match secs {
            900 => Self::Last15Min,
            3600 => Self::LastHour,
            86400 => Self::Last24Hours,
            604800 => Self::Last7Days,
            _ => Self::Custom(secs),
        }
    }

    /// Return the `ClickHouse` interval string for this range.
    pub fn interval(&self) -> String {
        match self {
            Self::Last15Min => "15 MINUTE".to_owned(),
            Self::LastHour => "1 HOUR".to_owned(),
            Self::Last24Hours => "24 HOUR".to_owned(),
            Self::Last7Days => "7 DAY".to_owned(),
            Self::Custom(sec) => format!("{} SECOND", sec),
        }
    }

    /// Return the duration in seconds for this range.
    pub const fn seconds(&self) -> u64 {
        match self {
            Self::Last15Min => 900,
            Self::LastHour => 3600,
            Self::Last24Hours => 86400,
            Self::Last7Days => 604800,
            Self::Custom(sec) => *sec,
        }
    }
}
