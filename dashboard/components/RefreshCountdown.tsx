import React from 'react';

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
  const [timeLeft, setTimeLeft] = React.useState(refreshRate);

  React.useEffect(() => {
    const update = () => {
      const diff = Date.now() - lastRefresh;
      setTimeLeft(Math.max(refreshRate - diff, 0));
    };
    update();
    const id = setInterval(update, 1000);
    return () => clearInterval(id);
  }, [refreshRate, lastRefresh]);

  const radius = 16;
  const circumference = 2 * Math.PI * radius;
  const progress = 1 - timeLeft / refreshRate;
  const dashoffset = circumference * (1 - progress);

  return (
    <svg
      viewBox="0 0 36 36"
      className="w-8 h-8 cursor-pointer -rotate-90"
      onClick={onRefresh}
      role="button"
      aria-label="Refresh now"
    >
      <circle
        cx="18"
        cy="18"
        r={radius}
        stroke="#e5e7eb"
        strokeWidth="4"
        fill="none"
      />
      <circle
        cx="18"
        cy="18"
        r={radius}
        stroke="#e81899"
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
