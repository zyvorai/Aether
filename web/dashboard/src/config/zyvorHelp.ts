// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { ZYVOR_COPY, ZYVOR_URL } from '../components/ZyvorBrand';

export { ZYVOR_URL, ZYVOR_COPY };

export const ZYVOR_HELP = {
  platform: ZYVOR_URL,
  docs: 'https://zyvor.dev/docs',
  docsIntro: 'https://zyvor.dev/docs/intro',
  products: 'https://zyvor.dev/docs/products',
  contact: 'https://zyvor.dev/contact',
  demo: 'https://zyvor.dev/demo',
  hypersdk: 'https://zyvor.dev/hypersdk',
  suite: 'https://zyvor.dev/docs/intro#suite-product-guides',
  sales: 'mailto:sales@zyvor.dev',
  info: 'mailto:info@zyvor.dev',
} as const;

export type ProductHelpMeta = {
  name: string;
  tagline: string;
  version: string;
  productUrl: string;
};

export const AETHER_HELP: ProductHelpMeta = {
  name: 'Aether',
  tagline: 'Universal Runtime Control Plane',
  version: '0.3.0',
  productUrl: 'https://zyvor.dev/aether',
};
