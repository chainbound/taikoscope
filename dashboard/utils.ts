export const formatSeconds = (seconds: number): string => {
  if (seconds >= 7200) {
    return `${Number((seconds / 3600).toFixed(2))}h`;
  }
  if (seconds >= 120) {
    return `${Number((seconds / 60).toFixed(2))}m`;
  }
  return `${seconds.toFixed(2)}s`;
};
