import React from 'react';
import { TAIKO_PINK } from '../theme';

interface RefreshCountdownProps {
  refreshRate: number;
  lastRefresh: number;
  onRefresh: () => void;
}

export const RefreshCountdown: React.FC<RefreshCountdownProps> = ({
  refreshRate,
  lastRefresh,
  onRefresh,
}) => {
  const [timeLeft, setTimeLeft] = React.useState(() =>
    Math.max(refreshRate - (Date.now() - lastRefresh), 0),
  );

  React.useEffect(() => {
    const update = () => {
      const diff = Date.now() - lastRefresh;
      setTimeLeft(Math.max(refreshRate - diff, 0));
    };
    update();
    const id = setInterval(update, 1000);
    const onVisibility = () => {
      if (document.visibilityState === 'visible') update();
    };
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, [refreshRate, lastRefresh]);

  const radius = 16;
  const circumference = 2 * Math.PI * radius;
  const progress = 1 - timeLeft / refreshRate;
  const dashoffset = circumference * (1 - progress);

  return (
    <svg
      viewBox="0 0 36 36"
      className="w-6 h-6 cursor-pointer -rotate-90"
      onClick={onRefresh}
      role="button"
      aria-label="Refresh now"
    >
      <circle
        cx="18"
        cy="18"
        r={radius}
        stroke={TAIKO_PINK}
        strokeWidth="4"
        fill="none"
        strokeDasharray={circumference}
        strokeDashoffset={dashoffset}
        style={{ transition: 'stroke-dashoffset 1s linear' }}
      />
      <text
        x="18"
        y="21"
        textAnchor="middle"
        fontSize="10"
        fill="#374151"
        className="rotate-90"
      >
        {Math.ceil(timeLeft / 1000)}
      </text>
    </svg>
  );
};
