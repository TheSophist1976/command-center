import { defineConfig, devices } from '@playwright/test';

// Each test starts its own `task serve` on a temporary database (see
// tests/fixtures.ts), so tests run in parallel without sharing state.
// Build first: `npm --prefix ../web run build && cargo build`.
export default defineConfig({
  testDir: './tests',
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? [['list'], ['html', { open: 'never' }]] : 'list',
  use: {
    ...devices['Desktop Chrome'],
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
});
