// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

/**
 * Geometry of the Aether identity. Original, hand-constructed from circles and
 * straight lines; no font is used or embedded. The files in `public/` repeat
 * MARK_PATH, and `geometry.test.ts` fails if they drift apart.
 */

/**
 * The mark: an AE in one gesture. One quarter circle (one spec) carries three
 * arms (three runtimes). Drawn on a 16-unit grid scaled to a 64 box, so edges
 * stay on whole pixels at any multiple of 16px. Fill with `evenodd`.
 */
export const MARK_PATH =
  'M4 56A48 48 0 0 1 52 8H60V16H44V28H56V36H44V48H60V56H36V36H17.359A40 40 0 0 0 12 56ZM36 28V19.339A40 40 0 0 0 23.434 28Z';
export const MARK_VIEWBOX = '0 0 64 64';
/** Ink bounds inside the 64 box, for optical alignment next to type. */
export const MARK_TIGHT_VIEWBOX = '4 8 56 48';

/**
 * "aether": x-height 100, ascender 140, baseline at y=100. Each entry is one
 * filled shape; fill each with `evenodd`.
 */
export const WORDMARK_PATHS: readonly string[] = [
  'M0 50a51 51 0 1 0 102 0a51 51 0 1 0 -102 0ZM19 50a32 34 0 1 1 64 0a32 34 0 1 1 -64 0Z',
  'M83 0H102V100H83Z',
  'M210.189 81.399A51 51 0 1 1 220.517 57H138.686A32 34 0 0 0 195.78 70.142ZM139.415 40A32 34 0 0 1 200.585 40Z',
  'M242 -24H261V0H285V17H261V66A15 17 0 0 0 276 83H287V100H276A34 34 0 0 1 242 66V17H230V0H242Z',
  'M302 -40H321V100H302Z',
  'M302 44A45 45 0 0 1 392 44V100H373V44A26 28 0 0 0 321 44V100H302Z',
  'M498.189 81.399A51 51 0 1 1 508.517 57H426.686A32 34 0 0 0 483.78 70.142ZM427.415 40A32 34 0 0 1 488.585 40Z',
  'M524 0H543V100H524Z',
  'M524 45A46 46 0 0 1 570 -1H580V16H570A27 29 0 0 0 543 45V100H524Z',
];
export const WORDMARK_VIEWBOX = '0 -40 580 144';

/** App icon tile gradient, top to bottom. */
export const ICON_GRADIENT = { from: '#2f9bff', to: '#0055cc' } as const;
/** Places the mark inside the 64 app icon tile, optically centred. */
export const ICON_MARK_TRANSFORM = 'translate(9.98 10.38) scale(0.66)';
