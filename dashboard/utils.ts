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

export const formatInterval = (ms: number, showMinutes: boolean): string => {
  return showMinutes
    ? `${formatDecimal(ms / 60000)} minutes`
    : `${Number(formatDecimal(ms / 1000))} seconds`;
};

export const formatBatchDuration = (
  value: number,
  showHours: boolean,
  showMinutes: boolean,
): string => {
  return showHours
    ? `${formatDecimal(value / 3600)} hours`
    : showMinutes
      ? `${formatDecimal(value / 60)} minutes`
      : `${Math.round(value)} seconds`;
};

export const computeBatchDurationFlags = (data: { value: number }[]) => {
  const showHours = data.some((d) => d.value >= 120 * 60);
  const showMinutes = !showHours && data.some((d) => d.value >= 120);
  return { showHours, showMinutes };
};

export const shouldShowMinutes = (data: { timestamp: number }[]) => {
  return data.some((d) => d.timestamp >= 120000);
};

export const findMetricValue = (
  metrics: { title: string | unknown; value: string }[],
  titlePart: string,
) => {
  const metric = metrics.find((m) => {
    const titleStr = typeof m.title === "string" ? m.title : "Avg. Verify Time";
    return titleStr.toLowerCase().includes(titlePart.toLowerCase());
  });
  return metric ? metric.value : "N/A";
};

export const formatSequencerTooltip = (
  data: { value: number }[],
  value: number,
) => {
  const total = data.reduce((acc, curr) => acc + curr.value, 0);
  const percentage = total > 0 ? ((value / total) * 100).toFixed(2) : "0";
  return `${value} blocks (${percentage}%)`;
};
