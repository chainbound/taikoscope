import { TimeRange } from '../types';

export interface DataValidationResult {
    isValid: boolean;
    warning?: string;
    errors: string[];
}

export interface DataConsistencyCheck {
    timeRange: TimeRange;
    chartDataCount: number;
    tableDataCount?: number;
    dataSource: string;
    lastUpdated: number;
}

/**
 * Validates that chart and table data are consistent
 */
export const validateDataConsistency = (
    checks: DataConsistencyCheck[],
): DataValidationResult => {
    const errors: string[] = [];
    let warning: string | undefined;

    // Check for data freshness (within last 5 minutes)
    const now = Date.now();
    const staleDataThreshold = 5 * 60 * 1000; // 5 minutes

    for (const check of checks) {
        // Check data freshness
        if (now - check.lastUpdated > staleDataThreshold) {
            warning = `Some data may be outdated. Last updated ${Math.round((now - check.lastUpdated) / 60000)} minutes ago.`;
        }

        // Check for empty datasets
        if (check.chartDataCount === 0) {
            errors.push(`No chart data available for ${check.dataSource} (${check.timeRange})`);
        }

        // Check for data count consistency between charts and tables if table data is provided
        if (check.tableDataCount !== undefined) {
            // For unlimited data sources, chart and table should have similar counts
            const difference = Math.abs(check.chartDataCount - check.tableDataCount);
            const tolerance = Math.max(1, Math.floor(check.chartDataCount * 0.1)); // 10% tolerance

            if (difference > tolerance) {
                errors.push(
                    `Data inconsistency detected in ${check.dataSource}: Chart has ${check.chartDataCount} records, table has ${check.tableDataCount} records`,
                );
            }
        }
    }

    return {
        isValid: errors.length === 0,
        warning,
        errors,
    };
};

/**
 * Creates a data consistency check object
 */
export const createDataCheck = (
    timeRange: TimeRange,
    chartData: any[],
    dataSource: string,
    tableData?: any[],
): DataConsistencyCheck => ({
    timeRange,
    chartDataCount: chartData?.length || 0,
    tableDataCount: tableData?.length,
    dataSource,
    lastUpdated: Date.now(),
});

/**
 * Validates data completeness for a given time range
 */
export const validateDataCompleteness = (
    timeRange: TimeRange,
    data: any[],
    expectedMinimumRecords: number = 1,
): DataValidationResult => {
    const errors: string[] = [];

    if (!data || data.length === 0) {
        errors.push(`No data available for ${timeRange} time range`);
    } else if (data.length < expectedMinimumRecords) {
        errors.push(
            `Insufficient data: Expected at least ${expectedMinimumRecords} records, got ${data.length}`,
        );
    }

    return {
        isValid: errors.length === 0,
        errors,
    };
};

/**
 * Validates that time series data is properly ordered
 */
export const validateTimeSeriesOrder = (
    data: Array<{ timestamp: number }>,
): DataValidationResult => {
    const errors: string[] = [];

    if (data.length > 1) {
        for (let i = 1; i < data.length; i++) {
            if (data[i].timestamp < data[i - 1].timestamp) {
                errors.push('Time series data is not properly ordered');
                break;
            }
        }
    }

    return {
        isValid: errors.length === 0,
        errors,
    };
};