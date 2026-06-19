#!/usr/bin/env node
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';

const expected = {
  domain: 'https://spark.user.cloudjkt01.com',
  sparkPath: '/opt/karyra/spark',
  apiPath: '/opt/karyra/spark-api',
  hubBuildPath: '/opt/karyra/hub/build',
  caddyfile: '/etc/caddy/Caddyfile',
  backendEnv: '/opt/karyra/spark-api/.env.host',
  imgproxyEnv: '/etc/karyra/imgproxy.env',
  services: ['karyra-spark-web', 'karyra-spark-api', 'karyra-imgproxy'],
  ports: [
    { name: 'frontend preview', host: '127.0.0.1', port: '4173' },
    { name: 'backend API', host: '127.0.0.1', port: '8787' },
    { name: 'imgproxy', host: '127.0.0.1', port: '8088' }
  ]
};

const ok = [];
const warnings = [];
const blockers = [];

function pass(name, detail = '') { ok.push({ name, detail }); }
function warn(name, detail = '') { warnings.push({ name, detail }); }
function block(name, detail = '') { blockers.push({ name, detail }); }
function check(condition, name, detail = '', level = 'blocker') {
  if (condition) pass(name, detail);
  else if (level === 'warning') warn(name, detail);
  else block(name, detail);
}

function read(file) {
  try { return fs.readFileSync(file, 'utf8'); } catch { return ''; }
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: options.timeout ?? 10000
  });
  return {
    status: result.status,
    stdout: result.stdout || '',
    stderr: result.stderr || '',
    error: result.error
  };
}

function commandExists(command) {
  return run('bash', ['-lc', `command -v ${command}`]).status === 0;
}

function gitShort(repoPath) {
  const result = run('git', ['-C', repoPath, 'status', '--short']);
  return result.status === 0 ? result.stdout.trim() : null;
}

function curlStatus(url, headers = []) {
  const args = ['-k', '-sS', '-o', '/dev/null', '-w', '%{http_code}', '--max-time', '12'];
  for (const header of headers) args.push('-H', header);
  args.push(url);
  const result = run('curl', args, { timeout: 15000 });
  if (result.status !== 0) return { ok: false, detail: result.stderr.trim() || result.error?.message || 'curl failed' };
  return { ok: true, detail: result.stdout.trim() };
}

function curlBody(url, headers = []) {
  const args = ['-k', '-sS', '--max-time', '12'];
  for (const header of headers) args.push('-H', header);
  args.push(url);
  const result = run('curl', args, { timeout: 15000 });
  if (result.status !== 0) return { ok: false, body: '', detail: result.stderr.trim() || result.error?.message || 'curl failed' };
  return { ok: true, body: result.stdout, detail: 'ok' };
}

function socketHasPort(sockets, port) {
  return new RegExp(`(^|[\\s:\\[]|\\]:)${port}(\\s|$)`).test(sockets);
}

console.log('PASS 22A live production surface audit');
console.log('root: ' + process.cwd());
console.log('');

check(process.cwd() === expected.apiPath, 'running from canonical backend path', process.cwd());

for (const [name, filePath] of Object.entries({
  'Spark frontend path': expected.sparkPath,
  'Spark API path': expected.apiPath,
  'Hub static build path': expected.hubBuildPath,
  Caddyfile: expected.caddyfile,
  'Backend env': expected.backendEnv,
  'imgproxy env': expected.imgproxyEnv
})) {
  check(fs.existsSync(filePath), `${name} exists`, filePath);
}

check(fs.existsSync(`${expected.sparkPath}/package.json`), 'frontend package.json exists');
check(fs.existsSync(`${expected.apiPath}/Cargo.toml`), 'backend Cargo.toml exists');
check(fs.existsSync(`${expected.hubBuildPath}/index.html`), 'hub build index exists');

const caddy = read(expected.caddyfile);
if (caddy) {
  check(caddy.includes('spark.user.cloudjkt01.com'), 'Caddyfile contains canonical domain');
  check(caddy.includes('127.0.0.1:4173') || caddy.includes('localhost:4173'), 'Caddyfile routes frontend preview port 4173');
  check(caddy.includes('127.0.0.1:8787') || caddy.includes('localhost:8787'), 'Caddyfile routes backend API port 8787');
  check(caddy.includes('127.0.0.1:8088') || caddy.includes('localhost:8088'), 'Caddyfile routes imgproxy port 8088');
  check(!caddy.includes('/home/spark'), 'Caddyfile has no old /home/spark path');
  check(!caddy.includes('/home/spark-api'), 'Caddyfile has no old /home/spark-api path');
  check(!caddy.includes('KARYRA_ADMIN_TOKEN'), 'Caddyfile does not expose admin token env name');
}

