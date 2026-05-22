import { test, expect } from '@playwright/test';

test.describe('Cost & chargeback API', () => {
  test('pricing config endpoint', async ({ request }) => {
    const res = await request.get('/api/cost/pricing');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.data).toMatchObject({
      source: expect.stringMatching(/baseline|live/),
      region: expect.any(String),
    });
  });

  test('chargeback report endpoint', async ({ request }) => {
    const res = await request.get('/api/cost/chargeback?provider=aws');
    expect(res.ok()).toBeTruthy();
    const body = await res.json();
    expect(body.data).toHaveProperty('totalMonthlyUsd');
    expect(body.data).toHaveProperty('lines');
    expect(body.data).toHaveProperty('tco36MonthsUsd');
  });
});
