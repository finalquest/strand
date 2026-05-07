import { readFileSync } from "fs";
import { join } from "path";

const dataDir = process.env.SPEC_TO_KANBAN_DATA_DIR || ".opencode/data";
const plan = JSON.parse(readFileSync(join(dataDir, "merged-task-plan.json"), "utf8"));
const errors = [];
const tasks = plan.tasks_to_create;
const allowedLabels = new Set(["cli", "storage", "evaluator", "validation", "observability", "docs", "definition", "foundation", "feature", "bug", "spike", "mvp", "blocked", "needs-decision", "high", "medium", "low"]);
const allowedTargetLists = new Set(["definitions", "backlog", "ready", "blocked"]);
const requiredBoundaryArrays = ["paths", "entrypoints", "modules"];

// 1. Top-level fields must be EXACTLY these 4
const allowedKeys = ["tasks_to_create", "dependency_edges", "cards_to_move", "output_contract"];
const actualKeys = Object.keys(plan);
const extraKeys = actualKeys.filter(k => !allowedKeys.includes(k));
if (extraKeys.length) errors.push(`Unexpected top-level keys: ${extraKeys.join(", ")}`);

const missingKeys = allowedKeys.filter(k => !(k in plan));
if (missingKeys.length) errors.push(`Missing required top-level keys: ${missingKeys.join(", ")}`);

// 2. output_contract must be "writer"
if (plan.output_contract !== "writer") {
  errors.push(`output_contract must be "writer", got: '${plan.output_contract}'`);
}

// 3. T-xxx format
for (const t of tasks) {
  if (!/^T-\d+$/.test(t.local_id)) {
    errors.push(`Bad ID format: '${t.local_id}'`);
  }
}

// 4. Uniqueness
const ids = tasks.map(t => t.local_id);
const dupes = [...new Set(ids.filter((id, i) => ids.indexOf(id) !== i))];
if (dupes.length) errors.push(`Duplicate IDs: ${dupes.join(", ")}`);

// 5. Check all deps exist
const idSet = new Set(ids);
for (const t of tasks) {
  for (const dep of t.dependencies) {
    if (!idSet.has(dep)) {
      errors.push(`${t.local_id} depends on unresolved: '${dep}'`);
    }
  }
}

// 6. Check dependency edges reference valid IDs
for (const e of plan.dependency_edges || []) {
  if (!idSet.has(e.from)) errors.push(`Edge: unknown 'from' ${e.from}`);
  if (!idSet.has(e.to)) errors.push(`Edge: unknown 'to' ${e.to}`);
}

// 7. Required arrays per task
const requiredArrays = ["acceptance_criteria", "implementation_notes", "blockers", "dependencies", "labels", "source_references"];
for (const t of tasks) {
  for (const field of requiredArrays) {
    if (!Array.isArray(t[field])) {
      errors.push(`${t.local_id}: ${field} must be array, got ${typeof t[field]}`);
    }
  }
  for (const field of ["implementation_steps", "out_of_scope"]) {
    if (!Array.isArray(t[field])) errors.push(`${t.local_id}: ${field} must be array, got ${typeof t[field]}`);
  }
  if (!t.implementation_boundary || typeof t.implementation_boundary !== "object" || Array.isArray(t.implementation_boundary)) {
    errors.push(`${t.local_id}: implementation_boundary must be object`);
  } else {
    for (const field of requiredBoundaryArrays) {
      if (!Array.isArray(t.implementation_boundary[field])) {
        errors.push(`${t.local_id}: implementation_boundary.${field} must be array`);
      }
    }
  }
  if (!t.verification || typeof t.verification !== "object" || Array.isArray(t.verification)) {
    errors.push(`${t.local_id}: verification must be object`);
  } else {
    if (typeof t.verification.command !== "string" || !t.verification.command.trim()) errors.push(`${t.local_id}: verification.command is required`);
    if (typeof t.verification.expected_result !== "string" || !t.verification.expected_result.trim()) errors.push(`${t.local_id}: verification.expected_result is required`);
  }
  if (!["definition", "docs"].includes(t.type)) {
    if (!t.implementation_steps || t.implementation_steps.length === 0) errors.push(`${t.local_id}: implementation_steps must not be empty`);
    if (!t.implementation_boundary?.paths?.length && !t.implementation_boundary?.entrypoints?.length && !t.implementation_boundary?.modules?.length) {
      errors.push(`${t.local_id}: implementation_boundary must name at least one path, entrypoint, or module`);
    }
  }
  // Required scalars
  if (!t.title) errors.push(`${t.local_id}: missing title`);
  if (!t.type) errors.push(`${t.local_id}: missing type`);
  if (!t.priority) errors.push(`${t.local_id}: missing priority`);
  if (!t.description) errors.push(`${t.local_id}: missing description`);
  if (!["high", "medium", "low"].includes(t.confidence)) {
    errors.push(`${t.local_id}: confidence must be high, medium, or low`);
  }
  if (!allowedTargetLists.has(t.target_list)) {
    errors.push(`${t.local_id}: target_list must be one of ${[...allowedTargetLists].join(", ")}`);
  }
  for (const label of t.labels || []) {
    if (!allowedLabels.has(label)) errors.push(`${t.local_id}: unsupported label '${label}'`);
  }
}

// 8. Simple cycle detection
const adj = {};
for (const t of tasks) {
  adj[t.local_id] = t.dependencies.filter(d => idSet.has(d));
}

function hasCycle() {
  const visited = new Set();
  const onStack = new Set();
  function visit(id) {
    if (onStack.has(id)) return true;
    if (visited.has(id)) return false;
    visited.add(id);
    onStack.add(id);
    for (const dep of (adj[id] || [])) {
      if (visit(dep)) return true;
    }
    onStack.delete(id);
    return false;
  }
  for (const id of idSet) {
    if (visit(id)) return true;
  }
  return false;
}

if (hasCycle()) errors.push("Circular dependencies detected");

// 9. Plan readability: dependencies should appear before dependents.
const position = new Map(ids.map((id, index) => [id, index]));
for (const t of tasks) {
  for (const dep of t.dependencies || []) {
    if (position.has(dep) && position.get(dep) > position.get(t.local_id)) {
      errors.push(`${t.local_id}: dependency ${dep} appears after dependent task`);
    }
  }
}

if (errors.length === 0) {
  console.log("VALIDATION PASSED");
  process.exit(0);
} else {
  console.error("VALIDATION FAILED:");
  errors.forEach(e => console.error(`  ✗ ${e}`));
  process.exit(1);
}
