#!/usr/bin/env node

const fs = require('fs');
const os = require('os');
const path = require('path');
const process = require('process');
const { spawn } = require('child_process');

const { createStaticServer } = require('../src/server');
const pkg = require('../package.json');

const DEFAULT_PORT = 8080;
const DEFAULT_HOST = '0.0.0.0';
const PID_DIR = path.join(os.homedir(), '.serve-here');
const LOG_DIR = path.join(os.homedir(), '.serve-here', 'logs');

function ensureDirectories() {
  if (!fs.existsSync(PID_DIR)) {
    fs.mkdirSync(PID_DIR, { recursive: true });
  }
  if (!fs.existsSync(LOG_DIR)) {
    fs.mkdirSync(LOG_DIR, { recursive: true });
  }
}

function getPidFile(port) {
  return path.join(PID_DIR, `serve-here-${port}.pid`);
}

function getLogFile(port) {
  return path.join(LOG_DIR, `serve-here-${port}.log`);
}

function readPidFile(port) {
  const pidFile = getPidFile(port);
  if (fs.existsSync(pidFile)) {
    try {
      const content = fs.readFileSync(pidFile, 'utf8').trim();
      const lines = content.split('\n');
      const pid = parseInt(lines[0], 10);
      const rootDir = lines[1] || '';
      return { pid, rootDir };
    } catch (error) {
      return null;
    }
  }
  return null;
}

function writePidFile(port, pid, rootDir) {
  const pidFile = getPidFile(port);
  fs.writeFileSync(pidFile, `${pid}\n${rootDir}`);
}

function removePidFile(port) {
  const pidFile = getPidFile(port);
  if (fs.existsSync(pidFile)) {
    fs.unlinkSync(pidFile);
  }
}

function isProcessRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return false;
  }
}

function startDaemon(options) {
  ensureDirectories();

  const rootDir = path.resolve(options.directory || process.cwd());
  const port = options.port || DEFAULT_PORT;
  const host = options.host || DEFAULT_HOST;

  // Check if already running
  const pidInfo = readPidFile(port);
  if (pidInfo && isProcessRunning(pidInfo.pid)) {
    console.error(`Error: Server already running on port ${port} (PID: ${pidInfo.pid})`);
    console.error(`Serving: ${pidInfo.rootDir}`);
    process.exit(1);
  }

  // Validate directory
  let stats;
  try {
    stats = fs.statSync(rootDir);
  } catch (error) {
    console.error(`Error: directory "${rootDir}" does not exist or is not accessible.`);
    process.exit(1);
  }
  if (!stats.isDirectory()) {
    console.error(`Error: path "${rootDir}" is not a directory.`);
    process.exit(1);
  }

  const logFile = getLogFile(port);
  const out = fs.openSync(logFile, 'a');
  const err = fs.openSync(logFile, 'a');

  const child = spawn(process.execPath, [__filename, '--daemon-child', '-d', rootDir, '-p', String(port), '-H', host], {
    detached: true,
    stdio: ['ignore', out, err],
    env: process.env
  });

  child.unref();

  writePidFile(port, child.pid, rootDir);

  console.log(`Server started in background (PID: ${child.pid})`);
  console.log(`Serving: ${rootDir}`);
  console.log(`Listening on port: ${port}`);
  console.log(`Log file: ${logFile}`);
  console.log(`\nTo stop: serve-here --stop -p ${port}`);
}

function stopDaemon(port) {
  const pidInfo = readPidFile(port);
  if (!pidInfo) {
    console.error(`No server running on port ${port}`);
    process.exit(1);
  }

  if (!isProcessRunning(pidInfo.pid)) {
    console.log(`Server (PID: ${pidInfo.pid}) is not running, cleaning up...`);
    removePidFile(port);
    process.exit(0);
  }

  try {
    process.kill(pidInfo.pid, 'SIGTERM');
    console.log(`Stopping server (PID: ${pidInfo.pid})...`);

    // Wait for process to stop
    let attempts = 0;
    const checkInterval = setInterval(() => {
      attempts++;
      if (!isProcessRunning(pidInfo.pid)) {
        clearInterval(checkInterval);
        removePidFile(port);
        console.log('Server stopped.');
        process.exit(0);
      } else if (attempts >= 10) {
        clearInterval(checkInterval);
        console.error('Server did not stop gracefully, force killing...');
        try {
          process.kill(pidInfo.pid, 'SIGKILL');
        } catch (e) {}
        removePidFile(port);
        process.exit(0);
      }
    }, 500);
  } catch (error) {
    console.error(`Failed to stop server: ${error.message}`);
    removePidFile(port);
    process.exit(1);
  }
}

function showStatus(port) {
  if (port) {
    const pidInfo = readPidFile(port);
    if (!pidInfo) {
      console.log(`No server running on port ${port}`);
      return;
    }

    const running = isProcessRunning(pidInfo.pid);
    console.log(`Port ${port}:`);
    console.log(`  PID: ${pidInfo.pid}`);
    console.log(`  Status: ${running ? 'running' : 'stopped'}`);
    console.log(`  Directory: ${pidInfo.rootDir}`);
    console.log(`  Log: ${getLogFile(port)}`);

    if (!running) {
      removePidFile(port);
    }
  } else {
    // Show all
    ensureDirectories();
    const files = fs.readdirSync(PID_DIR).filter(f => f.endsWith('.pid'));

    if (files.length === 0) {
      console.log('No servers running.');
      return;
    }

    console.log('Running servers:\n');
    for (const file of files) {
      const match = file.match(/serve-here-(\d+)\.pid/);
      if (match) {
        const p = parseInt(match[1], 10);
        const pidInfo = readPidFile(p);
        if (pidInfo) {
          const running = isProcessRunning(pidInfo.pid);
          console.log(`Port ${p}:`);
          console.log(`  PID: ${pidInfo.pid}`);
          console.log(`  Status: ${running ? 'running' : 'stopped'}`);
          console.log(`  Directory: ${pidInfo.rootDir}`);
          console.log('');

          if (!running) {
            removePidFile(p);
          }
        }
      }
    }
  }
}

