// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

/** Pixel height for YAML editor from line count; capped unless expanded. */
export function yamlEditorHeightPx(lineCount: number, expanded: boolean): number {
  const LINE_PX = 24;
  const PADDING_PX = 32;
  const MIN_PX = 280;

  if (expanded) {
    return Math.floor(window.innerHeight * 0.85);
  }

  const content = Math.max(lineCount, 1) * LINE_PX + PADDING_PX;
  const cap = Math.floor(window.innerHeight * 0.65);
  return Math.min(Math.max(content, MIN_PX), cap);
}

export function countYamlLines(text: string): number {
  if (!text) return 1;
  return text.split('\n').length;
}
