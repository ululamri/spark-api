#!/usr/bin/env node
import { readFileSync } from 'node:fs';

const file = 'docs/LIVE_DEPLOYMENT.md';
const text = readFileSync(file, 'utf8');
const required = [
  '/opt/karyra/spark-api',
  '.env.host',
  'karyra-spark-api',
  '127.0.0.1:8787',
  'spark.user.cloudjkt01.com',
  '/health/ready'
];

const blockers = required.filter((item) => !text.includes(item));
console.log('PASS 19B Spark API live deploy audit');
if (blockers.length) {
  for (const item of blockers) console.error(`Missing ${item}`);
  process.exit(1);
}
console.log('No PASS 19B API deploy blockers found.');
