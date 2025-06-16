export const isValidTimeRange = (range: string): boolean => {
  const trimmed = range.trim();
  const match = trimmed.match(/^(\d+)([mh])$/i);
  if (match) {
    const value = parseInt(match[1], 10);
    if (value <= 0) return false;
    const unit = match[2].toLowerCase();
    const minutes = unit === 'h' ? value * 60 : value;
    return minutes <= 24 * 60;
  }

  const custom = trimmed.match(/^(\d+)-(\d+)$/);
  if (custom) {
    const start = parseInt(custom[1], 10);
    const end = parseInt(custom[2], 10);
    if (isNaN(start) || isNaN(end) || end <= start) return false;
    return end - start <= 24 * 60 * 60 * 1000;
  }

  return false;
};

export const rangeToHours = (range: string): number => {
  const trimmed = range.trim();
  const match = trimmed.match(/^(\d+)([mh])$/i);
  if (match) {
    const value = parseInt(match[1], 10);
    return match[2].toLowerCase() === 'h' ? value : value / 60;
  }

  const custom = trimmed.match(/^(\d+)-(\d+)$/);
  if (custom) {
    const start = parseInt(custom[1], 10);
    const end = parseInt(custom[2], 10);
    if (isNaN(start) || isNaN(end) || end <= start) return 1;
    return (end - start) / 3_600_000;
  }

  return 1;
};
