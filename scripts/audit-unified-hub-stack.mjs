#!/usr/bin/env node
import { readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = process.cwd();
const required = [
  'infra/docker-compose.unified.staging.yml',
  'infra/caddy/Caddyfile',
  'config/env.unified.staging.example'
];

const blockers = [];

function exists(path) {
  try {
    statSync(path);
    return true;
  } catch {
    return false;
  }
}

function read(path) {
  return readFileSync(join(root, path), 'utf8');
}

for (const file of required) {
  if (!exists(join(root, file))) blockers.push(`Missing ${file}`);
}

if (!blockers.length) {
  const compose = read('infra/docker-compose.unified.staging.yml');
  const caddy = read('infra/caddy/Caddyfile');
  const env = read('config/env.unified.staging.example');

  const expectations = [
    ['compose spark-hub service', compose.includes('spark-hub:')],
    ['compose spark hub context', compose.includes('SPARK_HUB_CONTEXT')],
    ['compose hub base path build arg', compose.includes('PUBLIC_HUB_BASE_PATH')],
    ['compose spark hub url build arg', compose.includes('PUBLIC_SPARK_HUB_URL')],
    ['compose exposes hub port 4174', compose.includes('"4174"')],
    ['caddy handles /hub path', caddy.includes('handle_path /hub/*')],
    ['caddy redirects /hub root', caddy.includes('redir @hubRoot /hub/ 308')],
    ['caddy proxies hub container', caddy.includes('spark-hub:4174')],
    ['env contains hub context', env.includes('SPARK_HUB_CONTEXT=../../hub')],
    ['env contains hub base path', env.includes('PUBLIC_HUB_BASE_PATH=/hub')],
    ['env contains public hub url', env.includes('PUBLIC_SPARK_HUB_URL=https://spark.user.cloudjkt01.com/hub/')]
  ];

  for (const [label, ok] of expectations) {
    if (!ok) blockers.push(label);
  }

  for (const file of required) {
    const text = read(file);
    if (text.charCodeAt(0) === 92) blockers.push(`${file} starts with a leading backslash`);
  }
}

console.log('Spark unified Hub deployment audit');
console.log('==================================');
if (blockers.length) {
  console.error('\nBlockers:');
  for (const blocker of blockers) console.error(`- ${blocker}`);
  process.exit(1);
}

console.log('No hard blockers found.');
