// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { test, expect, type Locator } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

async function yamlText(dialog: Locator) {
  const editor = dialog.getByRole('textbox', { name: /workload yaml/i });
  await expect(editor).toBeVisible();
  return editor;
}

test.describe('Deploy workload UX', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('deploy modal shows complete default YAML including network ports', async ({ page }) => {
    await page.goto('/workloads?deploy=1&source=aether');

    const dialog = page.getByRole('dialog', { name: 'Deploy New Workload' });
    await expect(dialog).toBeVisible({ timeout: 15_000 });

    const editor = await yamlText(dialog);
    await expect(editor).toContainText('apiVersion: aether/v1');
    await expect(editor).toContainText('containerPort: 80');

    const editorShell = dialog.getByTestId('yaml-editor');
    const box = await editorShell.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThan(400);

    await expect(dialog.getByRole('button', { name: 'Copy' })).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Validate' })).toBeVisible();
  });

  test('deploy from workloads modal shows new Aether-managed workload', async ({ page }) => {
    const unique = `ux-${Date.now().toString(36)}`;

    await page.goto('/workloads?source=aether');

    await expect(page.getByRole('button', { name: 'Deploy' }).first()).toBeVisible({ timeout: 15_000 });
    await page.getByRole('button', { name: 'Deploy' }).first().click();

    const dialog = page.getByRole('dialog', { name: 'Deploy New Workload' });
    await expect(dialog).toBeVisible({ timeout: 10_000 });

    const editor = await yamlText(dialog);
    await expect(editor).toContainText('apiVersion: aether/v1');

    const yaml = `apiVersion: aether/v1
kind: Workload
metadata:
  name: ${unique}
  owner: dashboard
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 100m
  memory: 128Mi
  storage: 1Gi
runtime:
  preferred: kube
  allow:
    - kube
health:
  liveness:
    httpGet:
      path: /
      port: 80
    initialDelaySeconds: 10
    periodSeconds: 10
  readiness:
    httpGet:
      path: /
      port: 80
    initialDelaySeconds: 5
    periodSeconds: 5
network:
  service: true
  ports:
    - containerPort: 80
      servicePort: 80
      protocol: TCP
`;
    await editor.click();
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.insertText(yaml);
    await dialog.getByRole('button', { name: 'Deploy' }).click();

    await expect(
      dialog.getByTestId('deploy-success-panel').or(page.getByRole('button', { name: unique })),
    ).toBeVisible({ timeout: 60_000 });

    if (await dialog.getByTestId('deploy-success-panel').isVisible().catch(() => false)) {
      await dialog.getByRole('button', { name: 'Close' }).click();
      await expect(dialog).not.toBeVisible({ timeout: 10_000 });
    }

    await expect(page.getByRole('button', { name: unique })).toBeVisible({ timeout: 15_000 });
  });

  test('deploy via API then delete via UI removes workload', async ({ page, request }) => {
    const unique = `rm-api-${Date.now().toString(36)}`;
    const yaml = `apiVersion: aether/v1
kind: Workload
metadata:
  name: ${unique}
  owner: dashboard
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 100m
  memory: 128Mi
  storage: 1Gi
runtime:
  preferred: kube
  allow:
    - kube
network:
  service: true
  ports:
    - containerPort: 80
      servicePort: 80
      protocol: TCP
`;

    const deployRes = await request.post('/api/workloads', {
      data: { spec_yaml: yaml },
    });
    expect(deployRes.ok()).toBeTruthy();
    const deployBody = await deployRes.json();
    expect(deployBody.success).toBeTruthy();

    await ensureAuthenticated(page);
    await page.goto('/workloads?source=aether');
    await expect(page.getByRole('button', { name: unique })).toBeVisible({ timeout: 20_000 });

    const row = page.locator('tr').filter({ hasText: unique });
    await row.getByTitle('Delete').click();

    const confirm = page.getByRole('dialog', { name: 'Confirm Delete' });
    await expect(confirm).toBeVisible();
    await confirm.getByRole('button', { name: 'Delete' }).click();

    await expect(page.getByRole('button', { name: unique })).not.toBeVisible({ timeout: 20_000 });
  });

  test('deploy then delete removes workload from list', async ({ page, request }) => {
    const unique = `rm-${Date.now().toString(36)}`;
    const yaml = `apiVersion: aether/v1
kind: Workload
metadata:
  name: ${unique}
  owner: dashboard
  project: default
build:
  context: .
  dockerfile: Dockerfile
  registry: docker.io/library
  tag: latest
requirements:
  cpu: 100m
  memory: 128Mi
  storage: 1Gi
runtime:
  preferred: kube
  allow:
    - kube
network:
  service: true
  ports:
    - containerPort: 80
      servicePort: 80
      protocol: TCP
`;

    const deployRes = await request.post('/api/workloads', { data: { spec_yaml: yaml } });
    expect(deployRes.ok()).toBeTruthy();

    await ensureAuthenticated(page);
    await page.goto('/workloads?source=aether');
    await expect(page.getByRole('button', { name: unique })).toBeVisible({ timeout: 20_000 });

    const row = page.locator('tr').filter({ hasText: unique });
    await row.getByTitle('Delete').click();

    const confirm = page.getByRole('dialog', { name: 'Confirm Delete' });
    await expect(confirm).toBeVisible();
    await confirm.getByRole('button', { name: 'Delete' }).click();

    await expect(page.getByRole('button', { name: unique })).not.toBeVisible({ timeout: 20_000 });
  });
});
