#!/usr/bin/env node
// Records a cinematic MP4 walkthrough of a running Aether dashboard: navigates the
// primary nav (Overview → Fleet → Fabric → Workloads → AI Studio → Migrations →
// Security → Cost → GitOps → back to Overview), overlays branded captions, and
// encodes the capture to two MP4s (site + LinkedIn 1080p) via ffmpeg.
//
// Usage:
//   node scripts/record-demo-reel.mjs http://<host>:<port>
//
// Requires: `npm run test:e2e:install` once (installs the Chromium build this repo's
// playwright-core resolves to) and `ffmpeg` on PATH.
//
// Headless Chromium's built-in video recorder (CDP screencast) can silently capture
// solid-color frames on some hosts instead of real composited output — this script
// launches headed (a visible window) specifically to avoid that failure mode.
import { chromium } from 'playwright-core';
import { mkdirSync, writeFileSync, readdirSync, unlinkSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';

const BASE = (process.argv.find((a) => a.startsWith('http')) || 'http://localhost:5090').replace(/\/$/, '');
const OUT = process.env.DEMO_REEL_OUT || join(tmpdir(), 'aether-demo-reel');
const RAW = join(OUT, 'raw');
const SHOTS = join(OUT, 'shots');
const FINAL_WEB = join(OUT, 'out', 'aether-client-wow-reel.mp4');
const FINAL_LI = join(OUT, 'out', 'aether-client-wow-reel-linkedin-1080p.mp4');

mkdirSync(RAW, { recursive: true });
mkdirSync(SHOTS, { recursive: true });
mkdirSync(join(OUT, 'out'), { recursive: true });

async function ensureCaptionCss(page) {
  await page
    .addStyleTag({
      content: `
      #aether-demo-caption {
        position: fixed !important; left: 50% !important; bottom: 32px !important;
        transform: translateX(-50%) !important; z-index: 2147483647 !important;
        max-width: min(980px, 92vw) !important; padding: 16px 24px !important;
        border-radius: 16px !important; background: rgba(10, 8, 6, 0.9) !important;
        color: #f6f4f1 !important; font-family: "IBM Plex Sans", "Segoe UI", system-ui, sans-serif !important;
        font-size: 22px !important; font-weight: 560 !important; letter-spacing: 0.01em !important;
        line-height: 1.35 !important; text-align: center !important;
        box-shadow: 0 16px 48px rgba(0,0,0,0.5) !important;
        border: 1px solid rgba(211, 84, 0, 0.45) !important;
        pointer-events: none !important; backdrop-filter: blur(12px) !important;
      }
      #aether-demo-caption-brand {
        font-size: 11px !important; letter-spacing: 0.16em !important;
        text-transform: uppercase !important; color: #ff9d52 !important;
        margin-bottom: 6px !important; font-weight: 700 !important;
      }
    `,
    })
    .catch(() => {});
}

async function caption(page, text, holdMs = 2600) {
  await ensureCaptionCss(page);
  await page.evaluate((msg) => {
    let bar = document.getElementById('aether-demo-caption');
    if (!bar) {
      bar = document.createElement('div');
      bar.id = 'aether-demo-caption';
      bar.innerHTML =
        '<div id="aether-demo-caption-brand">Aether · Client Demo</div><div id="aether-demo-caption-text"></div>';
      document.documentElement.appendChild(bar);
    }
    const body = document.getElementById('aether-demo-caption-text');
    if (body) body.textContent = msg;
  }, text);
  console.log(`[caption] ${text}`);
  await page.waitForTimeout(holdMs);
}

async function hideAgents(page) {
  const btn = page.getByRole('button', { name: /^HIDE AGENTS$/i }).first();
  if (await btn.isVisible().catch(() => false)) await btn.click().catch(() => {});
}

async function dismissNoise(page) {
  for (const name of ['Dismiss', 'Skip', 'Got it', 'Close tour', 'Not now', 'Close']) {
    const b = page.getByRole('button', { name: new RegExp(`^${name}$`, 'i') }).first();
    if (await b.isVisible().catch(() => false)) await b.click().catch(() => {});
  }
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
}

async function gotoPath(page, path) {
  await page.goto(`${BASE}${path}`, { waitUntil: 'domcontentloaded', timeout: 90000 });
  await page.waitForTimeout(1400);
  await dismissNoise(page);
  await hideAgents(page);
  await ensureCaptionCss(page);
}

async function smoothScroll(page, amount) {
  const steps = 10;
  for (let i = 0; i < steps; i++) {
    await page.mouse.wheel(0, amount / steps);
    await page.waitForTimeout(60);
  }
}

function encode(srcWebm, dest, w, h) {
  const ff = spawnSync(
    'ffmpeg',
    [
      '-y',
      '-i',
      srcWebm,
      '-vf',
      `scale=${w}:${h}:force_original_aspect_ratio=decrease,pad=${w}:${h}:(ow-iw)/2:(oh-ih)/2,format=yuv420p`,
      '-c:v',
      'libx264',
      '-preset',
      'medium',
      '-crf',
      '19',
      '-movflags',
      '+faststart',
      '-an',
      dest,
    ],
    { encoding: 'utf8' },
  );
  if (ff.status !== 0) {
    console.error(ff.stderr?.slice(-1500));
    throw new Error(`ffmpeg failed → ${dest}`);
  }
  return dest;
}

async function main() {
  console.log(`[wow] base=${BASE}`);

  for (const f of readdirSync(RAW)) {
    try {
      unlinkSync(join(RAW, f));
    } catch {
      /* ignore */
    }
  }

  const browser = await chromium.launch({ headless: false });
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
    viewport: { width: 1440, height: 900 },
    recordVideo: { dir: RAW, size: { width: 1440, height: 900 } },
  });
  const page = await context.newPage();
  const t0 = Date.now();

  // —— Act 1: Arrive ——
  await gotoPath(page, '/');
  await caption(page, 'Aether — the Universal Runtime Control Plane.', 3000);
  await caption(page, 'One YAML spec. Podman, Kubernetes, KubeVirt, and Metal3.', 3200);
  await page.screenshot({ path: join(SHOTS, '01-overview.png') });

  // —— Act 2: Fleet ——
  await gotoPath(page, '/fleet');
  await caption(page, 'Fleet Overview — multi-cluster inventory, live.', 2800);
  await smoothScroll(page, 400);
  await page.screenshot({ path: join(SHOTS, '02-fleet.png') });

  // —— Act 3: Runtime Fabric ——
  await gotoPath(page, '/fabric');
  await caption(page, 'Runtime Fabric — a live digital twin of your infrastructure.', 3000);
  const simBtn = page.getByRole('button', { name: /Run simulation/i }).first();
  if (await simBtn.isVisible().catch(() => false)) {
    await caption(page, 'Simulate scale, cost, and risk before you touch production.', 2800);
  }
  await page.screenshot({ path: join(SHOTS, '03-fabric.png') });

  // —— Act 4: Workloads ——
  await gotoPath(page, '/workloads');
  await caption(page, 'Workloads — cards-first ops deck for your entire runtime inventory.', 3200);
  await smoothScroll(page, 700);
  await page.waitForTimeout(600);
  const infoBtn = page.getByRole('button', { name: /^Info$/i }).first();
  if (await infoBtn.isVisible().catch(() => false)) {
    await caption(page, 'Open Info, stream Logs, or Exec — without leaving the grid.', 2800);
    await infoBtn.click().catch(() => {});
    await page.waitForTimeout(1200);
    await hideAgents(page);
    const logsTab = page.getByRole('button', { name: /^Logs$/i }).first();
    if (await logsTab.isVisible().catch(() => false)) {
      await logsTab.click().catch(() => {});
      await page.waitForTimeout(1000);
      await caption(page, 'Live logs, drift detection, and AI scoring — one panel.', 3000);
    }
  }
  await page.screenshot({ path: join(SHOTS, '04-workloads.png') });

  // —— Act 5: Zeus / AI Studio ——
  await gotoPath(page, '/zeus');
  await caption(page, 'Zeus — multi-agent AI infrastructure intelligence.', 3000);
  await caption(page, 'Ask about health, cost, migrations, or security in plain English.', 3200);
  await page.screenshot({ path: join(SHOTS, '05-zeus.png') });

  // —— Act 6: Migrations ——
  await gotoPath(page, '/migrations');
  await caption(page, 'Migrations — AI-planned moves with risk analysis and strategy.', 3000);
  await page.screenshot({ path: join(SHOTS, '06-migrations.png') });

  // —— Act 7: Security ——
  await gotoPath(page, '/security');
  await caption(page, 'Security Center — threats, secrets, policy, and hardening.', 2800);
  await page.screenshot({ path: join(SHOTS, '07-security.png') });

  // —— Act 8: Cost ——
  await gotoPath(page, '/cost');
  await caption(page, 'FinOps recommendations — right-size before you overspend.', 2800);
  await page.screenshot({ path: join(SHOTS, '08-cost.png') });

  // —— Act 9: GitOps ——
  await gotoPath(page, '/gitops');
  await caption(page, 'GitOps — reconciliation status, drift-free by design.', 2800);
  await page.screenshot({ path: join(SHOTS, '09-gitops.png') });

  // —— Close ——
  await gotoPath(page, '/');
  await caption(page, 'Aether — one spec, any runtime, zero lock-in.', 3200);
  await caption(page, 'zyvor.dev  ·  Book a demo', 3600);
  await page.screenshot({ path: join(SHOTS, '10-close.png') });

  await page.close();
  await context.close();
  await browser.close();

  const webms = readdirSync(RAW).filter((f) => f.endsWith('.webm'));
  if (!webms.length) throw new Error('no webm recorded');
  const src = join(RAW, webms[0]);
  console.log(`[wow] encoding ${src}`);
  encode(src, FINAL_WEB, 1440, 900);
  encode(src, FINAL_LI, 1920, 1080);

  const elapsed = ((Date.now() - t0) / 1000).toFixed(1);
  const meta = {
    base: BASE,
    elapsedSec: Number(elapsed),
    websiteMp4: FINAL_WEB,
    linkedInMp4: FINAL_LI,
    shots: SHOTS,
    recordedAt: new Date().toISOString(),
  };
  writeFileSync(join(OUT, 'out', 'manifest.json'), JSON.stringify(meta, null, 2));
  console.log(JSON.stringify(meta, null, 2));
  console.log(`[wow] DONE in ${elapsed}s`);
}

main().catch((e) => {
  console.error('[wow] FAIL', e);
  process.exit(1);
});
