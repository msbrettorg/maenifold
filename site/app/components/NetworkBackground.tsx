'use client';

import { useEffect, useState } from 'react';

interface NetworkBackgroundProps {
  className?: string;
}

interface Node {
  id: number;
  x: number;
  y: number;
  r: number;
}

interface Connection {
  from: Node;
  to: Node;
}

// Generate nodes with randomized positions within viewBox bounds
function generateNodes(count: number): Node[] {
  const nodes: Node[] = [];
  const padding = 100; // Keep nodes away from edges

  for (let i = 0; i < count; i++) {
    nodes.push({
      id: i,
      x: padding + Math.random() * (1920 - 2 * padding),
      y: padding + Math.random() * (1080 - 2 * padding),
      r: 4 + Math.random() * 4, // Radius between 4-8px
    });
  }

  return nodes;
}

// Create connections between nearby nodes
function generateConnections(nodes: Node[], threshold: number = 300): Connection[] {
  const connections: Connection[] = [];

  for (let i = 0; i < nodes.length; i++) {
    for (let j = i + 1; j < nodes.length; j++) {
      const dx = nodes[i].x - nodes[j].x;
      const dy = nodes[i].y - nodes[j].y;
      const distance = Math.sqrt(dx * dx + dy * dy);

      if (distance < threshold) {
        connections.push({
          from: nodes[i],
          to: nodes[j],
        });
      }
    }
  }

  return connections;
}

