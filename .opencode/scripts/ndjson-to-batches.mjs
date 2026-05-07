#!/usr/bin/env node
// ndjson-to-batches.mjs
// Reads analyzer-output-shard*.ndjson files and converts them to batch files.
// Each batch contains ~10 tasks as pretty-printed JSON.

import { readdirSync, readFileSync, writeFileSync, mkdirSync, statSync, unlinkSync } from 'node:fs';
import { join, basename } from 'node:path';

const BATCH_SIZE = 10;
const DATA_DIR = process.env.SPEC_TO_KANBAN_DATA_DIR || join(import.meta.dirname, '..', 'data');

// Simple glob replacement for simple patterns
function globPattern(dir, pattern) {
  const base = pattern.split('*')[0].replace(dir + '/', '');
  const endsWith = pattern.split('*').pop();
  const entries = readdirSync(dir);
  return entries
    .filter(f => f.startsWith(base) && f.endsWith(endsWith))
    .map(f => join(dir, f))
    .filter(f => statSync(f).isFile());
}

function ensureDir() {
  mkdirSync(DATA_DIR, { recursive: true });
}

function clearOldBatches() {
  const oldBatches = globPattern(DATA_DIR, 'analyzer-output-batch-*.json');
  for (const file of oldBatches) {
    unlinkSync(file);
    console.log(`Cleared old batch: ${basename(file)}`);
  }
}

function main() {
  ensureDir();
  clearOldBatches();

  // Find all NDJSON shard files
  const shardFiles = globPattern(DATA_DIR, 'analyzer-output-shard*.ndjson').sort();

  if (shardFiles.length === 0) {
    console.error(`No analyzer-output-shard*.ndjson files found in ${DATA_DIR}`);
    process.exit(1);
  }

  // Read and parse all tasks from all shards
  const allTasks = [];

  for (const filePath of shardFiles) {
    const content = readFileSync(filePath, 'utf8').trim();
    if (!content) continue;

    const lines = content.split('\n').filter(l => l.trim());

    for (const line of lines) {
      const trimmed = line.trim();
      if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
        try {
          const task = JSON.parse(trimmed);
          if (task.local_id && task.title && !String(task.local_id).startsWith('[WARNING]') && !String(task.local_id).startsWith('[QUESTION]')) {
            allTasks.push({ task, sourceFile: filePath });
          }
        } catch {
          console.error(`Warning: Failed to parse line in ${basename(filePath)}: ${trimmed.slice(0, 100)}`);
        }
      }
    }
  }

  if (allTasks.length === 0) {
    console.error('No tasks found across all shard files');
    process.exit(1);
  }

  // Group into batches
  const batches = [];
  let batchNum = 0;
  let batchTasks = [];
  let batchShards = new Set();

  for (const { task, sourceFile } of allTasks) {
    batchTasks.push(task);
    batchShards.add(basename(sourceFile));

    if (batchTasks.length >= BATCH_SIZE) {
      batchNum++;
      batches.push({
        batch: batchNum,
        source_shards: [...batchShards].sort(),
        tasks: batchTasks,
      });
      batchTasks = [];
      batchShards = new Set();
    }
  }

  // Flush remaining
  if (batchTasks.length > 0) {
    batchNum++;
    batches.push({
      batch: batchNum,
      source_shards: [...batchShards].sort(),
      tasks: batchTasks,
    });
  }

  // Write batch files
  for (const batch of batches) {
    const outPath = join(DATA_DIR, `analyzer-output-batch-${String(batch.batch).padStart(3, '0')}.json`);
    const output = {
      batch: batch.batch,
      source_shards: batch.source_shards,
      tasks: batch.tasks,
    };
    writeFileSync(outPath, JSON.stringify(output, null, 2) + '\n', 'utf8');
    console.log(`Wrote ${basename(outPath)}: ${batch.tasks.length} tasks from ${batch.source_shards.length} shard(s)`);
  }

  console.log(`Total: ${allTasks.length} tasks across ${batches.length} batches`);
}

try {
  main();
} catch (err) {
  console.error('Error:', err.message);
  process.exit(1);
}
