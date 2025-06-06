import { useMemo } from 'react';
import { TimeRange } from '../types';
import {
    validateDataConsistency,
    validateDataCompleteness,
    validateTimeSeriesOrder,
    createDataCheck,
    DataValidationResult,
} from '../utils/dataValidator';

interface UseDataValidationProps {
    timeRange: TimeRange;
    chartsData: {
        blockTxData: any[];
        l2BlockTimeData: any[];
        l2GasUsedData: any[];
        secondsToProveData: any[];
        secondsToVerifyData: any[];
        batchBlobCounts: any[];
        sequencerDistribution: any[];
    };
    hasData: boolean;
}

export const useDataValidation = ({
    timeRange,
    chartsData,
    hasData,
}: UseDataValidationProps) => {
    const validationResult = useMemo(() => {
        if (!hasData) {
            return {
                isValid: false,
                errors: ['No data available'],
            } as DataValidationResult;
        }

        // Create consistency checks for key datasets
        const checks = [
            createDataCheck(
                timeRange,
                chartsData.blockTxData,
                'Block Transactions',
            ),
            createDataCheck(
                timeRange,
                chartsData.l2BlockTimeData,
                'L2 Block Times',
            ),
            createDataCheck(
                timeRange,
                chartsData.l2GasUsedData,
                'L2 Gas Usage',
            ),
            createDataCheck(
                timeRange,
                chartsData.secondsToProveData,
                'Prove Times',
            ),
            createDataCheck(
                timeRange,
                chartsData.secondsToVerifyData,
                'Verify Times',
            ),
        ];

        // Validate overall consistency
        const consistencyResult = validateDataConsistency(checks);

        // Validate data completeness for critical datasets
        const completenessResults = [
            validateDataCompleteness(timeRange, chartsData.blockTxData),
            validateDataCompleteness(timeRange, chartsData.l2BlockTimeData),
        ];

        // Validate time series ordering for time-based data
        const timeSeriesResults = [
            validateTimeSeriesOrder(chartsData.l2BlockTimeData),
            validateTimeSeriesOrder(chartsData.secondsToProveData),
            validateTimeSeriesOrder(chartsData.secondsToVerifyData),
        ];

        // Combine all validation results
        const allErrors = [
            ...consistencyResult.errors,
            ...completenessResults.flatMap(r => r.errors),
            ...timeSeriesResults.flatMap(r => r.errors),
        ];

        return {
            isValid: allErrors.length === 0,
            warning: consistencyResult.warning,
            errors: allErrors,
            checks,
        };
    }, [timeRange, chartsData, hasData]);

    const getDataQualityScore = useMemo(() => {
        if (!hasData) return 0;

        const totalChecks = Object.values(chartsData).length;
        const validChecks = Object.values(chartsData).filter(data =>
            Array.isArray(data) && data.length > 0
        ).length;

        return Math.round((validChecks / totalChecks) * 100);
    }, [chartsData, hasData]);

    const hasWarnings = validationResult.warning !== undefined;
    const hasErrors = validationResult.errors.length > 0;

    return {
        validationResult,
        dataQualityScore: getDataQualityScore,
        hasWarnings,
        hasErrors,
        isDataReliable: validationResult.isValid && !hasErrors,
    };
};