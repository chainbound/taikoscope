import React from 'react';
import { TAIKO_PINK } from '../../theme';

interface LoadingStateProps {
    message?: string;
}

export const LoadingState: React.FC<LoadingStateProps> = ({
    message = 'Loading...'
}) => {
    return (
        <div className="p-4">
            <div className="flex items-center space-x-2">
                <div
                    className="animate-spin rounded-full h-4 w-4 border-b-2"
                    style={{ borderColor: TAIKO_PINK }}
                />
                <span>{message}</span>
            </div>
        </div>
    );
};