import { useState, useEffect, useCallback } from 'react';
import { loadRefreshRate, saveRefreshRate } from '../utils';

export const useRefreshTimer = () => {
    const [refreshRate, setRefreshRate] = useState<number>(() => loadRefreshRate());
    const [lastRefresh, setLastRefresh] = useState<number>(Date.now());

    useEffect(() => {
        saveRefreshRate(refreshRate);
    }, [refreshRate]);

    const updateLastRefresh = useCallback(() => {
        setLastRefresh(Date.now());
    }, []);

    return {
        refreshRate,
        setRefreshRate,
        lastRefresh,
        updateLastRefresh,
    };
};