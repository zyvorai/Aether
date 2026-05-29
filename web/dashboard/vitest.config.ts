// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
