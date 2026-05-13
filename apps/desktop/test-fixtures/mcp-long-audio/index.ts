import { Buffer } from 'node:buffer';

export type TextContentBlock = {
  type: 'text';
  text: string;
};

export type AudioContentBlock = {
  type: 'audio';
  data: string;
  mimeType: 'audio/wav';
};

const LOREM = [
  'Lorem ipsum dolor sit amet, consectetur adipiscing elit.',
  'Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.',
  'Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.',
  'Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.',
  'Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.',
].join(' ');

export async function getLongText(sizeKb: number): Promise<TextContentBlock> {
  const targetBytes = Math.max(0, Math.floor(sizeKb * 1000));
  const chunk = `${LOREM}
`;
  const chunkBytes = Buffer.byteLength(chunk, 'utf8');
  const repeatCount = Math.floor(targetBytes / chunkBytes);
  const remainder = targetBytes - repeatCount * chunkBytes;
  const text = chunk.repeat(repeatCount) + ' '.repeat(remainder);

  return { type: 'text', text };
}

export async function getSineWaveAudio(seconds: number): Promise<AudioContentBlock> {
  const sampleRate = 8000;
  const channels = 1;
  const bitsPerSample = 16;
  const bytesPerSample = bitsPerSample / 8;
  const sampleCount = Math.max(0, Math.floor(seconds * sampleRate));
  const dataSize = sampleCount * channels * bytesPerSample;
  const wav = Buffer.alloc(44 + dataSize);

  wav.write('RIFF', 0, 'ascii');
  wav.writeUInt32LE(wav.length - 8, 4);
  wav.write('WAVE', 8, 'ascii');
  wav.write('fmt ', 12, 'ascii');
  wav.writeUInt32LE(16, 16);
  wav.writeUInt16LE(1, 20);
  wav.writeUInt16LE(channels, 22);
  wav.writeUInt32LE(sampleRate, 24);
  wav.writeUInt32LE(sampleRate * channels * bytesPerSample, 28);
  wav.writeUInt16LE(channels * bytesPerSample, 32);
  wav.writeUInt16LE(bitsPerSample, 34);
  wav.write('data', 36, 'ascii');
  wav.writeUInt32LE(dataSize, 40);

  for (let sample = 0; sample < sampleCount; sample += 1) {
    const t = sample / sampleRate;
    const amplitude = Math.sin(2 * Math.PI * 440 * t);
    wav.writeInt16LE(Math.round(amplitude * 32767), 44 + sample * bytesPerSample);
  }

  return { type: 'audio', data: wav.toString('base64'), mimeType: 'audio/wav' };
}
