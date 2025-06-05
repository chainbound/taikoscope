import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useRouterNavigation } from './useRouterNavigation';

interface UseNavigationHandlerProps {
  onError: (message: string) => void;
}

export const useNavigationHandler = ({
  onError,
}: UseNavigationHandlerProps) => {
  const navigate = useNavigate();
  const { updateSearchParams, navigateToDashboard } = useRouterNavigation();

  const handleBack = useCallback(() => {
    try {
      if (window.history.length > 1) {
        navigate(-1);
      } else {
        navigateToDashboard();
      }
    } catch (err) {
      console.error('Failed to handle back navigation:', err);
      navigateToDashboard();
      onError('Navigation error occurred.');
    }
  }, [navigate, navigateToDashboard, onError]);

  const handleSequencerChange = useCallback(
    (seq: string | null) => {
      try {
        updateSearchParams({ sequencer: seq });
      } catch (err) {
        console.error('Failed to handle sequencer change:', err);
        onError('Failed to update sequencer selection.');
      }
    },
    [updateSearchParams, onError],
  );

  return {
    handleBack,
    handleSequencerChange,
  };
};
