const { test, expect } = require('@playwright/test');
const { spawn } = require('child_process');
const http = require('http');
const net = require('net');
const path = require('path');

const SERVER_CWD = path.join(__dirname, '..', 'src-rust');

let baseUrl;
let serverPort;
let serverProcess;
let startupError = '';

function reservePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const port = typeof address === 'object' && address ? address.port : null;
      server.close(error => {
        if (error) {
          reject(error);
          return;
        }
        resolve(port);
      });
    });
    server.once('error', reject);
  });
}

function waitForServer(url) {
  return new Promise((resolve, reject) => {
    const startedAt = Date.now();
    const onExit = code => {
      reject(new Error(`serve-here exited before becoming ready with code ${code}\n${startupError}`.trim()));
    };

    serverProcess.once('exit', onExit);

    function finish(callback) {
      serverProcess.off('exit', onExit);
      callback();
    }

    function ping() {
      const req = http.get(url, response => {
        response.resume();
        if (response.statusCode && response.statusCode < 500) {
          finish(resolve);
          return;
        }
        retry(new Error(`Unexpected status ${response.statusCode}`));
      });

      req.on('error', retry);
      req.setTimeout(2000, () => {
        req.destroy(new Error('Timed out waiting for HTTP response'));
      });
    }

    function retry(error) {
      if (Date.now() - startedAt > 60000) {
        finish(() => reject(error));
        return;
      }
      setTimeout(ping, 250);
    }

    ping();
  });
}

test.beforeAll(async () => {
  serverPort = await reservePort();
  baseUrl = `http://127.0.0.1:${serverPort}`;
  serverProcess = spawn('cargo', ['run', '--', '--host', '127.0.0.1', '--port', String(serverPort), '.'], {
    cwd: SERVER_CWD,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  serverProcess.stderr.on('data', data => {
    startupError += data.toString();
  });

  await waitForServer(`${baseUrl}/`);
  if (serverProcess.exitCode && serverProcess.exitCode !== 0) {
    throw new Error(`serve-here exited early with code ${serverProcess.exitCode}\n${startupError}`.trim());
  }
});

test.afterAll(async () => {
  if (!serverProcess || serverProcess.killed) {
    return;
  }

  await new Promise(resolve => {
    serverProcess.once('exit', () => resolve());
    serverProcess.kill('SIGTERM');
    setTimeout(() => {
      if (!serverProcess.killed) {
        serverProcess.kill('SIGKILL');
      }
      resolve();
    }, 5000);
  });
});

test('listing preferences persist into stats and back', async ({ page }) => {
  await page.goto(`${baseUrl}/`);

  await expect(page.locator('[data-role="open-dashboard"]')).toHaveText('Open stats dashboard');
  await page.getByRole('button', { name: '中文' }).click();
  await expect(page.locator('[data-role="open-dashboard"]')).toHaveText('打开统计面板');

  await page.getByRole('button', { name: '亮色' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(page.locator('html')).toHaveAttribute('data-lang', 'zh');

  await page.getByPlaceholder('按文件或文件夹名称筛选...').fill('cargo');
  const visibleRows = page.locator('tbody tr:not([hidden])');
  await expect(visibleRows).toHaveCount(3);
  await expect(visibleRows.nth(1)).toContainText('Cargo.lock');

  await page.locator('[data-role="open-dashboard"]').click();
  await expect(page).toHaveURL(`${baseUrl}/stats`);
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(page.locator('html')).toHaveAttribute('data-lang', 'zh');
  await expect(page.locator('h1')).toHaveText('服务控制台');
  await expect(page.locator('[data-role="browse-files"]')).toHaveText('浏览文件');
  await expect(page.locator('#summaryCards .summary-card').first()).toBeVisible();

  await page.locator('[data-role="browse-files"]').click();
  await expect(page).toHaveURL(`${baseUrl}/`);
  await expect(page.locator('html')).toHaveAttribute('data-lang', 'zh');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
});

test('stats page preferences persist back to listing', async ({ page }) => {
  await page.goto(`${baseUrl}/stats`);

  await expect(page.locator('h1')).toHaveText('Service control deck');
  await page.getByRole('button', { name: '中文' }).click();
  await page.getByRole('button', { name: '亮色' }).click();

  await expect(page.locator('html')).toHaveAttribute('data-lang', 'zh');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(page.locator('[data-role="browse-files"]')).toHaveText('浏览文件');

  await page.locator('[data-role="browse-files"]').click();
  await expect(page).toHaveURL(`${baseUrl}/`);
  await expect(page.locator('html')).toHaveAttribute('data-lang', 'zh');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(page.locator('[data-role="open-dashboard"]')).toHaveText('打开统计面板');
});
