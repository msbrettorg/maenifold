'use client';

import React from 'react';

interface AnimatedTextProps {
  children: React.ReactNode;
  className?: string;
  gradientColors?: string[];
}

export const AnimatedText: React.FC<AnimatedTextProps> = ({
  children,
  className = '',
  gradientColors = ['#0ea5e9', '#a855f7'],
}) => {
  // Generate the gradient string from provided colors
  const gradientString = gradientColors.join(', ');
  const gradient = `linear-gradient(135deg, ${gradientString})`;

  return (
    <>
      <style>{`
        @keyframes gradient-shift {
          0%, 100% {
            background-position: 0% 50%;
          }
          50% {
            background-position: 100% 50%;
          }
        }

        .animated-text-gradient {
          background: ${gradient};
          background-size: 200% 200%;
          -webkit-background-clip: text;
          background-clip: text;
          -webkit-text-fill-color: transparent;
          animation: gradient-shift 10s ease-in-out infinite;
        }

        /* Fallback for browsers that don't support background-clip: text */
        @supports not (-webkit-background-clip: text) {
          .animated-text-gradient {
            color: #0ea5e9;
            background: none;
            -webkit-text-fill-color: initial;
            animation: none;
          }
        }

        /* Respect prefers-reduced-motion for accessibility */
        @media (prefers-reduced-motion: reduce) {
          .animated-text-gradient {
            animation: none;
            background: ${gradient};
            background-size: 100% 100%;
            -webkit-background-clip: text;
            background-clip: text;
            -webkit-text-fill-color: transparent;
            background-position: 0% 50%;
          }
        }
      `}</style>

      <span className={`animated-text-gradient ${className}`}>
        {children}
      </span>
    </>
  );
};

export default AnimatedText;
