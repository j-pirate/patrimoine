#!/usr/bin/env node
// Compare a local npm lockfile with a separately downloaded public OSV database.
// No network client is used and dependency names/versions never leave this process.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const semver = require('semver');

function affectedByEvents(version, events) {
  let lower = null;
  let introduced = false;
  for (const event of events) {
    if ('introduced' in event) {
      lower = event.introduced === '0' ? null : event.introduced;
      if (lower !== null && !semver.valid(lower)) throw new Error(`Invalid introduced version ${lower}`);
      introduced = true;
    } else if ('fixed' in event || 'last_affected' in event || 'limit' in event) {
      const upper = event.fixed ?? event.last_affected ?? event.limit;
      if (!semver.valid(upper)) throw new Error(`Invalid upper version ${upper}`);
      const below = 'last_affected' in event ? semver.lte(version, upper) : semver.lt(version, upper);
      if (introduced && (lower === null || semver.gte(version, lower)) && below) return true;
      lower = null;
      introduced = false;
    } else throw new Error('Unsupported OSV version event');
  }
  return introduced && (lower === null || semver.gte(version, lower));
}

if (process.argv.includes('--self-test')) {
  assert.equal(affectedByEvents('1.2.2', [{ introduced: '0' }, { fixed: '1.2.3' }]), true);
  assert.equal(affectedByEvents('1.2.3', [{ introduced: '0' }, { fixed: '1.2.3' }]), false);
  assert.equal(affectedByEvents('1.2.3', [{ introduced: '1.2.0' }, { last_affected: '1.2.3' }]), true);
  assert.equal(affectedByEvents('1.1.9', [{ introduced: '1.2.0' }]), false);
  assert.equal(affectedByEvents('1.2.3-beta.1', [{ introduced: '1.2.3-beta.0' }, { fixed: '1.2.3' }]), true);
  assert.equal(affectedByEvents('2.1.0', [{ introduced: '0' }, { fixed: '1.0.0' }, { introduced: '2.0.0' }, { fixed: '2.2.0' }]), true);
  assert.equal(affectedByEvents('1.5.0', [{ introduced: '0' }, { fixed: '1.0.0' }, { introduced: '2.0.0' }, { fixed: '2.2.0' }]), false);
  console.log('7 local OSV range boundary checks passed.');
  process.exit(0);
}

