import React, { useState } from 'react';
import { DataValidationResult } from '../utils/dataValidator';

interface DataQualityIndicatorProps {
    validationResult: DataValidationResult;
    dataQualityScore: number;
    hasWarnings: boolean;
    hasErrors: boolean;
    isDataReliable: boolean;
}

export const DataQualityIndicator: React.FC<DataQualityIndicatorProps> = ({
    validationResult,
    dataQualityScore,
    hasWarnings,
    hasErrors,
    isDataReliable,
}) => {
    const [showDetails, setShowDetails] = useState(false);

    const getStatusColor = () => {
        if (hasErrors) return 'text-red-600 dark:text-red-400';
        if (hasWarnings) return 'text-yellow-600 dark:text-yellow-400';
        return 'text-green-600 dark:text-green-400';
    };

    const getStatusIcon = () => {
        if (hasErrors) return '⚠️';
        if (hasWarnings) return '⚡';
        return '✅';
    };

    const getStatusText = () => {
        if (hasErrors) return 'Data Issues';
        if (hasWarnings) return 'Minor Issues';
        return 'Good Quality';
    };

    if (!hasWarnings && !hasErrors && dataQualityScore > 90) {
        // Don't show indicator when everything is good
        return null;
    }

    return (
        <div className="mb-4">
            <button
                onClick={() => setShowDetails(!showDetails)}
                className={`flex items-center space-x-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors
          ${hasErrors ? 'bg-red-50 dark:bg-red-900/20 hover:bg-red-100 dark:hover:bg-red-900/30' :
                        hasWarnings ? 'bg-yellow-50 dark:bg-yellow-900/20 hover:bg-yellow-100 dark:hover:bg-yellow-900/30' :
                            'bg-green-50 dark:bg-green-900/20 hover:bg-green-100 dark:hover:bg-green-900/30'}`}
            >
                <span className="text-lg">{getStatusIcon()}</span>
                <span className={getStatusColor()}>
                    Data Quality: {getStatusText()} ({dataQualityScore}%)
                </span>
                <span className={`${getStatusColor()} transition-transform ${showDetails ? 'rotate-180' : ''}`}>
                    ▼
                </span>
            </button>

            {showDetails && (
                <div className="mt-2 p-3 bg-gray-50 dark:bg-gray-800 rounded-lg border">
                    <div className="space-y-2">
                        {validationResult.warning && (
                            <div className="flex items-start space-x-2">
                                <span className="text-yellow-500 font-bold">⚡</span>
                                <span className="text-yellow-800 dark:text-yellow-200 text-sm">
                                    {validationResult.warning}
                                </span>
                            </div>
                        )}

                        {validationResult.errors.map((error, index) => (
                            <div key={index} className="flex items-start space-x-2">
                                <span className="text-red-500 font-bold">⚠️</span>
                                <span className="text-red-800 dark:text-red-200 text-sm">
                                    {error}
                                </span>
                            </div>
                        ))}

                        {!hasWarnings && !hasErrors && (
                            <div className="flex items-start space-x-2">
                                <span className="text-green-500 font-bold">✅</span>
                                <span className="text-green-800 dark:text-green-200 text-sm">
                                    All data validation checks passed. Charts and tables are consistent.
                                </span>
                            </div>
                        )}

                        <div className="pt-2 border-t border-gray-200 dark:border-gray-700">
                            <div className="text-xs text-gray-600 dark:text-gray-400">
                                Data Quality Score: {dataQualityScore}% |
                                Reliability: {isDataReliable ? 'High' : 'Moderate'}
                            </div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};