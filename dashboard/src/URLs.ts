/**
 * This file contains all localhost URLs used throughout the application.
 * URLs are organized by service type and include descriptive comments.
 */

// WebSocket URLs for real-time data
export const WEBSOCKET_URLS = {
  // Main WebSocket for general real-time updates (used in multiple components)
  MAIN_WEBSOCKET: 'ws://localhost:5000',

  // WebSocket for BraidPool DAG visualization (simulator API)
  BRAIDPOOL_DAG_WEBSOCKET: 'ws://localhost:65433/',

  
} as const;

// HTTP API URLs for data fetching
export const API_URLS = {
  
  // Miner Device Api endpoints
  MINER_DEVICE_URL: 'http://localhost:5001',
} as const;


export const EXTERNAL_LINKS = {
  // Project Info
  ABOUT: 'https://github.com/braidpool/braidpool/',
  DOCUMENTATION: 'https://github.com/braidpool/braidpool/tree/main/docs',
  CONTRIBUTE:
    'https://github.com/braidpool/braidpool/blob/main/CONTRIBUTING.md',

  // Community
  GITHUB: 'https://github.com/braidpool/braidpool',
  TWITTER: 'https://twitter.com/braidpool',
  DISCORD: 'https://discord.com/invite/pZYUDwkpPv',

  // Legal
  LICENSE: 'https://github.com/braidpool/braidpool/?tab=AGPL-3.0-1-ov-file',
} as const;
