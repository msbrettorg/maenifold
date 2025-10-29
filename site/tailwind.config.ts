import type { Config } from 'tailwindcss';

const config: Config = {
  content: [
    './app/**/*.{js,ts,jsx,tsx,mdx}',
    './components/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
    extend: {
      colors: {
        primary: '#0ea5e9',      // Sky Blue
        secondary: '#a855f7',    // Purple
        accent: '#06b6d4',       // Cyan
      },
    },
  },
};

export default config;
