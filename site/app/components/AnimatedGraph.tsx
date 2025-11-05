'use client';

import { useEffect, useRef } from 'react';

interface Node {
  x: number;
  y: number;
  vx: number;
  vy: number;
  radius: number;
  connections: number[];
}

interface Connection {
  from: number;
  to: number;
}

interface Pulse {
  connectionIndex: number;
  progress: number;
  speed: number;
  intensity: number;
}

export function AnimatedGraph() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    const resizeCanvas = () => {
      canvas.width = canvas.offsetWidth;
      canvas.height = canvas.offsetHeight;
    };
    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);

    // Create galaxy structure - nodes in clusters
    const nodeCount = 50;
    const nodes: Node[] = [];
    const centerX = canvas.width / 2;
    const centerY = canvas.height / 2;

    // Create spiral galaxy pattern
    for (let i = 0; i < nodeCount; i++) {
      const angle = (i / nodeCount) * Math.PI * 4; // 2 full spirals
      const radius = (i / nodeCount) * Math.min(canvas.width, canvas.height) * 0.4;
      const spread = 50; // Random spread around spiral

      nodes.push({
        x: centerX + Math.cos(angle) * radius + (Math.random() - 0.5) * spread,
        y: centerY + Math.sin(angle) * radius + (Math.random() - 0.5) * spread,
        vx: (Math.random() - 0.5) * 0.2,
        vy: (Math.random() - 0.5) * 0.2,
        radius: Math.random() * 2 + 2,
        connections: [],
      });
    }

    // Build well-connected graph structure
    const connections: Connection[] = [];
    const minConnections = 3; // Ensure each node has at least 3 connections
    const maxDistance = Math.min(canvas.width, canvas.height) * 0.25;

    // First pass: connect nearby nodes
    for (let i = 0; i < nodes.length; i++) {
      nodes[i].connections = [];
      for (let j = i + 1; j < nodes.length; j++) {
        const dx = nodes[i].x - nodes[j].x;
        const dy = nodes[i].y - nodes[j].y;
        const distance = Math.sqrt(dx * dx + dy * dy);

        if (distance < maxDistance) {
          connections.push({ from: i, to: j });
          nodes[i].connections.push(j);
          nodes[j].connections.push(i);
        }
      }
    }

    // Second pass: ensure minimum connectivity
    for (let i = 0; i < nodes.length; i++) {
      while (nodes[i].connections.length < minConnections) {
        // Find nearest unconnected node
        let nearest = -1;
        let nearestDist = Infinity;

        for (let j = 0; j < nodes.length; j++) {
          if (i === j || nodes[i].connections.includes(j)) continue;

          const dx = nodes[i].x - nodes[j].x;
          const dy = nodes[i].y - nodes[j].y;
          const distance = Math.sqrt(dx * dx + dy * dy);

          if (distance < nearestDist) {
            nearest = j;
            nearestDist = distance;
          }
        }

        if (nearest !== -1) {
          connections.push({ from: i, to: nearest });
          nodes[i].connections.push(nearest);
          nodes[nearest].connections.push(i);
        } else {
          break;
        }
      }
    }

    // Animated pulses traveling along connections
    const pulses: Pulse[] = [];
    const maxPulses = 20;

    const spawnPulse = () => {
      if (pulses.length < maxPulses && connections.length > 0) {
        pulses.push({
          connectionIndex: Math.floor(Math.random() * connections.length),
          progress: 0,
          speed: 0.005 + Math.random() * 0.01,
          intensity: 0.6 + Math.random() * 0.4,
        });
      }
    };

    // Animation loop
    let animationFrame: number;
    let frameCount = 0;

    const animate = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      frameCount++;

      // Gentle node movement
      nodes.forEach((node) => {
        node.x += node.vx;
        node.y += node.vy;

        // Subtle edge bouncing
        if (node.x < 50 || node.x > canvas.width - 50) node.vx *= -1;
        if (node.y < 50 || node.y > canvas.height - 50) node.vy *= -1;

        node.x = Math.max(50, Math.min(canvas.width - 50, node.x));
        node.y = Math.max(50, Math.min(canvas.height - 50, node.y));
      });

      // Spawn new pulses randomly
      if (frameCount % 5 === 0 && Math.random() > 0.6) {
        spawnPulse();
      }

      // Draw connections (static lines)
      connections.forEach((conn) => {
        const fromNode = nodes[conn.from];
        const toNode = nodes[conn.to];

        ctx.beginPath();
        ctx.moveTo(fromNode.x, fromNode.y);
        ctx.lineTo(toNode.x, toNode.y);
        ctx.strokeStyle = 'rgba(59, 130, 246, 0.15)';
        ctx.lineWidth = 1;
        ctx.stroke();
      });

      // Update and draw pulses
      pulses.forEach((pulse, index) => {
        pulse.progress += pulse.speed;

        if (pulse.progress >= 1) {
          pulses.splice(index, 1);
          return;
        }

        const conn = connections[pulse.connectionIndex];
        const fromNode = nodes[conn.from];
        const toNode = nodes[conn.to];

        // Interpolate position along connection
        const x = fromNode.x + (toNode.x - fromNode.x) * pulse.progress;
        const y = fromNode.y + (toNode.y - fromNode.y) * pulse.progress;

        // Fade in/out effect
        const fadeIn = Math.min(pulse.progress * 3, 1);
        const fadeOut = Math.min((1 - pulse.progress) * 3, 1);
        const alpha = Math.min(fadeIn, fadeOut) * pulse.intensity;

        // Draw glowing pulse
        const gradient = ctx.createRadialGradient(x, y, 0, x, y, 10);
        gradient.addColorStop(0, `rgba(59, 130, 246, ${alpha})`);
        gradient.addColorStop(0.5, `rgba(139, 92, 246, ${alpha * 0.7})`);
        gradient.addColorStop(1, `rgba(59, 130, 246, 0)`);

        ctx.beginPath();
        ctx.arc(x, y, 10, 0, Math.PI * 2);
        ctx.fillStyle = gradient;
        ctx.fill();

        // Draw trailing glow along the path
        const trailLength = 0.15;
        const trailStart = Math.max(0, pulse.progress - trailLength);

        const startX = fromNode.x + (toNode.x - fromNode.x) * trailStart;
        const startY = fromNode.y + (toNode.y - fromNode.y) * trailStart;

        const trailGradient = ctx.createLinearGradient(startX, startY, x, y);
        trailGradient.addColorStop(0, 'rgba(139, 92, 246, 0)');
        trailGradient.addColorStop(1, `rgba(139, 92, 246, ${alpha * 0.6})`);

        ctx.beginPath();
        ctx.moveTo(startX, startY);
        ctx.lineTo(x, y);
        ctx.strokeStyle = trailGradient;
        ctx.lineWidth = 3;
        ctx.stroke();
      });

      // Draw nodes
      nodes.forEach((node) => {
        // Outer glow
        const glowGradient = ctx.createRadialGradient(
          node.x, node.y, 0,
          node.x, node.y, node.radius + 6
        );
        glowGradient.addColorStop(0, 'rgba(59, 130, 246, 0.6)');
        glowGradient.addColorStop(1, 'rgba(59, 130, 246, 0)');

        ctx.beginPath();
        ctx.arc(node.x, node.y, node.radius + 6, 0, Math.PI * 2);
        ctx.fillStyle = glowGradient;
        ctx.fill();

        // Node core
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(59, 130, 246, 0.8)';
        ctx.fill();

        // Bright center
        ctx.beginPath();
        ctx.arc(node.x, node.y, node.radius * 0.4, 0, Math.PI * 2);
        ctx.fillStyle = 'rgba(255, 255, 255, 0.9)';
        ctx.fill();
      });

      animationFrame = requestAnimationFrame(animate);
    };

    animate();

    return () => {
      window.removeEventListener('resize', resizeCanvas);
      cancelAnimationFrame(animationFrame);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0 w-full h-full"
      style={{ mixBlendMode: 'screen' }}
    />
  );
}
