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
    await expect(editor).toContainText('protocol: TCP');

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

    const successPanel = dialog.getByTestId('deploy-success-panel');
    await expect(successPanel).toBeVisible({ timeout: 30_000 });
    await expect(successPanel).toContainText(unique);
    await dialog.getByRole('button', { name: 'Close' }).click();
    await expect(dialog).not.toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole('button', { name: unique })).toBeVisible({ timeout: 15_000 });
  });
});
