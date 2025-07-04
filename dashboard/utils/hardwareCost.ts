export const HOURS_IN_MONTH = 30 * 24;

export interface HardwareCost {
  totalUsd: number;
  perSequencerUsd: number;
}

export const calculateHardwareCost = (
  cloudCost: number,
  proverCost: number,
  sequencerCount: number,
  hours: number,
): HardwareCost => {
  const count = Math.max(sequencerCount, 1);
  const totalUsd = ((cloudCost + proverCost) * count) / HOURS_IN_MONTH * hours;
  const perSequencerUsd = totalUsd / count;
  return { totalUsd, perSequencerUsd };
};
