const http = require('http');
const fs = require('fs');
const path = require('path');
const { lookup } = require('mime-types');

const { readdir, stat } = fs.promises;

/**
 * Creates an HTTP server that serves static files rooted at the provided directory.
 *
 * @param {object} options
 * @param {string} options.rootDir Absolute or relative directory to serve.
 * @param {string[]} [options.indexFiles] Index files to try when a directory is requested.
 * @param {boolean} [options.enableDirectoryListing] Whether to render a simple directory listing when no index file exists.
 * @param {Console} [options.logger] Logger implementation, defaults to console.
 * @returns {http.Server}
 */
function createStaticServer({
  rootDir,
  indexFiles = ['index.html', 'index.htm'],
  enableDirectoryListing = true,
  logger = console
}) {
  if (!rootDir) {
    throw new Error('rootDir is required to create a static server');
  }

  const resolvedRoot = path.resolve(rootDir);
  const rootWithSep = resolvedRoot.endsWith(path.sep)
    ? resolvedRoot
    : `${resolvedRoot}${path.sep}`;

  const server = http.createServer(async (req, res) => {
    const startTime = Date.now();

    if (!req.url) {
      sendError(res, 400, 'Bad Request');
      return;
    }

    if (req.method !== 'GET' && req.method !== 'HEAD') {
      res.setHeader('Allow', 'GET, HEAD');
      sendError(res, 405, 'Method Not Allowed');
      logRequest(logger, req, res.statusCode, startTime);
      return;
    }

    let decodedPath;
    try {
      const url = new URL(req.url, 'http://localhost');
      decodedPath = decodeURIComponent(url.pathname);
    } catch (error) {
      sendError(res, 400, 'Bad Request');
      logRequest(logger, req, res.statusCode, startTime, error);
      return;
    }

    const candidateSegments = decodedPath
      .split('/')
      .filter(segment => segment && segment !== '.');
    const resolvedPath = path.resolve(resolvedRoot, ...candidateSegments);

    if (!resolvedPath.startsWith(rootWithSep) && resolvedPath !== resolvedRoot) {
      sendError(res, 403, 'Forbidden');
      logRequest(logger, req, res.statusCode, startTime);
      return;
    }

    let stats;
    try {
      stats = await stat(resolvedPath);
    } catch (error) {
      if (error.code === 'ENOENT') {
        sendError(res, 404, 'Not Found');
      } else {
        sendError(res, 500, 'Internal Server Error');
        logger.error('Error reading path', resolvedPath, error);
      }
      logRequest(logger, req, res.statusCode, startTime, error);
      return;
    }

    if (stats.isDirectory()) {
      if (!decodedPath.endsWith('/')) {
        // Align with browser expectations for relative asset loading.
        res.statusCode = 301;
        res.setHeader('Location', `${decodedPath}/`);
        res.end();
        logRequest(logger, req, res.statusCode, startTime);
        return;
      }

      for (const indexFile of indexFiles) {
        const candidate = path.join(resolvedPath, indexFile);
        try {
          const indexStats = await stat(candidate);
          if (indexStats.isFile()) {
            await serveFile(res, candidate, indexStats, req.method === 'HEAD');
            logRequest(logger, req, res.statusCode, startTime);
            return;
          }
        } catch (error) {
          if (error.code !== 'ENOENT') {
            logger.error('Error reading index file', candidate, error);
          }
        }
      }

      if (!enableDirectoryListing) {
        sendError(res, 403, 'Directory listing disabled');
        logRequest(logger, req, res.statusCode, startTime);
        return;
      }

      try {
        await serveDirectoryListing(
          res,
          decodedPath,
          resolvedPath,
          req.method === 'HEAD'
        );
        logRequest(logger, req, res.statusCode, startTime);
        return;
      } catch (error) {
        sendError(res, 500, 'Internal Server Error');
        logger.error('Error generating directory listing', resolvedPath, error);
        logRequest(logger, req, res.statusCode, startTime, error);
        return;
      }
    }

    if (stats.isFile()) {
      try {
        await serveFile(res, resolvedPath, stats, req.method === 'HEAD');
      } catch (error) {
        logger.error('Error serving file', resolvedPath, error);
        // Stream errors may happen after headers were sent.
      }
      logRequest(logger, req, res.statusCode, startTime);
      return;
    }

    sendError(res, 403, 'Forbidden');
    logRequest(logger, req, res.statusCode, startTime);
  });

  return server;
}

function sendError(res, statusCode, message) {
  res.statusCode = statusCode;
  res.setHeader('Content-Type', 'text/plain; charset=utf-8');
  res.setHeader('Cache-Control', 'no-store');
  res.end(`${statusCode} ${message}`);
}

