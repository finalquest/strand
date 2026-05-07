#!/usr/bin/env node
// Applies semantic merger operations and writes the writer input plan.

import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const DATA_DIR = process.env.SPEC_TO_KANBAN_DATA_DIR || join(import.meta.dirname, '..', 'data');
const WINDOWS_DIR = join(DATA_DIR, 'merger-task-windows');
const ARRAY_FIELDS = ['acceptance_criteria', 'implementation_notes', 'blockers', 'dependencies', 'labels', 'source_references'];
const ACTION_ARRAY_FIELDS = ['implementation_steps', 'out_of_scope'];
const BOUNDARY_ARRAY_FIELDS = ['paths', 'entrypoints', 'modules'];

function uniq(items) {
  return [...new Set((items || []).filter(Boolean))];
}

function loadTasks() {
  return readdirSync(WINDOWS_DIR)
    .filter(f => /^window-\d+\.json$/.test(f))
    .map(f => join(WINDOWS_DIR, f))
    .filter(f => statSync(f).isFile())
    .sort()
    .flatMap(file => JSON.parse(readFileSync(file, 'utf8')).tasks || []);
}

function normalizeTask(task) {
  const clean = { ...task };
  delete clean.original_local_id;
  delete clean.origin_key;
  for (const field of ARRAY_FIELDS) {
    clean[field] = Array.isArray(clean[field]) ? clean[field] : [];
  }
  for (const field of ACTION_ARRAY_FIELDS) {
    clean[field] = Array.isArray(clean[field]) ? clean[field] : [];
  }
  clean.implementation_boundary = clean.implementation_boundary && typeof clean.implementation_boundary === 'object' && !Array.isArray(clean.implementation_boundary)
    ? clean.implementation_boundary
    : {};
  for (const field of BOUNDARY_ARRAY_FIELDS) {
    clean.implementation_boundary[field] = Array.isArray(clean.implementation_boundary[field]) ? clean.implementation_boundary[field] : [];
  }
  clean.verification = clean.verification && typeof clean.verification === 'object' && !Array.isArray(clean.verification)
    ? clean.verification
    : {};
  clean.verification.command = typeof clean.verification.command === 'string' ? clean.verification.command : '';
  clean.verification.expected_result = typeof clean.verification.expected_result === 'string' ? clean.verification.expected_result : '';
  return clean;
}

function priorityRank(task) {
  return ({ high: 0, medium: 1, low: 2 })[task.priority] ?? 9;
}

function sortForExecution(tasks) {
  const taskById = new Map(tasks.map(task => [task.local_id, task]));
  const originalIndex = new Map(tasks.map((task, index) => [task.local_id, index]));
  const indegree = new Map(tasks.map(task => [task.local_id, 0]));
  const dependents = new Map(tasks.map(task => [task.local_id, []]));
  for (const task of tasks) {
    for (const dep of task.dependencies || []) {
      if (!taskById.has(dep)) continue;
      indegree.set(task.local_id, indegree.get(task.local_id) + 1);
      dependents.get(dep).push(task.local_id);
    }
  }
  const compare = (a, b) => priorityRank(a) - priorityRank(b) || (originalIndex.get(a.local_id) ?? 0) - (originalIndex.get(b.local_id) ?? 0);
  let ready = tasks.filter(task => indegree.get(task.local_id) === 0).sort(compare);
  const ordered = [];
  while (ready.length) {
    const task = ready.shift();
    ordered.push(task);
    const nextReady = [];
    for (const id of dependents.get(task.local_id) || []) {
      const next = indegree.get(id) - 1;
      indegree.set(id, next);
      if (next === 0) nextReady.push(taskById.get(id));
    }
    ready = ready.concat(nextReady).sort(compare);
  }
  if (ordered.length !== tasks.length) throw new Error('Dependency cycle detected while ordering final plan');
  return ordered;
}