export function NetworkBackground({ className = '' }: NetworkBackgroundProps) {
  const [isClient, setIsClient] = useState(false);

  // Responsive node counts
  // Desktop (≥1024px): 25 nodes
  // Tablet (640-1023px): 18 nodes
  // Mobile (<640px): 12 nodes
  const [desktopNodes, setDesktopNodes] = useState<Node[]>([]);
  const [tabletNodes, setTabletNodes] = useState<Node[]>([]);
  const [mobileNodes, setMobileNodes] = useState<Node[]>([]);
  const [desktopConnections, setDesktopConnections] = useState<Connection[]>([]);
  const [tabletConnections, setTabletConnections] = useState<Connection[]>([]);
  const [mobileConnections, setMobileConnections] = useState<Connection[]>([]);

  useEffect(() => {
    setIsClient(true);
    const dNodes = generateNodes(25);
    const tNodes = generateNodes(18);
    const mNodes = generateNodes(12);

    setDesktopNodes(dNodes);
    setTabletNodes(tNodes);
    setMobileNodes(mNodes);

    setDesktopConnections(generateConnections(dNodes));
    setTabletConnections(generateConnections(tNodes));
    setMobileConnections(generateConnections(mNodes));
  }, []);

  // Render static placeholder on server to match client structure
  if (!isClient) {
    return (
      <div className={className}>
        {/* Desktop placeholder */}
        <svg
          viewBox="0 0 1920 1080"
          className="hidden lg:block w-full h-full"
          role="presentation"
          aria-hidden="true"
        >
          <defs>
            <linearGradient id="node-gradient-desktop-static" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="rgb(99, 102, 241)" stopOpacity="0.8" />
              <stop offset="100%" stopColor="rgb(139, 92, 246)" stopOpacity="0.8" />
            </linearGradient>
          </defs>
        </svg>

        {/* Tablet placeholder */}
        <svg
          viewBox="0 0 1920 1080"
          className="hidden sm:block lg:hidden w-full h-full"
          role="presentation"
          aria-hidden="true"
        >
          <defs>
            <linearGradient id="node-gradient-tablet-static" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="rgb(99, 102, 241)" stopOpacity="0.8" />
              <stop offset="100%" stopColor="rgb(139, 92, 246)" stopOpacity="0.8" />
            </linearGradient>
          </defs>
        </svg>

        {/* Mobile placeholder */}
        <svg
          viewBox="0 0 1920 1080"
          className="block sm:hidden w-full h-full"
          role="presentation"
          aria-hidden="true"
        >
          <defs>
            <linearGradient id="node-gradient-mobile-static" x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor="rgb(99, 102, 241)" stopOpacity="0.8" />
              <stop offset="100%" stopColor="rgb(139, 92, 246)" stopOpacity="0.8" />
            </linearGradient>
          </defs>
        </svg>
      </div>
    );
  }

  return (
    <div className={className}>
      {/* Desktop version */}
      <svg
        viewBox="0 0 1920 1080"
        className="hidden lg:block w-full h-full"
        role="presentation"
        aria-hidden="true"
      >
        <defs>
          <linearGradient id="node-gradient-desktop" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="rgb(99, 102, 241)" stopOpacity="0.8" />
            <stop offset="100%" stopColor="rgb(139, 92, 246)" stopOpacity="0.8" />
          </linearGradient>
        </defs>

        {/* Connections */}
        <g className="opacity-20">
          {desktopConnections.map((conn, i) => (
            <line
              key={`desktop-line-${i}`}
              x1={conn.from.x}
              y1={conn.from.y}
              x2={conn.to.x}
              y2={conn.to.y}
              stroke="url(#node-gradient-desktop)"
              strokeWidth="1"
              className="animate-float"
              style={{
                animationDelay: `${(i * 0.1) % 3}s`,
                animationDuration: `${3 + (i % 3)}s`,
              }}
            />
          ))}
        </g>

        {/* Nodes */}
        <g>
          {desktopNodes.map((node) => (
            <circle
              key={`desktop-node-${node.id}`}
              cx={node.x}
              cy={node.y}
              r={node.r}
              fill="url(#node-gradient-desktop)"
              className="animate-float"
              style={{
                animationDelay: `${(node.id * 0.15) % 3}s`,
                animationDuration: `${4 + (node.id % 2)}s`,
              }}
            />
          ))}
        </g>
      </svg>

      {/* Tablet version */}
      <svg
        viewBox="0 0 1920 1080"
        className="hidden sm:block lg:hidden w-full h-full"
        role="presentation"
        aria-hidden="true"
      >
        <defs>
          <linearGradient id="node-gradient-tablet" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="rgb(99, 102, 241)" stopOpacity="0.8" />
            <stop offset="100%" stopColor="rgb(139, 92, 246)" stopOpacity="0.8" />
          </linearGradient>
        </defs>

        {/* Connections */}
        <g className="opacity-20">
          {tabletConnections.map((conn, i) => (
            <line
              key={`tablet-line-${i}`}
              x1={conn.from.x}
              y1={conn.from.y}
              x2={conn.to.x}
              y2={conn.to.y}
              stroke="url(#node-gradient-tablet)"
              strokeWidth="1"
              className="animate-float"
              style={{
                animationDelay: `${(i * 0.1) % 3}s`,
                animationDuration: `${3 + (i % 3)}s`,
              }}
            />
          ))}
        </g>

        {/* Nodes */}
        <g>
          {tabletNodes.map((node) => (
            <circle
              key={`tablet-node-${node.id}`}
              cx={node.x}
              cy={node.y}
              r={node.r}
              fill="url(#node-gradient-tablet)"
              className="animate-float"
              style={{
                animationDelay: `${(node.id * 0.15) % 3}s`,
                animationDuration: `${4 + (node.id % 2)}s`,
              }}
            />
          ))}
        </g>
      </svg>

      {/* Mobile version */}
      <svg
        viewBox="0 0 1920 1080"
        className="block sm:hidden w-full h-full"
        role="presentation"
        aria-hidden="true"
      >
        <defs>
          <linearGradient id="node-gradient-mobile" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="rgb(99, 102, 241)" stopOpacity="0.8" />
            <stop offset="100%" stopColor="rgb(139, 92, 246)" stopOpacity="0.8" />
          </linearGradient>
        </defs>

        {/* Connections */}
        <g className="opacity-20">
          {mobileConnections.map((conn, i) => (
            <line
              key={`mobile-line-${i}`}
              x1={conn.from.x}
              y1={conn.from.y}
              x2={conn.to.x}
              y2={conn.to.y}
              stroke="url(#node-gradient-mobile)"
              strokeWidth="1"
              className="animate-float"
              style={{
                animationDelay: `${(i * 0.1) % 3}s`,
                animationDuration: `${3 + (i % 3)}s`,
              }}
            />
          ))}
        </g>

        {/* Nodes */}
        <g>
          {mobileNodes.map((node) => (
            <circle
              key={`mobile-node-${node.id}`}
              cx={node.x}
              cy={node.y}
              r={node.r}
              fill="url(#node-gradient-mobile)"
              className="animate-float"
              style={{
                animationDelay: `${(node.id * 0.15) % 3}s`,
                animationDuration: `${4 + (node.id % 2)}s`,
              }}
            />
          ))}
        </g>
      </svg>
    </div>
  );
}
