export const SEQUENCER_PAIRS: Array<[string, string]> = [
  ['0x00a00800c28f2616360dcfadee02d761d14ad94e', 'Chainbound A'],
  ['0x00b00194cdc219921784ab1eb4eaa9634fe1f1a8', 'Chainbound B'],
];

export const SEQUENCER_NAME_BY_ADDRESS: Record<string, string> =
  Object.fromEntries(
    SEQUENCER_PAIRS.map(([addr, name]) => [addr.toLowerCase(), name]),
  );

export const SEQUENCER_ADDRESS_BY_NAME: Record<string, string> =
  Object.fromEntries(SEQUENCER_PAIRS.map(([addr, name]) => [name, addr]));

export const getSequencerName = (address: string): string =>
  SEQUENCER_NAME_BY_ADDRESS[address.toLowerCase()] ?? 'Unknown';

export const getSequencerAddress = (name: string): string | undefined =>
  SEQUENCER_ADDRESS_BY_NAME[name];
