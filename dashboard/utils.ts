export const formatSeconds = (seconds: number): string => {
  return seconds >= 120
    ? `${(seconds / 60).toFixed(2)}m`
    : `${seconds.toFixed(2)}s`;
};
