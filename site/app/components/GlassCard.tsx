'use client';

import { useRef, useState, useEffect } from 'react';
import './GlassCard.css';

interface GlassCardProps {
  children: React.ReactNode;
  className?: string;
}

export function GlassCard({ children, className = '' }: GlassCardProps) {
  const cardRef = useRef<HTMLDivElement>(null);
  const [isTouch, setIsTouch] = useState(false);
  const [tiltX, setTiltX] = useState(0);
  const [tiltY, setTiltY] = useState(0);

  // Detect touch device capability on mount
  useEffect(() => {
    // Check for touch capability
    const isTouchDevice = () => {
      return (
        typeof window !== 'undefined' &&
        navigator.maxTouchPoints > 0
      );
    };

    setIsTouch(isTouchDevice());
  }, []);

  // Handle 3D tilt effect on mousemove (desktop only)
  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (isTouch || !cardRef.current) return;

    const card = cardRef.current;
    const rect = card.getBoundingClientRect();

    // Calculate position relative to card center
    const centerX = rect.width / 2;
    const centerY = rect.height / 2;
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;

    // Calculate rotation angles (max ±15 degrees)
    const rotateY = ((mouseX - centerX) / centerX) * 15;
    const rotateX = -((mouseY - centerY) / centerY) * 15;

    setTiltX(rotateX);
    setTiltY(rotateY);
  };

  // Reset tilt when mouse leaves
  const handleMouseLeave = () => {
    setTiltX(0);
    setTiltY(0);
  };

  const tiltStyle: React.CSSProperties = !isTouch
    ? {
        transform: `perspective(1000px) rotateX(${tiltX}deg) rotateY(${tiltY}deg)`,
      }
    : {};

  return (
    <div
      ref={cardRef}
      className={`glass-card ${className}`}
      onMouseMove={handleMouseMove}
      onMouseLeave={handleMouseLeave}
      style={tiltStyle}
    >
      {children}
    </div>
  );
}