function main() {
  const opsPath = join(DATA_DIR, 'merge-ops.json');
  const ops = JSON.parse(readFileSync(opsPath, 'utf8'));
  const tasksById = new Map(loadTasks().map(task => [task.local_id, normalizeTask(task)]));
  const droppedToKeep = new Map();

  for (const op of ops.merge || []) {
    const keep = tasksById.get(op.keep);
    const drop = tasksById.get(op.drop);
    if (!keep || !drop) throw new Error(`Invalid merge operation: ${op.keep} <- ${op.drop}`);
    for (const field of ARRAY_FIELDS) {
      keep[field] = uniq([...(keep[field] || []), ...(drop[field] || [])]);
    }
    for (const field of ACTION_ARRAY_FIELDS) {
      keep[field] = uniq([...(keep[field] || []), ...(drop[field] || [])]);
    }
    for (const field of BOUNDARY_ARRAY_FIELDS) {
      keep.implementation_boundary[field] = uniq([...(keep.implementation_boundary[field] || []), ...(drop.implementation_boundary[field] || [])]);
    }
    if (!keep.verification.command && drop.verification.command) keep.verification.command = drop.verification.command;
    if (!keep.verification.expected_result && drop.verification.expected_result) keep.verification.expected_result = drop.verification.expected_result;
    if (op.reason) keep.implementation_notes = uniq([...(keep.implementation_notes || []), `Merged ${op.drop}: ${op.reason}`]);
    droppedToKeep.set(op.drop, op.keep);
    tasksById.delete(op.drop);
  }

  function resolve(id) {
    let current = id;
    const seen = new Set();
    while (droppedToKeep.has(current) && !seen.has(current)) {
      seen.add(current);
      current = droppedToKeep.get(current);
    }
    return current;
  }

  for (const task of tasksById.values()) {
    task.dependencies = uniq((task.dependencies || []).map(resolve).filter(id => id !== task.local_id && tasksById.has(id)));
  }

  for (const op of ops.add_dependencies || []) {
    const task = tasksById.get(resolve(op.task || op.to));
    const dependency = resolve(op.depends_on || op.dependency || op.from);
    if (!task || !tasksById.has(dependency)) throw new Error(`Invalid dependency operation: ${JSON.stringify(op)}`);
    if (dependency !== task.local_id) task.dependencies = uniq([...(task.dependencies || []), dependency]);
  }

  for (const op of ops.update_tasks || []) {
    const task = tasksById.get(resolve(op.id));
    if (!task) throw new Error(`Invalid update operation for ${op.id}`);
    for (const key of ['title', 'type', 'priority', 'description', 'target_list', 'confidence']) {
      if (op[key] !== undefined) task[key] = op[key];
    }
    for (const key of ARRAY_FIELDS) {
      if (op[`add_${key}`]) task[key] = uniq([...(task[key] || []), ...op[`add_${key}`]]);
      if (op[`set_${key}`]) task[key] = uniq(op[`set_${key}`]);
    }
    for (const key of ACTION_ARRAY_FIELDS) {
      if (op[`add_${key}`]) task[key] = uniq([...(task[key] || []), ...op[`add_${key}`]]);
      if (op[`set_${key}`]) task[key] = uniq(op[`set_${key}`]);
    }
    if (op.implementation_boundary) {
      for (const key of BOUNDARY_ARRAY_FIELDS) {
        if (op.implementation_boundary[`add_${key}`]) task.implementation_boundary[key] = uniq([...(task.implementation_boundary[key] || []), ...op.implementation_boundary[`add_${key}`]]);
        if (op.implementation_boundary[`set_${key}`]) task.implementation_boundary[key] = uniq(op.implementation_boundary[`set_${key}`]);
      }
    }
    if (op.verification) {
      if (op.verification.command !== undefined) task.verification.command = op.verification.command;
      if (op.verification.expected_result !== undefined) task.verification.expected_result = op.verification.expected_result;
    }
  }

  const edgeKeys = new Set();
  const dependencyEdges = [];
  function addEdge(from, to, type = 'blocks') {
    from = resolve(from);
    to = resolve(to);
    if (!tasksById.has(from) || !tasksById.has(to) || from === to) return;
    const key = `${from}|${to}|${type}`;
    if (!edgeKeys.has(key)) {
      edgeKeys.add(key);
      dependencyEdges.push({ from, to, type });
    }
  }

  for (const task of tasksById.values()) {
    for (const dep of task.dependencies || []) addEdge(dep, task.local_id, 'blocks');
  }
  for (const op of ops.add_edges || []) addEdge(op.from, op.to, op.type || 'blocks');

  const plan = {
    tasks_to_create: sortForExecution([...tasksById.values()]),
    dependency_edges: dependencyEdges.sort((a, b) => `${a.from}${a.to}${a.type}`.localeCompare(`${b.from}${b.to}${b.type}`)),
    cards_to_move: [],
    output_contract: 'writer',
  };

  writeFileSync(join(DATA_DIR, 'merged-task-plan.json'), JSON.stringify(plan, null, 2) + '\n', 'utf8');
  console.log(`Wrote merged-task-plan.json with ${plan.tasks_to_create.length} tasks and ${plan.dependency_edges.length} edges`);
}

try {
  main();
} catch (err) {
  console.error('Error:', err.message);
  process.exit(1);
}
