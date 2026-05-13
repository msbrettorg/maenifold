// Lifted from modelcontextprotocol/typescript-sdk@2c0c481cb9dbfd15c8613f765c940a5f5bace94d — MIT
// Auth configuration
export type { CreateDemoAuthOptions, DemoAuth } from './auth.js';
export { createDemoAuth } from './auth.js';

// Auth server setup + demo token verifier (pass to `requireBearerAuth` from @modelcontextprotocol/express)
export type { SetupAuthServerOptions } from './authServer.js';
export { createProtectedResourceMetadataRouter, demoTokenVerifier, getAuth, setupAuthServer } from './authServer.js';
