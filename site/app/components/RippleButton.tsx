'use client';

import React, { useState } from 'react';

interface RippleButtonProps {
  children: React.ReactNode;
  className?: string;
  onClick?: () => void;
  variant?: 'primary' | 'secondary' | 'tertiary';
  href?: string;
}

interface Ripple {
  x: number;
  y: number;
  id: number;
}

export function RippleButton({
  children,
  className = '',
  onClick,
  variant = 'primary',
  href,
}: RippleButtonProps) {
  const [ripples, setRipples] = useState<Ripple[]>([]);

  const handleClick = (event: React.MouseEvent<HTMLButtonElement | HTMLAnchorElement>) => {
    const button = event.currentTarget;
    const rect = button.getBoundingClientRect();

    // Calculate click position relative to button
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    // Create new ripple with unique ID
    const newRipple: Ripple = {
      x,
      y,
      id: Date.now(),
    };

    setRipples((prev) => [...prev, newRipple]);

    // Remove ripple after animation completes (500ms)
    setTimeout(() => {
      setRipples((prev) => prev.filter((r) => r.id !== newRipple.id));
    }, 500);

    // Call original onClick handler if provided
    if (onClick) {
      onClick();
    }
  };

  // Base styles for positioning
  const baseStyles = 'relative overflow-hidden';

  // If href is provided, render as anchor
  if (href) {
    return (
      <a
        href={href}
        className={`${baseStyles} ${className}`}
        onClick={handleClick}
      >
        {children}
        {ripples.map((ripple) => (
          <span
            key={ripple.id}
            className="ripple-effect"
            style={{
              left: ripple.x,
              top: ripple.y,
            }}
          />
        ))}
        <style jsx>{`
          .ripple-effect {
            position: absolute;
            border-radius: 50%;
            background-color: rgba(255, 255, 255, 0.6);
            width: 20px;
            height: 20px;
            margin-left: -10px;
            margin-top: -10px;
            pointer-events: none;
            animation: ripple-animation 500ms ease-out;
          }

          @keyframes ripple-animation {
            0% {
              transform: scale(0);
              opacity: 1;
            }
            100% {
              transform: scale(4);
              opacity: 0;
            }
          }

          @media (prefers-reduced-motion: reduce) {
            .ripple-effect {
              animation: none;
              display: none;
            }
          }
        `}</style>
      </a>
    );
  }

  // Otherwise render as button
  return (
    <button
      className={`${baseStyles} ${className}`}
      onClick={handleClick}
    >
      {children}
      {ripples.map((ripple) => (
        <span
          key={ripple.id}
          className="ripple-effect"
          style={{
            left: ripple.x,
            top: ripple.y,
          }}
        />
      ))}
      <style jsx>{`
        .ripple-effect {
          position: absolute;
          border-radius: 50%;
          background-color: rgba(255, 255, 255, 0.6);
          width: 20px;
          height: 20px;
          margin-left: -10px;
          margin-top: -10px;
          pointer-events: none;
          animation: ripple-animation 500ms ease-out;
        }

        @keyframes ripple-animation {
          0% {
            transform: scale(0);
            opacity: 1;
          }
          100% {
            transform: scale(4);
            opacity: 0;
          }
        }

        @media (prefers-reduced-motion: reduce) {
          .ripple-effect {
            animation: none;
            display: none;
          }
        }
      `}</style>
    </button>
  );
}
