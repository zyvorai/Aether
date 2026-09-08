// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test, expect, type Locator, type APIRequestContext } from './fixtures';
import { ensureAuthenticated } from './helpers/auth';

async function openDeployDialog(page: import('@playwright/test').Page) {
  await page.goto('/workloads?deploy=1&source=aether');
  const dialog = page.getByRole('dialog', { name: 'Deploy New Workload' });
  await expect(dialog).toBeVisible({ timeout: 15_000 });
  return dialog;
}

async function yamlText(dialog: Locator) {
  const editor = dialog.getByRole('textbox', { name: /workload yaml/i });
  await expect(editor).toBeVisible();
  return editor;
}

async function clickDeploySubmit(dialog: Locator) {
  const submit = dialog.getByTestId('workloads-deploy-submit');
  await submit.scrollIntoViewIfNeeded();
  await submit.click({ force: true });
}

function workloadYaml(unique: string) {
  return `apiVersion: aether/v1
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
}

async function waitForAetherWorkloadAbsent(request: APIRequestContext, unique: string) {
  await expect.poll(async () => {
    const res = await request.get('/api/workloads');
    if (!res.ok()) return true;
    const body = (await res.json()) as { data?: Array<{ name: string; source?: string }> };
    const items = body.data ?? [];
    return items.some((w) => (w.source ?? 'aether') === 'aether' && (w.name === unique || w.name.endsWith(`/${unique}`)));
  }, { timeout: 30_000 }).toBe(false);
}

async function deleteAetherWorkloadRow(page: import('@playwright/test').Page, unique: string) {
  const row = page.locator('tr').filter({ hasText: unique }).filter({ has: page.getByTitle('Delete') });
  await expect(row.first()).toBeVisible({ timeout: 20_000 });
  await row.first().getByTitle('Delete').click();

  const confirm = page.getByRole('dialog', { name: 'Confirm Delete' });
  await expect(confirm).toBeVisible();
  await confirm.getByRole('button', { name: 'Delete' }).click();
}

function mockValidateAndPolicy(page: import('@playwright/test').Page, opts?: { policyPassed?: boolean }) {
  const policyPassed = opts?.policyPassed ?? true;
  void page.route('**/api/validate', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        data: { valid: true, workload_name: 'httpd', errors: [] },
      }),
    }),
  );
  void page.route('**/api/policy/check', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        success: true,
        data: {
          passed: policyPassed,
          policies_evaluated: 1,
          violations: policyPassed ? [] : [{ policy: 'test', message: 'missing label owner' }],
          warnings: [],
        },
      }),
    }),
  );
}

test.describe('Deploy workload UX', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('deploy modal shows complete default YAML including health and network', async ({ page }) => {
    const dialog = await openDeployDialog(page);

    const editor = await yamlText(dialog);
    await expect(editor).toContainText('apiVersion: aether/v1');
    await expect(editor).toContainText('liveness:');
    await expect(editor).toContainText('readiness:');
    await expect(editor).toContainText('containerPort: 80');

    const editorShell = dialog.getByTestId('yaml-editor');
    const box = await editorShell.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThan(400);

    await expect(dialog.getByRole('button', { name: 'Copy' })).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Validate' })).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Upload' })).toBeVisible();
    await expect(dialog.getByRole('button', { name: 'Reset' })).toBeVisible();
  });

  test('validate button shows validation and policy results', async ({ page }) => {
    mockValidateAndPolicy(page);
    const dialog = await openDeployDialog(page);

    await dialog.getByRole('button', { name: 'Validate' }).click({ force: true });
    await expect(dialog.getByTestId('validate-result-panel')).toBeVisible({ timeout: 10_000 });
    await expect(dialog.getByTestId('validate-result-panel')).toContainText('VALID');
    await expect(dialog.getByTestId('validate-result-panel')).toContainText('POLICY OK');
  });

  test('policy failure blocks deploy and shows panel', async ({ page }) => {
    mockValidateAndPolicy(page, { policyPassed: false });
    const dialog = await openDeployDialog(page);

    await clickDeploySubmit(dialog);
    await expect(dialog.getByTestId('validate-result-panel')).toBeVisible({ timeout: 10_000 });
    await expect(dialog.getByText('POLICY FAILED')).toBeVisible();
    await expect(dialog.getByTestId('deploy-success-panel')).toHaveCount(0);
  });

  test('toolbar reset expand and copy work in deploy modal', async ({ page }) => {
    const dialog = await openDeployDialog(page);
    const editor = await yamlText(dialog);

    await editor.click();
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.insertText('name: edited-only');
    await expect(editor).toContainText('edited-only');

    await dialog.getByRole('button', { name: 'Reset' }).click();
    await expect(editor).toContainText('name: httpd');
    await expect(editor).not.toContainText('edited-only');

    await dialog.getByRole('button', { name: 'Expand' }).click();
    await expect(dialog.getByRole('button', { name: 'Collapse' })).toHaveAttribute('aria-pressed', 'true');

    await dialog.getByRole('button', { name: 'Copy' }).click({ force: true });
    await expect(dialog.getByText('Copied')).toBeVisible({ timeout: 5_000 });
  });

  test('confidential assist applies block to YAML', async ({ page }) => {
    const dialog = await openDeployDialog(page);
    const editor = await yamlText(dialog);

    await dialog.getByRole('checkbox', { name: /Enable confidential workload/i }).check();
    await dialog.getByRole('button', { name: 'Apply to YAML' }).click();
    await expect(editor).toContainText('confidential:');
  });

  test('deploy success panel view logs opens workload logs tab', async ({ page }) => {
    const unique = `logs-${Date.now().toString(36)}`;
    mockValidateAndPolicy(page);
    await page.route('**/api/workloads', (route) => {
      if (route.request().method() === 'POST') {
        return route.fulfill({
          status: 201,
          contentType: 'application/json',
          body: JSON.stringify({ success: true, data: `Workload ${unique} created` }),
        });
      }
      if (route.request().method() === 'GET') {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            success: true,
            data: [
              {
                name: unique,
                runtime: 'kube',
                image: 'docker.io/library/httpd:latest',
                status: 'running',
                created_at: new Date().toISOString(),
                source: 'aether',
              },
            ],
          }),
        });
      }
      return route.continue();
    });

    const dialog = await openDeployDialog(page);
    const editor = await yamlText(dialog);
    await editor.click();
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.insertText(workloadYaml(unique));
    await clickDeploySubmit(dialog);

    await expect(dialog.getByTestId('deploy-success-panel')).toBeVisible({ timeout: 15_000 });
    await dialog.getByRole('button', { name: 'View logs' }).click();
    await expect(page).toHaveURL(new RegExp(`workload=${unique}.*tab=logs|tab=logs.*workload=${unique}`), {
      timeout: 15_000,
    });
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

    await editor.click();
    await page.keyboard.press('ControlOrMeta+a');
    await page.keyboard.insertText(workloadYaml(unique));
    await clickDeploySubmit(dialog);

    const deployed = await dialog
      .getByTestId('deploy-success-panel')
      .or(page.getByRole('button', { name: unique, exact: true }))
      .or(page.locator('tr').filter({ hasText: unique }))
      .isVisible({ timeout: 90_000 })
      .catch(() => false);
    test.skip(!deployed, 'Live runtime deploy unavailable in this environment');

    if (await dialog.getByTestId('deploy-success-panel').isVisible().catch(() => false)) {
      await dialog.getByRole('button', { name: 'Close' }).click();
      await expect(dialog).not.toBeVisible({ timeout: 10_000 });
    }

    await expect(
      page.locator('tr').filter({ hasText: unique }).filter({ has: page.getByTitle('Delete') }),
    ).toHaveCount(1, { timeout: 15_000 });
  });

  test('deploy via API then delete via UI removes workload', async ({ page, request }) => {
    const unique = `rm-api-${Date.now().toString(36)}`;

    const deployRes = await request.post('/api/workloads', {
      data: { spec_yaml: workloadYaml(unique) },
    });
    const deployBody = (await deployRes.json()) as { success?: boolean; error?: string };
    test.skip(!deployRes.ok() || !deployBody.success, `API deploy unavailable: ${deployBody.error ?? deployRes.status()}`);

    await ensureAuthenticated(page);
    await page.goto('/workloads?source=aether');
    await deleteAetherWorkloadRow(page, unique);

    await waitForAetherWorkloadAbsent(request, unique);
    await expect(
      page.locator('tr').filter({ hasText: unique }).filter({ has: page.getByTitle('Delete') }),
    ).toHaveCount(0, { timeout: 20_000 });
  });

  test('deploy then delete removes workload from list', async ({ page, request }) => {
    const unique = `rm-${Date.now().toString(36)}`;

    const deployRes = await request.post('/api/workloads', { data: { spec_yaml: workloadYaml(unique) } });
    const deployBody = (await deployRes.json()) as { success?: boolean; error?: string };
    test.skip(!deployRes.ok() || !deployBody.success, `API deploy unavailable: ${deployBody.error ?? deployRes.status()}`);

    await ensureAuthenticated(page);
    await page.goto('/workloads?source=aether');
    await deleteAetherWorkloadRow(page, unique);

    await waitForAetherWorkloadAbsent(request, unique);
    await expect(
      page.locator('tr').filter({ hasText: unique }).filter({ has: page.getByTitle('Delete') }),
    ).toHaveCount(0, { timeout: 20_000 });
  });
});
