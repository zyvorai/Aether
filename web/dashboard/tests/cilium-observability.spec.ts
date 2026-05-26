import { test, expect } from '@playwright/test';
import { ensureAuthenticated } from './helpers/auth';

const ciliumStatus = {
  cluster: 'kind-aether',
  namespace: 'aether-system',
  cni: 'cilium',
  crds: {
    ciliumnetworkpolicies: true,
    ciliumclusterwidenetworkpolicies: true,
  },
  cilium_daemonset_ready: true,
  managed_policies: [
    {
      name: 'allow-aether-egress',
      scope: 'namespace',
      namespace: 'aether-system',
      exists: true,
      aether_managed: true,
    },
  ],
  egress_mode: 'permissive',
  metrics_server: true,
};

function mockPlatformApis(page: import('@playwright/test').Page) {
  return Promise.all([
    page.route('**/api/server', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            version: '0.3.0',
            ha_mode: 'single',
            safety: {
              mutation_confirm_required: false,
              mutation_confirm_header: 'X-Aether-Confirm',
              mutation_confirm_values: ['1'],
            },
            opa: { configured: false, enforce: false },
            oidc: { enabled: false },
            workload_state: { backend: 'local-json', configured: true },
            integrations: {
              grafana_url: 'https://grafana.example.com',
              prometheus_url: 'https://prom.example.com',
            },
            kubernetes: { cilium: ciliumStatus },
          },
        }),
      }),
    ),
    page.route('**/api/platform/recommendations', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            items: [
              {
                id: 'prometheus-not-linked',
                category: 'observability',
                severity: 'info',
                title: 'Prometheus not linked',
                detail: 'mock',
                action: 'mock',
              },
            ],
          },
        }),
      }),
    ),
    page.route('**/api/system/ready', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ ready: true, redis: false }),
      }),
    ),
    page.route('**/api/gitops/status', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: { configured: false } }),
      }),
    ),
  ]);
}

test.describe('Cilium & observability UI', () => {
  test.beforeEach(async ({ page }) => {
    await mockPlatformApis(page);
    await ensureAuthenticated(page);
  });

  test('Platform page shows Kubernetes / Cilium card', async ({ page }) => {
    await page.goto('/platform');
    await expect(page.getByRole('heading', { name: 'Kubernetes / Cilium' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('allow-aether-egress')).toBeVisible();
    await expect(page.getByText('permissive')).toBeVisible();
    await expect(page.getByRole('link', { name: /Grafana/i })).toBeVisible();
  });

  test('Metrics page shows observability summary panels', async ({ page }) => {
    await page.route('**/api/metrics', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'text/plain',
        body: '# HELP aether_migrations_total x\naether_migrations_total 2\naether_api_http_requests_total 42\n',
      }),
    );
    await page.route('**/api/observability/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            api_http_requests_total: 42,
            migrations_total: 2,
            migration_rollbacks_total: 0,
            workloads_running: { Kube: 3 },
            cilium: ciliumStatus,
            prometheus_configured: true,
          },
        }),
      }),
    );
    await page.route('**/api/cost/chargeback**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            totalMonthlyUsd: 0,
            totalSpotMonthlyUsd: 0,
            tco36MonthsUsd: 0,
            pricingSource: 'mock',
            region: 'us-east-1',
            lines: [],
          },
        }),
      }),
    );

    await page.goto('/metrics');
    await expect(page.getByText('API requests')).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText('Workloads by runtime')).toBeVisible();
    await expect(page.getByText('Cluster & Cilium')).toBeVisible();
  });

  test('Clusters page Network tab lists policies', async ({ page }) => {
    await page.route('**/api/cluster/summary', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            enabled: true,
            connected: true,
            backend: 'kubeconfig',
            cluster_count: 1,
            healthy_clusters: 1,
            workload_count: 0,
            clusters: [{ name: 'kind-aether', server: null, version: null, reachable: true }],
            error: null,
          },
        }),
      }),
    );
    await page.route('**/api/cluster/namespaces**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: [{ name: 'aether-system', status: 'Active' }],
        }),
      }),
    );
    await page.route('**/api/cluster/metrics/summary**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          data: {
            scope: 'namespace/aether-system',
            pod_count: 1,
            total_cpu_millicores: 100,
            total_memory_mib: 128,
            pods: [],
          },
        }),
      }),
    );
    await page.route('**/api/cluster/cilium/status**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: ciliumStatus }),
      }),
    );
    await page.route('**/api/cluster/browse**', (route) => {
      const url = route.request().url();
      const kind = new URL(url).searchParams.get('kind');
      const items =
        kind === 'CiliumClusterwideNetworkPolicy'
          ? [
              {
                cluster: 'kind-aether',
                namespace: '_cluster',
                kind: 'CiliumClusterwideNetworkPolicy',
                name: 'aether-control-plane-egress',
                status: 'aether-managed',
                created_at: '2026-01-01T00:00:00Z',
                detail: 'cilium.io/v2 clusterwide',
              },
            ]
          : [
              {
                cluster: 'kind-aether',
                namespace: 'aether-system',
                kind: kind ?? 'NetworkPolicy',
                name: 'allow-aether-egress',
                status: 'aether-managed',
                created_at: '2026-01-01T00:00:00Z',
                detail: 'mock',
              },
            ];
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ success: true, data: items }),
      });
    });

    await page.goto('/clusters?tab=network');
    await expect(page.getByRole('button', { name: 'Network' })).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('button', { name: 'allow-aether-egress' }).first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole('button', { name: 'aether-control-plane-egress' })).toBeVisible();
  });
});
