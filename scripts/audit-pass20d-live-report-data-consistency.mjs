#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

const ok = [];
const warnings = [];
const blockers = [];

function pass(name, detail = '') {
  ok.push({ name, detail });
}

function warn(name, detail = '') {
  warnings.push({ name, detail });
}

function block(name, detail = '') {
  blockers.push({ name, detail });
}

function psql(sql) {
  const args = [];

  if (process.env.DATABASE_URL) {
    args.push(process.env.DATABASE_URL);
  }

  args.push('-t', '-A', '-F', '\t', '-v', 'ON_ERROR_STOP=1', '-c', sql);

  const result = spawnSync('psql', args, {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  });

  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout || 'psql failed');
  }

  return result.stdout.trim();
}

function scalar(sql) {
  return psql(sql).split('\n').map((line) => line.trim()).filter(Boolean)[0] || '';
}

function bool(sql) {
  return scalar(sql) === 't';
}

function count(sql) {
  const value = scalar(sql);
  const parsed = Number.parseInt(value || '0', 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

function tableExists(table) {
  return bool(`select to_regclass('public.${table}') is not null;`);
}

function columnExists(table, column) {
  return bool(`
    select exists (
      select 1
      from information_schema.columns
      where table_schema = 'public'
        and table_name = '${table}'
        and column_name = '${column}'
    );
  `);
}

function checkCount(name, sql, level = 'blocker') {
  const n = count(sql);

  if (n === 0) {
    pass(name, '0 rows');
    return;
  }

  if (level === 'warning') {
    warn(name, `${n} rows`);
  } else {
    block(name, `${n} rows`);
  }
}

console.log('PASS 20D-C live report data consistency audit');
console.log('');

const requiredTables = [
  'social_reports',
  'social_posts',
  'social_comments',
  'social_moderation_actions'
];

for (const table of requiredTables) {
  if (tableExists(table)) pass(`table exists: ${table}`);
  else block(`missing table: ${table}`);
}

const reportColumns = ['id', 'target_type', 'target_id', 'status'];
for (const column of reportColumns) {
  if (columnExists('social_reports', column)) pass(`social_reports.${column} exists`);
  else block(`missing social_reports.${column}`);
}

const moderationColumns = ['id', 'target_type', 'target_id'];
for (const column of moderationColumns) {
  if (columnExists('social_moderation_actions', column)) {
    pass(`social_moderation_actions.${column} exists`);
  } else {
    block(`missing social_moderation_actions.${column}`);
  }
}

if (blockers.length === 0) {
  checkCount(
    'reports with unsupported target_type',
    `
      select count(*)
      from social_reports
      where lower(target_type::text) not in ('post', 'comment');
    `
  );

  checkCount(
    'post reports pointing to missing social_posts',
    `
      select count(*)
      from social_reports r
      left join social_posts p on p.id::text = r.target_id::text
      where lower(r.target_type::text) = 'post'
        and p.id is null;
    `
  );

  checkCount(
    'comment reports pointing to missing social_comments',
    `
      select count(*)
      from social_reports r
      left join social_comments c on c.id::text = r.target_id::text
      where lower(r.target_type::text) = 'comment'
        and c.id is null;
    `
  );

  checkCount(
    'moderation actions with unsupported target_type',
    `
      select count(*)
      from social_moderation_actions
      where lower(target_type::text) not in ('post', 'comment', 'report');
    `,
    'warning'
  );

  checkCount(
    'post moderation actions pointing to missing social_posts',
    `
      select count(*)
      from social_moderation_actions a
      left join social_posts p on p.id::text = a.target_id::text
      where lower(a.target_type::text) = 'post'
        and p.id is null;
    `
  );

  checkCount(
    'comment moderation actions pointing to missing social_comments',
    `
      select count(*)
      from social_moderation_actions a
      left join social_comments c on c.id::text = a.target_id::text
      where lower(a.target_type::text) = 'comment'
        and c.id is null;
    `
  );

  checkCount(
    'report moderation actions pointing to missing social_reports',
    `
      select count(*)
      from social_moderation_actions a
      left join social_reports r on r.id::text = a.target_id::text
      where lower(a.target_type::text) = 'report'
        and r.id is null;
    `
  );

  checkCount(
    'reports with unknown status',
    `
      select count(*)
      from social_reports
      where lower(status::text) not in ('pending', 'reviewed', 'dismissed');
    `,
    'warning'
  );

  if (columnExists('social_reports', 'reporter_user_id')) {
    checkCount(
      'duplicate pending reports by same reporter and target',
      `
        select count(*)
        from (
          select reporter_user_id, target_type, target_id, count(*) as n
          from social_reports
          where lower(status::text) = 'pending'
          group by reporter_user_id, target_type, target_id
          having count(*) > 1
        ) duplicates;
      `,
      'warning'
    );
  } else {
    warn('duplicate reporter-target check skipped', 'social_reports.reporter_user_id missing');
  }

  if (columnExists('social_posts', 'status')) {
    checkCount(
      'posts with unknown status',
      `
        select count(*)
        from social_posts
        where lower(status::text) not in ('published', 'hidden', 'removed', 'deleted');
      `,
      'warning'
    );
  }

  if (columnExists('social_comments', 'status')) {
    checkCount(
      'comments with unknown status',
      `
        select count(*)
        from social_comments
        where lower(status::text) not in ('published', 'hidden', 'removed', 'deleted');
      `,
      'warning'
    );
  }
}

console.log('OK:');
for (const item of ok) {
  console.log(`  OK  ${item.name}${item.detail ? ` — ${item.detail}` : ''}`);
}

if (warnings.length > 0) {
  console.log('');
  console.log('Warnings:');
  for (const item of warnings) {
    console.log(`  WARN  ${item.name}${item.detail ? ` — ${item.detail}` : ''}`);
  }
}

if (blockers.length > 0) {
  console.log('');
  console.log('Blockers:');
  for (const item of blockers) {
    console.log(`  FAIL  ${item.name}${item.detail ? ` — ${item.detail}` : ''}`);
  }

  console.log('');
  console.log('PASS 20D-C FAILED');
  process.exit(1);
}

console.log('');
console.log('PASS 20D-C OK');
