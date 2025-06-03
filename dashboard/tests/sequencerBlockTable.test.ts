import { describe, it, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { SequencerBlockTable } from '../components/SequencerBlockTable';

describe('SequencerBlockTable', () => {
  it('renders rows', () => {
    const html = renderToStaticMarkup(
      React.createElement(SequencerBlockTable, {
        data: [
          { block: 1, txs: 2, sequencer: '0xabc' },
          { block: 2, txs: 3, sequencer: '0xabc' },
        ],
      }),
    );
    expect(html.includes('Block Number')).toBe(true);
    expect(html.includes('Tx Count')).toBe(true);
    expect(html.includes('>1<')).toBe(true);
    expect(html.includes('>3<')).toBe(true);
  });
});
