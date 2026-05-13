import { Buffer } from 'node:buffer';

import { describe, expect, it } from 'vitest';

import { getLongText, getSineWaveAudio } from '../index';

describe('mcp-long-audio fixture content helpers', () => {
  describe('getLongText', () => {
    it('returns a text content block around 25 KB', async () => {
      const block = await getLongText(25);

      expect(block).toEqual({
        type: 'text',
        text: expect.any(String),
      });

      const byteLength = Buffer.byteLength(block.text, 'utf8');
      expect(byteLength).toBeGreaterThanOrEqual(25_000);
      expect(byteLength).toBeLessThanOrEqual(26_000);
    });

    it('returns a text content block around 100 KB', async () => {
      const block = await getLongText(100);

      expect(block).toEqual({
        type: 'text',
        text: expect.any(String),
      });

      const byteLength = Buffer.byteLength(block.text, 'utf8');
      expect(byteLength).toBeGreaterThanOrEqual(100_000);
      expect(byteLength).toBeLessThanOrEqual(101_000);
    });
  });

  describe('getSineWaveAudio', () => {
    it('returns an audio/wav content block with a RIFF/WAVE header', async () => {
      const block = await getSineWaveAudio(1);

      expect(block).toEqual({
        type: 'audio',
        data: expect.any(String),
        mimeType: 'audio/wav',
      });

      const wav = Buffer.from(block.data, 'base64');

      expect([...wav.subarray(0, 4)]).toEqual([0x52, 0x49, 0x46, 0x46]);
      expect([...wav.subarray(8, 12)]).toEqual([0x57, 0x41, 0x56, 0x45]);
    });
  });
});
