#!/usr/bin/env node
import { existsSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = process.env.SPEC_TO_KANBAN_REPO_ROOT || process.cwd();
const IGNORED = new Set(['.git', '.opencode', 'node_modules', '.vanguard']);
const PRODUCT_DIRS = ['src', 'app', 'lib', 'bin', 'cli', 'cmd', 'internal', 'packages'];
const TEST_DIRS = ['test', 'tests', '__tests__', 'spec'];
const PRODUCT_FILES = ['package.json', 'pyproject.toml', 'Cargo.toml', 'go.mod', 'deno.json', 'tsconfig.json'];
const LOCKFILES = ['package-lock.json', 'pnpm-lock.yaml', 'yarn.lock', 'bun.lockb'];

function exists(path) {
  return existsSync(join(ROOT, path));
}

function topLevelEntries() {
  return readdirSync(ROOT)
    .filter(name => !IGNORED.has(name))
    .filter(name => !name.startsWith('.DS_Store'))
    .map(name => {
      const full = join(ROOT, name);
      const stat = statSync(full);
      return { name, type: stat.isDirectory() ? 'dir' : 'file' };
    });
}

function inferRuntime(files) {
  if (files.includes('package.json')) return 'node/typescript';
  if (files.includes('deno.json')) return 'deno/typescript';
  if (files.includes('pyproject.toml')) return 'python';
  if (files.includes('Cargo.toml')) return 'rust';
  if (files.includes('go.mod')) return 'go';
  return 'node/typescript';
}

function main() {
  const entries = topLevelEntries();
  const files = entries.filter(e => e.type === 'file').map(e => e.name).sort();
  const dirs = entries.filter(e => e.type === 'dir').map(e => e.name).sort();
  const productFiles = PRODUCT_FILES.filter(exists);
  const lockfiles = LOCKFILES.filter(exists);
  const productDirs = PRODUCT_DIRS.filter(exists);
  const testDirs = TEST_DIRS.filter(exists);
  const hasProductCode = productDirs.length > 0 || productFiles.length > 0;
  const repoState = hasProductCode ? 'existing_app' : 'greenfield';
  const recommendedRuntime = inferRuntime(productFiles);

  const context = {
    repo_state: repoState,
    existing_product_code: hasProductCode,
    existing_product_files: productFiles,
    existing_product_dirs: productDirs,
    existing_test_dirs: testDirs,
    lockfiles,
    ignored_as_product: ['.opencode'],
    recommended_runtime: recommendedRuntime,
    reason: hasProductCode
      ? `Detected product indicators: ${[...productFiles, ...productDirs].join(', ')}`
      : 'No product runtime, source directory, package manifest, or tests were found outside repo-local OpenCode tooling; treat as greenfield and choose a minimal CLI architecture.',
    top_level_files: files,
    top_level_dirs: dirs,
  };

  process.stdout.write(`${JSON.stringify(context, null, 2)}\n`);
}

main();
