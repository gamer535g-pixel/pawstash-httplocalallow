#!/usr/bin/env node

import { execSync, spawn } from 'node:child_process';

const c = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
};

// Tunnels for the Vite dev server and HMR socket on every device: a bare `adb reverse` fails when more than one is connected.
const ports = [1420, 1421];
let devices = [];
try {
  devices = execSync('adb devices', { encoding: 'utf8' })
    .split('\n')
    .slice(1)
    .map((line) => line.trim().split(/\s+/))
    .filter(([serial, state]) => serial && state === 'device')
    .map(([serial]) => serial);
} catch {
  console.warn(`${c.yellow}warn${c.reset} adb is not available; skipping port tunnels`);
}
if (devices.length === 0) {
  console.warn(`${c.yellow}warn${c.reset} no device connected yet; tunnels not set up`);
}
for (const serial of devices) {
  for (const port of ports) {
    try {
      execSync(`adb -s ${serial} reverse tcp:${port} tcp:${port}`, { stdio: 'ignore' });
      console.log(`${c.green}ok${c.reset} ${serial}: adb reverse tcp:${port} -> tcp:${port}`);
    } catch {
      console.warn(`${c.yellow}warn${c.reset} ${serial}: could not adb reverse tcp:${port}`);
    }
  }
}

console.log(`${c.cyan}info${c.reset} starting tauri android dev with --host 127.0.0.1...\n`);

const extraArgs = process.argv.slice(2);
const args = ['tauri', 'android', 'dev', '--host', '127.0.0.1', ...extraArgs];

const child = spawn(process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm', args, {
  stdio: 'inherit',
  shell: true,
});

child.on('exit', (code) => {
  process.exit(code ?? 0);
});
