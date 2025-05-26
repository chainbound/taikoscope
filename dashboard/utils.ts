export const formatDecimal = (value: number): string => {
  const decimals = Math.abs(value) >= 10 ? 1 : 2;
  return value.toFixed(decimals);
};

export const formatSeconds = (seconds: number): string => {
  return seconds >= 120
    ? `${Number(formatDecimal(seconds / 60))}m`
    : `${formatDecimal(seconds)}s`;
};
