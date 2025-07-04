export const SEQUENCER_PAIRS: Array<[string, string]> = [
  ['0x00a00800c28f2616360dcfadee02d761d14ad94e', 'Chainbound A'],
  ['0x00b00194cdc219921784ab1eb4eaa9634fe1f1a8', 'Chainbound B'],
  ['0x0000008f5dd9a790ffbe9142e6828a11c2cf51c0', 'Chainbound'],
  ['0x205a600d515091b473b6c1a8477d967533d10749', 'Chainbound (Taiko)'],
  ['0x445179507c3b0b84cca739398966236a35ad8ea1', 'Gattaca (Taiko)'],
  ['0x75141cd01f50a17a915d59d245ae6b2c947d37d9', 'Nethermind (Taiko)'],
];

export const SEQUENCER_NAME_BY_ADDRESS: Record<string, string> =
  Object.fromEntries(
    SEQUENCER_PAIRS.map(([addr, name]) => [addr.toLowerCase(), name]),
  );

export const SEQUENCER_ADDRESS_BY_NAME: Record<string, string> =
  Object.fromEntries(SEQUENCER_PAIRS.map(([addr, name]) => [name, addr]));

export const getSequencerName = (address: string): string =>
  SEQUENCER_NAME_BY_ADDRESS[address.toLowerCase()] ?? address;

export const getSequencerAddress = (name: string): string | undefined =>
  SEQUENCER_ADDRESS_BY_NAME[name];