async function serveFile(res, filePath, stats, headOnly) {
  const mimeType = lookup(filePath) || 'application/octet-stream';
  res.statusCode = 200;
  res.setHeader('Content-Type', mimeType);
  res.setHeader('Content-Length', stats.size);
  res.setHeader('Last-Modified', stats.mtime.toUTCString());
  res.setHeader('Cache-Control', 'no-cache');

  if (headOnly) {
    res.end();
    return;
  }

  await streamFile(res, filePath);
}

function streamFile(res, filePath) {
  return new Promise((resolve, reject) => {
    const stream = fs.createReadStream(filePath);
    stream.on('error', error => {
      if (!res.headersSent) {
        sendError(res, 500, 'Internal Server Error');
      } else {
        res.destroy(error);
      }
      reject(error);
    });
    stream.on('end', resolve);
    stream.pipe(res);
  });
}

async function serveDirectoryListing(res, requestPath, directoryPath, headOnly) {
  const entries = await readdir(directoryPath, { withFileTypes: true });
  const items = entries
    .map(entry => ({
      name: entry.name,
      isDirectory: entry.isDirectory()
    }))
    .sort((a, b) => {
      if (a.isDirectory && !b.isDirectory) return -1;
      if (!a.isDirectory && b.isDirectory) return 1;
      return a.name.localeCompare(b.name);
    });

  const detailedItems = [];
  for (const item of items) {
    const fullPath = path.join(directoryPath, item.name);
    try {
      const stats = await stat(fullPath);
      detailedItems.push({
        ...item,
        size: stats.isFile() ? stats.size : null,
        mtime: stats.mtime
      });
    } catch (error) {
      detailedItems.push({
        ...item,
        size: null,
        mtime: null
      });
    }
  }

  const tableRows = detailedItems
    .map(item => {
      const trailingSlash = item.isDirectory ? '/' : '';
      const href = encodeURIComponent(item.name) + trailingSlash;
      const displayName = `${escapeHtml(item.name)}${trailingSlash}`;
      const size = item.isDirectory ? '-' : formatBytes(item.size);
      const modified = item.mtime ? formatDate(item.mtime) : '-';
      return `<tr><td><a href="${href}">${displayName}</a></td><td>${size}</td><td>${modified}</td></tr>`;
    })
    .join('\n');

  const body = `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <title>Index of ${escapeHtml(requestPath)}</title>
    <style>
      body { font-family: sans-serif; padding: 1rem 2rem; }
      h1 { font-size: 1.5rem; margin-bottom: 1rem; }
      table { border-collapse: collapse; width: 100%; max-width: 60rem; }
      th, td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid #e1e4e8; }
      th { font-weight: 600; background: #f6f8fa; }
      td:nth-child(2), th:nth-child(2) { width: 8rem; }
      td:nth-child(3), th:nth-child(3) { width: 16rem; }
      a { text-decoration: none; color: #0366d6; }
      a:hover { text-decoration: underline; }
      @media (max-width: 600px) {
        table { font-size: 0.9rem; }
        td:nth-child(3), th:nth-child(3) { width: auto; }
      }
    </style>
  </head>
  <body>
    <h1>Index of ${escapeHtml(requestPath)}</h1>
    <table>
      <thead>
        <tr><th>Name</th><th>Size</th><th>Last Modified</th></tr>
      </thead>
      <tbody>
        ${requestPath !== '/' ? '<tr><td><a href="../">../</a></td><td>-</td><td>-</td></tr>' : ''}
        ${tableRows}
      </tbody>
    </table>
    <footer>
      <p>
        Served by <a href="https://github.com/chenpu17/serve-here" target="_blank" rel="noreferrer">serve-here</a>
      </p>
    </footer>
  </body>
</html>`;

  res.statusCode = 200;
  res.setHeader('Content-Type', 'text/html; charset=utf-8');
  res.setHeader('Cache-Control', 'no-cache');

  if (headOnly) {
    res.end();
    return;
  }

  res.end(body);
}

function escapeHtml(value) {
  return value.replace(/[&<>"']/g, character => {
    const replacements = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#39;'
    };
    return replacements[character] || character;
  });
}

function formatBytes(bytes) {
  if (bytes === null || bytes === undefined) {
    return '-';
  }
  if (bytes === 0) {
    return '0 B';
  }
  const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];
  const exponent = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1
  );
  const size = bytes / 1024 ** exponent;
  return `${size >= 10 ? size.toFixed(0) : size.toFixed(1)} ${units[exponent]}`;
}

function formatDate(date) {
  if (!date) return '-';
  return date.toLocaleString();
}

function logRequest(logger, req, statusCode, startTime, error) {
  const duration = Date.now() - startTime;
  const method = req.method;
  const url = req.url;
  const message = `${method} ${url} ${statusCode} ${duration}ms`;
  if (error) {
    logger.error(message);
  } else {
    logger.info ? logger.info(message) : logger.log(message);
  }
}

module.exports = {
  createStaticServer
};
