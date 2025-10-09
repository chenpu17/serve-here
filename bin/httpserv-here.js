#!/usr/bin/env node

const fs = require('fs');
const os = require('os');
const path = require('path');
const process = require('process');

const { createStaticServer } = require('../src/server');
const pkg = require('../package.json');

const DEFAULT_PORT = 8080;
const DEFAULT_HOST = '0.0.0.0';

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

    server.on('error', error => {
      console.error('Failed to start server:', error.message);
      process.exit(1);
    });

    server.listen(port, host, () => {
      const messageLines = [
        `Serving ${rootDir}`,
        `Listening on:`,
        ...formatListeningAddresses(host, port)
      ];
      console.log(messageLines.join('\n  '));
    });

    const shutdown = () => {
      console.log('\nShutting down...');
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
    version: false
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
  console.log(`httpserv-here v${pkg.version}

Usage:
  httpserv-here [options] [directory]

Options:
  -d, --dir <path>        Directory to serve (defaults to current working directory)
  -p, --port <number>     Port to listen on (default: ${DEFAULT_PORT})
  -H, --host <address>    Hostname or IP to bind (default: ${DEFAULT_HOST})
  -h, --help              Show this help message
  -V, --version           Show version`);
}

main();