function argument(name, fallback) {
  const i = process.argv.indexOf(name);
  if (i < 0) return fallback;
  if (!process.argv[i + 1] || process.argv[i + 1].startsWith('--')) throw new Error(`Missing ${name}`);
  return process.argv[i + 1];
}
const database = resolve(argument('--database', '/workspace/.tooling/github-advisory-database'));
const lockfile = resolve(argument('--lockfile', 'package-lock.json'));
const output = argument('--output', null);
function databaseCommit() {
  const head = readFileSync(join(database, '.git', 'HEAD'), 'utf8').trim();
  if (/^[a-f0-9]{40}$/.test(head)) return head;
  const reference = /^ref: (refs\/[a-zA-Z0-9_./-]+)$/.exec(head)?.[1];
  if (!reference || reference.includes('..')) throw new Error('Unsupported public database Git HEAD');
  try { return readFileSync(join(database, '.git', reference), 'utf8').trim(); }
  catch {
    const row = readFileSync(join(database, '.git', 'packed-refs'), 'utf8').split('\n').find(line => line.split(' ')[1] === reference);
    if (!row || !/^[a-f0-9]{40} /.test(row)) throw new Error('Missing public database Git reference');
    return row.split(' ')[0];
  }
}
const rawLock = readFileSync(lockfile);
const lock = JSON.parse(rawLock);
if (lock.lockfileVersion !== 3 || !lock.packages) throw new Error('A package-lock.json version 3 is required.');
const packages = new Map();
for (const [path, entry] of Object.entries(lock.packages)) {
  if (!path || !entry.version || entry.link) continue;
  const name = entry.name ?? path.split('node_modules/').at(-1);
  if (!name || !semver.valid(entry.version)) throw new Error(`Unsupported locked package ${path}`);
  const key = `${name}@${entry.version}`;
  const record = packages.get(key) ?? { name, version: entry.version, paths: [], development_only: true };
  record.paths.push(path);
  record.development_only &&= entry.dev === true;
  packages.set(key, record);
}
const byName = new Map();
for (const item of packages.values()) {
  if (!byName.has(item.name)) byName.set(item.name, []);
  byName.get(item.name).push(item);
}
let jsonFiles = 0;
let npmStatements = 0;
let withdrawn = 0;
const matches = new Map();
const unresolved = [];
function visit(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) { visit(path); continue; }
    if (!entry.isFile() || !entry.name.endsWith('.json')) continue;
    jsonFiles += 1;
    const advisory = JSON.parse(readFileSync(path, 'utf8'));
    if (advisory.withdrawn) { withdrawn += 1; continue; }
    for (const affected of advisory.affected ?? []) {
      if (affected.package?.ecosystem !== 'npm') continue;
      npmStatements += 1;
      const candidates = byName.get(affected.package.name) ?? [];
      for (const candidate of candidates) {
        let vulnerable = (affected.versions ?? []).some(version => semver.valid(version) && semver.eq(candidate.version, version));
        for (const range of affected.ranges ?? []) {
          // GitHub's npm OSV records also use ECOSYSTEM ranges. For npm,
          // their introduced/fixed version events use npm SemVer ordering.
          if (!['SEMVER', 'ECOSYSTEM'].includes(range.type)) {
            unresolved.push({ advisory: advisory.id, package: candidate.name, version: candidate.version, reason: `Unsupported ${range.type} range` });
            continue;
          }
          try { vulnerable ||= affectedByEvents(candidate.version, range.events ?? []); }
          catch (error) { unresolved.push({ advisory: advisory.id, package: candidate.name, version: candidate.version, reason: error.message }); }
        }
        if (!vulnerable) continue;
        const result = { advisory: advisory.id, aliases: advisory.aliases ?? [],
          package: candidate.name, version: candidate.version, development_only: candidate.development_only,
          paths: candidate.paths, severity: advisory.database_specific?.severity ?? 'UNKNOWN',
          summary: advisory.summary ?? '', modified: advisory.modified,
          ranges: affected.ranges ?? [], references: advisory.references ?? [] };
        matches.set(`${advisory.id}:${candidate.name}:${candidate.version}`, result);
      }
    }
  }
}
visit(join(database, 'advisories', 'github-reviewed'));
const found = [...matches.values()].sort((a, b) => `${a.package}:${a.version}:${a.advisory}`.localeCompare(`${b.package}:${b.version}:${b.advisory}`));
const result = {
  status: unresolved.length ? 'coverage_incomplete' : found.length ? 'advisories_found' : 'no_reviewed_advisory_match',
  generated_at: new Date().toISOString(),
  method: 'Offline comparison of locked npm names/versions with public github-reviewed OSV SEMVER and npm ECOSYSTEM version events using npm SemVer, plus explicit versions; no project metadata upload',
  semver_version: require('semver/package.json').version,
  lockfile_sha256: createHash('sha256').update(rawLock).digest('hex'),
  database_repository: 'https://github.com/github/advisory-database.git',
  database_commit: databaseCommit(),
  database_commit_time: argument('--database-commit-time', null),
  database_scope: 'advisories/github-reviewed only; unreviewed advisories, malicious-package lists and non-npm ecosystems are outside this JavaScript comparison',
  scanned_advisory_files: jsonFiles, npm_affected_statements: npmStatements, withdrawn_advisories_skipped: withdrawn,
  locked_unique_packages: packages.size, locked_production_packages: [...packages.values()].filter(p => !p.development_only).length,
  matches: found, unresolved_ranges: unresolved,
  limits: 'An absent match does not establish absence of unknown vulnerabilities. Rust and native runtime libraries require separate reviews.',
};
const text = `${JSON.stringify(result, null, 2)}\n`;
if (output) writeFileSync(output, text, { mode: 0o600 });
else console.log(text);
console.log(JSON.stringify({ status: result.status, packages: result.locked_unique_packages, production_packages: result.locked_production_packages, advisories: result.scanned_advisory_files, matches: found.length, unresolved: unresolved.length, output }));
process.exitCode = unresolved.length ? 2 : found.length ? 1 : 0;
