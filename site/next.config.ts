import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
  output: 'export',
  basePath: '/maenifold',
  images: {
    unoptimized: true,
  },
  typedRoutes: true,
}

export default nextConfig
