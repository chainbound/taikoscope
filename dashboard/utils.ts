export const formatDecimal = (value: number): string => {
  const decimals = Math.abs(value) >= 10 ? 1 : 2;
  return value.toFixed(decimals);
};

export const formatSeconds = (seconds: number): string => {
  if (seconds >= 120 * 60) {
    return `${Number(formatDecimal(seconds / 3600))}h`;
  }
  if (seconds >= 120) {
    return `${Number(formatDecimal(seconds / 60))}m`;
  }
  return `${formatDecimal(seconds)}s`;
};
