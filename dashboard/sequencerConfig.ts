export const SEQUENCER_PAIRS: Array<[string, string]> = [
  // Hekla
  ['0x0000008f5dd9a790ffbe9142e6828a11c2cf51c0', 'Chainbound'],
  ['0x205a600d515091b473b6c1a8477d967533d10749', 'Chainbound (Taiko)'],
  ['0x3c96db4a6cef604de81e12959465ba4b918851e4', 'Gattaca'],
  ['0xf3384dcc14f03f079ac7cd3c2299256b19261bb0', 'Gattaca'],
  ['0x445179507c3b0b84cca739398966236a35ad8ea1', 'Gattaca (Taiko)'],
  ['0xdE023f59daCee4e1B4B4216E1B6DfF624555cd2E', 'Nethermind'],
  ['0x75141cd01f50a17a915d59d245ae6b2c947d37d9', 'Nethermind (Taiko)'],
  // Mainnet
  ['0x000cb000E880A92a8f383D69dA2142a969B93DE7', 'Chainbound'],
  ['0xe2dA8aC2E550cd141198a117520D4EDc8692AB74', 'Gattaca'],
  ['0x2C89DC1b6ECA603AdaCe60A76d3074F3835f6cBE', 'Gattaca'],
  ['0x5F62d006C10C009ff50C878Cd6157aC861C99990', 'Taiko'],
  ['0x7A853a6480F4D7dB79AE91c16c960dBbB6710d25', 'Taiko Fallback'],
  ['0xCbeB5d484b54498d3893A0c3Eb790331962e9e9d', 'Nethermind'],
  // Test addresses used in unit tests
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
  SEQUENCER_NAME_BY_ADDRESS[address.toLowerCase()] ?? address;

export const getSequencerAddress = (name: string): string | undefined =>
  SEQUENCER_ADDRESS_BY_NAME[name];
