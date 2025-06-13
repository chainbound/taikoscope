export const isValidTimeRange = (range: string): boolean => {
  const match = range.trim().match(/^(\d+)([mh])$/i);
  if (!match) return false;
  const value = parseInt(match[1], 10);
  if (value <= 0) return false;
  const unit = match[2].toLowerCase();
  const minutes = unit === 'h' ? value * 60 : value;
  return minutes <= 24 * 60;
};

export const rangeToHours = (range: string): number => {
  const match = range.trim().match(/^(\d+)([mh])$/i);
  if (!match) return 1;
  const value = parseInt(match[1], 10);
  return match[2].toLowerCase() === 'h' ? value : value / 60;
};
