#!/usr/bin/env node
// Builds compact merger inputs from analyzer batch files.
// Now queries the board for the last used T-XXX ID to avoid collisions.

import { mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import https from 'node:https';

const DATA_DIR = process.env.SPEC_TO_KANBAN_DATA_DIR || join(import.meta.dirname, '..', 'data');
const WINDOW_SIZE = Number(process.env.SPEC_TO_KANBAN_WINDOW_SIZE || 10);
const MIN_SEQUENCE = Number(process.env.SPEC_TO_KANBAN_MIN_SEQUENCE || 0);

// Board config
const CONFIG_FILE = join(import.meta.dirname, '..', 'config', 'spec-to-kanban.json');
let boardConfig = {};
try {
  boardConfig = JSON.parse(readFileSync(CONFIG_FILE, 'utf8'));
} catch {
  boardConfig = {};
}

const BOARD_ID = boardConfig.planka?.board_id;
const BASE_URL = (boardConfig.planka?.base_url || '').replace(/\/+$/, '');
const API_KEY = process.env.BOARD_API_KEY || '';

function apiReq(urlPath) {
  return new Promise((resolve, reject) => {
    const urlStr = `${BASE_URL}${urlPath.startsWith('/') ? urlPath : `/${urlPath}`}`;
    const parsed = new URL(urlStr);
    const headers = { 'Accept': 'application/json' };
    if (API_KEY) headers['X-Api-Key'] = API_KEY;

    const req = https.request(
      {
        hostname: parsed.hostname,
        port: parsed.port,
        path: parsed.pathname + parsed.search,
        method: 'GET',
        headers,
      },
      (res) => {
        const chunks = [];
        res.on('data', (c) => chunks.push(c));
        res.on('end', () => {
          const body = Buffer.concat(chunks).toString('utf8');
          let parsedBody = {};
          try {
            parsedBody = body ? JSON.parse(body) : {};
          } catch {
            reject(new Error(`GET ${urlPath} returned non-JSON (${res.statusCode})`));
            return;
          }
          if (res.statusCode >= 400) {
            reject(new Error(`GET ${urlPath} ${res.statusCode}: ${JSON.stringify(parsedBody).slice(0, 200)}`));
          } else {
            resolve(parsedBody);
          }
        });
      }
    );
    req.on('error', reject);
    req.end();
  });
}

async function getLastTaskId() {
  if (!BOARD_ID || !BASE_URL) {
    console.warn('No board config found; starting sequence from 0');
    return 0;
  }

  try {
    const data = await apiReq(`/api/boards/${BOARD_ID}/cards`);
    const cards = Array.isArray(data) ? data : (data.items || []);
    let maxId = 0;

    for (const card of cards) {
      const match = String(card.name || '').match(/T-(\d+)/);
      if (match) {
        const num = parseInt(match[1], 10);
        if (num > maxId) maxId = num;
      }
    }

    console.log(`Last board task ID: T-${String(maxId).padStart(3, '0')}`);
    return maxId;
  } catch (err) {
    console.warn(`Failed to query board for last ID: ${err.message}`);
    return 0;
  }
}

function batchFiles() {
  return readdirSync(DATA_DIR)
    .filter(f => /^analyzer-output-batch-\d+\.json$/.test(f))
    .map(f => join(DATA_DIR, f))
    .filter(f => statSync(f).isFile())
    .sort();
}

function compactText(value, max = 180) {
  const text = String(value || '').replace(/\s+/g, ' ').trim();
  return text.length > max ? `${text.slice(0, max - 1)}…` : text;
}

function compactCriteria(criteria) {
  return (Array.isArray(criteria) ? criteria : [])
    .slice(0, 3)
    .map(item => compactText(item, 120));
}

function compactSources(sources) {
  return (Array.isArray(sources) ? sources : [])
    .slice(0, 2)
    .map(item => compactText(item, 160));
}

async function main() {
  const files = batchFiles();
  if (files.length === 0) {
    console.error(`No analyzer-output-batch-*.json files found in ${DATA_DIR}`);
    process.exit(1);
  }

  const boardLastId = await getLastTaskId();
  const lastId = Math.max(boardLastId, MIN_SEQUENCE);
  const tasks = [];
  const idMap = {};
  let sequence = lastId;

  for (const filePath of files) {
    const batch = JSON.parse(readFileSync(filePath, 'utf8'));
    for (const task of batch.tasks || []) {
      sequence += 1;
      const finalId = `T-${String(sequence).padStart(3, '0')}`;
      const originKey = `${basename(filePath)}#${task.local_id}`;
      const normalized = { ...task, local_id: finalId, original_local_id: task.local_id, origin_key: originKey };
      tasks.push(normalized);
      idMap[originKey] = finalId;
    }
  }

  const byBatch = new Map();
  for (const task of tasks) {
    const batchName = task.origin_key.split('#')[0];
    if (!byBatch.has(batchName)) byBatch.set(batchName, []);
    byBatch.get(batchName).push(task);
  }

  for (const task of tasks) {
    const batchName = task.origin_key.split('#')[0];
    task.dependencies = (Array.isArray(task.dependencies) ? task.dependencies : [])
      .map(dep => idMap[`${batchName}#${dep}`])
      .filter(Boolean);
  }

  const index = tasks.map(task => ({
    id: task.local_id,
    title: task.title,
    type: task.type,
    priority: task.priority,
    labels: Array.isArray(task.labels) ? task.labels : [],
    target_list: task.target_list,
    description_preview: compactText(task.description),
    criteria_fingerprint: compactCriteria(task.acceptance_criteria),
    source_references_preview: compactSources(task.source_references),
    dependencies: task.dependencies,
    original_local_id: task.original_local_id,
    origin_key: task.origin_key,
  }));

  const windowsDir = join(DATA_DIR, 'merger-task-windows');
  rmSync(windowsDir, { recursive: true, force: true });
  mkdirSync(windowsDir, { recursive: true });

  const windows = [];
  for (let i = 0; i < tasks.length; i += WINDOW_SIZE) {
    const windowTasks = tasks.slice(i, i + WINDOW_SIZE);
    const number = String(windows.length + 1).padStart(3, '0');
    const file = `window-${number}.json`;
    windows.push({ file, task_ids: windowTasks.map(t => t.local_id) });
    writeFileSync(join(windowsDir, file), JSON.stringify({ window: windows.length + 1, tasks: windowTasks }, null, 2) + '\n', 'utf8');
  }

  writeFileSync(join(DATA_DIR, 'merger-task-index.json'), JSON.stringify({ task_count: tasks.length, windows, tasks: index }, null, 2) + '\n', 'utf8');
  writeFileSync(join(DATA_DIR, 'merger-task-map.json'), JSON.stringify({ id_map: idMap }, null, 2) + '\n', 'utf8');
  writeFileSync(join(DATA_DIR, 'merge-ops.json'), JSON.stringify({ merge: [], add_edges: [], add_dependencies: [], update_tasks: [] }, null, 2) + '\n', 'utf8');

  console.log(`Prepared ${tasks.length} tasks from ${files.length} batches into ${windows.length} windows`);
  console.log(`ID sequence starts at T-${String(lastId + 1).padStart(3, '0')}`);
}

main().catch(err => {
  console.error('Error:', err.message);
  process.exit(1);
});
