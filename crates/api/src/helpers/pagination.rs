//! Pagination utilities

/// Paginate results in descending order by a key function
pub fn paginate_desc<T, F>(
    mut rows: Vec<T>,
    limit: u64,
    starting_after: Option<u64>,
    ending_before: Option<u64>,
    key: F,
) -> Vec<T>
where
    F: Fn(&T) -> u64,
{
    rows.sort_by_key(|r| std::cmp::Reverse(key(r)));
    rows.into_iter()
        .filter(|r| {
            let v = key(r);
            if let Some(start) = starting_after {
                if v >= start {
                    return false;
                }
            }
            if let Some(end) = ending_before {
                if v <= end {
                    return false;
                }
            }
            true
        })
        .take(limit as usize)
        .collect()
}
