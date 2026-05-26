// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Optional dev-only default bearer token baked into the SPA (avoid in production). */
  readonly VITE_AETHER_DEFAULT_API_KEY?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

declare const __AETHER_DASHBOARD_BUILD__: string;
