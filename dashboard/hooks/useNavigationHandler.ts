import { useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { TableViewState } from './useTableActions';

interface UseNavigationHandlerProps {
    setTableView: (view: TableViewState | null) => void;
    onError: (message: string) => void;
}

export const useNavigationHandler = ({
    setTableView,
    onError,
}: UseNavigationHandlerProps) => {
    const navigate = useNavigate();
    const [searchParams, setSearchParams] = useSearchParams();

    const handleBack = useCallback(() => {
        try {
            if (window.history.length > 1) {
                navigate(-1);
            } else {
                setSearchParams({});
            }
            setTableView(null);
        } catch (err) {
            console.error('Failed to handle back navigation:', err);
            // Emergency fallback: just clear the table view
            setTableView(null);
            onError('Navigation error occurred.');
        }
    }, [navigate, setSearchParams, setTableView, onError]);

    const handleSequencerChange = useCallback((seq: string | null) => {
        try {
            const params = new URLSearchParams(searchParams);
            if (seq) {
                params.set('sequencer', seq);
            } else {
                params.delete('sequencer');
            }
            navigate({ search: params.toString() });
        } catch (err) {
            console.error('Failed to handle sequencer change:', err);
            onError('Failed to update sequencer selection.');
        }
    }, [navigate, searchParams, onError]);

    return {
        handleBack,
        handleSequencerChange,
    };
};