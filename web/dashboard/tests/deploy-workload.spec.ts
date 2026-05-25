import { test, expect } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

test.describe('Deploy workload UX', () => {
  test.beforeEach(async ({ page }) => {
    await ensureAuthenticated(page);
  });

  test('deploy from workloads modal shows new Aether-managed workload', async ({ page }) => {
    const unique = `ux-${Date.now().toString(36)}`;

    await page.goto('/workloads?source=aether');

    await expect(page.getByRole('button', { name: 'Deploy' }).first()).toBeVisible({ timeout: 15_000 });
    await page.getByRole('button', { name: 'Deploy' }).first().click();

    const dialog = page.getByRole('dialog', { name: 'Deploy New Workload' });
    await expect(dialog).toBeVisible({ timeout: 10_000 });

    const textarea = dialog.locator('textarea');
    await expect(textarea).toHaveValue(/apiVersion: aether\/v1/);

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
    await textarea.fill(yaml);
    await dialog.getByRole('button', { name: 'Deploy' }).click();

    await expect(page.getByText(new RegExp(`Deployed "${unique}" successfully`, 'i'))).toBeVisible({
      timeout: 30_000,
    });

    await expect(dialog).not.toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole('button', { name: unique })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/deployed/i).first()).toBeVisible();
  });
});