function main() {
  try {
    const options = parseArguments(process.argv.slice(2));

    if (options.help) {
      printHelp();
      process.exit(0);
    }

    if (options.version) {
      console.log(pkg.version);
      process.exit(0);
    }

    // Handle daemon commands
    if (options.stop) {
      stopDaemon(options.port || DEFAULT_PORT);
      return;
    }

    if (options.status) {
      showStatus(options.port);
      return;
    }

    if (options.daemon && !options.daemonChild) {
      startDaemon(options);
      return;
    }

    const rootDir = path.resolve(options.directory || process.cwd());

    let stats;
    try {
      stats = fs.statSync(rootDir);
    } catch (error) {
      console.error(`Error: directory "${rootDir}" does not exist or is not accessible.`);
      process.exit(1);
    }

    if (!stats.isDirectory()) {
      console.error(`Error: path "${rootDir}" is not a directory.`);
      process.exit(1);
    }

    const server = createStaticServer({ rootDir });
    const port = options.port || DEFAULT_PORT;
    const host = options.host || DEFAULT_HOST;

    if (options.daemonChild) {
      ensureDirectories();
      process.on('exit', () => removePidFile(port));
    }

    server.on('error', error => {
      console.error('Failed to start server:', error.message);
      if (options.daemonChild) {
        removePidFile(port);
      }
      process.exit(1);
    });

    server.listen(port, host, () => {
      const messageLines = [
        `Serving ${rootDir}`,
        `Listening on:`,
        ...formatListeningAddresses(host, port)
      ];
      console.log(messageLines.join('\n  '));

      // Write PID file when running as daemon child
      if (options.daemonChild) {
        writePidFile(port, process.pid, rootDir);
      }
    });

    const shutdown = () => {
      console.log('\nShutting down...');
      if (options.daemonChild) {
        removePidFile(port);
      }
      server.close(() => process.exit(0));
    };

    process.on('SIGINT', shutdown);
    process.on('SIGTERM', shutdown);
  } catch (error) {
    console.error(error.message);
    process.exit(1);
  }
}

function parseArguments(args) {
  const options = {
    directory: undefined,
    port: undefined,
    host: undefined,
    help: false,
    version: false,
    daemon: false,
    daemonChild: false,
    stop: false,
    status: false
  };

  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];

    switch (arg) {
      case '-h':
      case '--help':
        options.help = true;
        break;
      case '-V':
      case '--version':
        options.version = true;
        break;
      case '-d':
      case '--dir':
      case '--directory':
        options.directory = requireValue(args, ++i, '--dir');
        break;
      case '-p':
      case '--port':
        options.port = parsePort(requireValue(args, ++i, '--port'));
        break;
      case '-H':
      case '--host':
        options.host = requireValue(args, ++i, '--host');
        break;
      case '-D':
      case '--daemon':
        options.daemon = true;
        break;
      case '--daemon-child':
        options.daemonChild = true;
        break;
      case '--stop':
        options.stop = true;
        break;
      case '--status':
        options.status = true;
        break;
      default:
        if (arg.startsWith('-')) {
          throw new Error(`Unknown option: ${arg}`);
        }
        if (options.directory) {
          throw new Error('Multiple directories specified. Use --dir <path> to set the directory.');
        }
        options.directory = arg;
        break;
    }
  }

  return options;
}

function requireValue(args, index, flagName) {
  const value = args[index];
  if (value === undefined) {
    throw new Error(`Missing value for ${flagName}`);
  }
  return value;
}

function parsePort(value) {
  const port = Number.parseInt(value, 10);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`Invalid port: ${value}`);
  }
  return port;
}

function formatListeningAddresses(host, port) {
  const addresses = [];

  if (host === '0.0.0.0' || host === '::') {
    addresses.push(`http://localhost:${port}/`);
    const interfaces = os.networkInterfaces();
    for (const iface of Object.values(interfaces)) {
      if (!iface) continue;
      for (const addrInfo of iface) {
        if (addrInfo.internal) continue;
        if (addrInfo.family !== 'IPv4' && addrInfo.family !== 4) continue;
        addresses.push(`http://${addrInfo.address}:${port}/`);
      }
    }
  } else {
    addresses.push(`http://${host}:${port}/`);
  }

  return [...new Set(addresses)];
}

function printHelp() {
  console.log(`serve-here v${pkg.version}

Usage:
  serve-here [options] [directory]

Options:
  -d, --dir <path>        Directory to serve (defaults to current working directory)
  -p, --port <number>     Port to listen on (default: ${DEFAULT_PORT})
  -H, --host <address>    Hostname or IP to bind (default: ${DEFAULT_HOST})
  -D, --daemon            Run as a background daemon (does not occupy terminal)
  --stop                  Stop a running daemon (use with -p to specify port)
  --status                Show status of running daemon(s)
  -h, --help              Show this help message
  -V, --version           Show version

Examples:
  serve-here                      Start server in foreground on port 8080
  serve-here -D                   Start server as daemon on port 8080
  serve-here -D -p 3000           Start daemon on port 3000
  serve-here --stop               Stop daemon on port 8080
  serve-here --stop -p 3000       Stop daemon on port 3000
  serve-here --status             Show all running daemons`);
}

main();