const backendEnv = read(expected.backendEnv);
if (backendEnv) {
  check(backendEnv.includes('DATABASE_URL='), 'backend env declares DATABASE_URL');
  check(backendEnv.includes('KARYRA_ADMIN_TOKEN='), 'backend env declares KARYRA_ADMIN_TOKEN');
  check(!backendEnv.includes('/home/spark'), 'backend env has no old /home/spark path');
}

const imgproxyEnv = read(expected.imgproxyEnv);
if (imgproxyEnv) {
  check(imgproxyEnv.includes('IMGPROXY_'), 'imgproxy env contains IMGPROXY settings', '', 'warning');
}

if (commandExists('systemctl')) {
  for (const service of expected.services) {
    check(run('systemctl', ['is-active', '--quiet', service]).status === 0, `service active: ${service}`);
  }
} else {
  warn('systemctl is unavailable', 'service status checks skipped');
}

if (commandExists('ss')) {
  const sockets = run('ss', ['-ltnH']).stdout;
  for (const item of expected.ports) {
    const anyBind = socketHasPort(sockets, item.port);
    const canonicalBind = sockets.includes(`${item.host}:${item.port}`);
    check(anyBind, `port listening: ${item.name}`, `:${item.port}`);
    if (anyBind && !canonicalBind) {
      warn(`port bind is not strict ${item.host}:${item.port}`, `${item.name} is listening on another bind address; accepted because service/socket is live`);
    }
  }
} else {
  warn('ss is unavailable', 'port listening checks skipped');
}

if (commandExists('curl')) {
  const localFrontend = curlStatus('http://127.0.0.1:4173/');
  check(localFrontend.ok && ['200', '301', '302', '307', '308'].includes(localFrontend.detail), 'local frontend responds', localFrontend.detail);

  const publicRoot = curlStatus(`${expected.domain}/`);
  check(publicRoot.ok && ['200', '301', '302', '307', '308'].includes(publicRoot.detail), 'public root responds', publicRoot.detail);

  const publicHub = curlStatus(`${expected.domain}/hub`);
  check(publicHub.ok && ['200', '301', '302', '307', '308'].includes(publicHub.detail), 'public /hub responds', publicHub.detail, 'warning');

  const publicHubResources = curlStatus(`${expected.domain}/hub/resources`);
  check(publicHubResources.ok && ['200', '301', '302', '307', '308'].includes(publicHubResources.detail), 'public /hub/resources responds', publicHubResources.detail, 'warning');

  const publicHubMissions = curlStatus(`${expected.domain}/hub/missions`);
  check(publicHubMissions.ok && ['200', '301', '302', '307', '308'].includes(publicHubMissions.detail), 'public /hub/missions responds', publicHubMissions.detail, 'warning');

  const noAuthAudit = curlStatus('http://127.0.0.1:8787/api/admin/audit/events?limit=1');
  check(noAuthAudit.ok && noAuthAudit.detail === '401', 'admin audit rejects unauthenticated request', noAuthAudit.detail);

  const token = process.env.KARYRA_ADMIN_TOKEN?.trim();
  if (token) {
    const auditScope = curlBody('http://127.0.0.1:8787/api/admin/audit/scope', [`x-karyra-admin-token: ${token}`]);
    check(auditScope.ok && auditScope.body.includes('"ok":true'), 'admin audit scope responds with superadmin token');

    const auditEvents = curlBody('http://127.0.0.1:8787/api/admin/audit/events?limit=1', [`x-karyra-admin-token: ${token}`]);
    check(auditEvents.ok && auditEvents.body.includes('"ok":true') && auditEvents.body.includes('"items"'), 'admin audit events respond with superadmin token');
  } else {
    warn('KARYRA_ADMIN_TOKEN not loaded in environment', 'run set -a; source .env.host; set +a before this audit');
  }
} else {
  warn('curl is unavailable', 'HTTP smoke checks skipped');
}

for (const [name, repoPath] of Object.entries({ frontend: expected.sparkPath, backend: expected.apiPath })) {
  const status = gitShort(repoPath);
  if (status === null) warn(`${name} git status unavailable`, repoPath);
  else if (status.length === 0) pass(`${name} git working tree clean`);
  else warn(`${name} git working tree has local changes`, status.split('\n').slice(0, 8).join(' | '));
}

console.log('OK:');
for (const item of ok) console.log(`  OK  ${item.name}${item.detail ? ` — ${item.detail}` : ''}`);

if (warnings.length > 0) {
  console.log('');
  console.log('Warnings:');
  for (const item of warnings) console.log(`  WARN  ${item.name}${item.detail ? ` — ${item.detail}` : ''}`);
}

if (blockers.length > 0) {
  console.log('');
  console.log('Blockers:');
  for (const item of blockers) console.log(`  FAIL  ${item.name}${item.detail ? ` — ${item.detail}` : ''}`);
  console.log('');
  console.log('PASS 22A FAILED');
  process.exit(1);
}

console.log('');
console.log('PASS 22A OK');
