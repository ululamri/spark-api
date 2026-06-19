#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';

const file = 'src/admin_ai.rs';
let s = readFileSync(file, 'utf8');

s = s.replace("use sha2::{Digest, Sha256};\n", '');
s = s.replace('use crate::{ai_runtime, moderation, state::AppState};', 'use crate::{admin_auth, ai_runtime, moderation, state::AppState};');
s = s.replace(/^const ADMIN_HEADER: &str = .*\n/m, '');

const oldAuthStart = s.indexOf('fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), AdminAiError> {');
const oldAuthEnd = s.indexOf('\n#[derive(Serialize)]\nstruct ScopeData');
if (oldAuthStart === -1 || oldAuthEnd === -1 || oldAuthEnd <= oldAuthStart) {
  if (s.includes('admin_auth::authorize_with_capability(state, headers, "ai_manage").await?')) {
    console.log('PASS 20C admin AI RBAC already applied.');
    process.exit(0);
  }
  throw new Error('Could not locate legacy admin AI authorize block.');
}

const newAuth = `async fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), AdminAiError> {
    admin_auth::authorize_with_capability(state, headers, "ai_manage").await?;
    Ok(())
}`;
s = s.slice(0, oldAuthStart) + newAuth + s.slice(oldAuthEnd);

s = s.replaceAll('authorize(&state, &headers)?;', 'authorize(&state, &headers).await?;');

writeFileSync(file, s);
console.log('PASS 20C admin AI RBAC patch applied.');
