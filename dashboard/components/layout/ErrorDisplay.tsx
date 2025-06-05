import React from 'react';

interface ErrorDisplayProps {
    errorMessage: string;
    navigationError?: string;
    errorCount?: number;
    onResetNavigation: () => void;
    onClearError: () => void;
}

export const ErrorDisplay: React.FC<ErrorDisplayProps> = ({
    errorMessage,
    navigationError,
    errorCount,
    onResetNavigation,
    onClearError,
}) => {
    const displayError = errorMessage || navigationError;
    const count = errorCount ?? 0;

    if (!displayError) return null;

    return (
        <div className="mt-4 p-3 bg-red-50 border border-red-200 text-red-700 rounded">
            <div className="flex justify-between items-start">
                <div className="flex-1">
                    {displayError}
                    {count > 0 && (
                        <div className="text-sm mt-1 text-red-600">
                            Navigation issues detected. Try refreshing the page if problems persist.
                        </div>
                    )}
                </div>
                <div className="flex space-x-2 ml-4">
                    {count > 2 && (
                        <button
                            onClick={onResetNavigation}
                            className="text-sm bg-red-600 text-white px-2 py-1 rounded hover:bg-red-700"
                        >
                            Reset
                        </button>
                    )}
                    <button
                        onClick={onClearError}
                        className="text-sm text-red-600 hover:text-red-800"
                    >
                        ✕
                    </button>
                </div>
            </div>
        </div>
    );
};