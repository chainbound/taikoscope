import { useCallback } from 'react';
import { useSearchParams } from './useSearchParams';
import { TableViewState } from './useTableActions';

interface UseNavigationHandlerProps {
    setTableView: (view: TableViewState | null) => void;
    onError: (message: string) => void;
}

export const useNavigationHandler = ({
    setTableView,
    onError,
}: UseNavigationHandlerProps) => {
    const searchParams = useSearchParams();

    const handleBack = useCallback(() => {
        try {
            if (searchParams.navigationState.canGoBack) {
                searchParams.goBack();
            } else {
                // Fallback: navigate to dashboard home
                const url = new URL(window.location.href);
                url.searchParams.delete('view');
                url.searchParams.delete('table');
                url.searchParams.delete('address');
                url.searchParams.delete('page');
                url.searchParams.delete('start');
                url.searchParams.delete('end');
                searchParams.navigate(url, true);
            }
            setTableView(null);
        } catch (err) {
            console.error('Failed to handle back navigation:', err);
            // Emergency fallback: just clear the table view
            setTableView(null);
            onError('Navigation error occurred.');
        }
    }, [searchParams, setTableView, onError]);

    const handleSequencerChange = useCallback((seq: string | null) => {
        try {
            const url = new URL(window.location.href);
            if (seq) {
                url.searchParams.set('sequencer', seq);
            } else {
                url.searchParams.delete('sequencer');
            }
            searchParams.navigate(url);
        } catch (err) {
            console.error('Failed to handle sequencer change:', err);
            onError('Failed to update sequencer selection.');
        }
    }, [searchParams, onError]);

    return {
        handleBack,
        handleSequencerChange,
    };
};