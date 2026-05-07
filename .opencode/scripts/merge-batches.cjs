const fs = require("fs");
const path = require("path");

const DATA_DIR = path.join(__dirname, "..", "data");
const BATCH_FILES = fs.readdirSync(DATA_DIR)
  .filter(f => f.startsWith("analyzer-output-batch-") && f.endsWith(".json"))
  .sort();

// Read all batches
const allTasks = [];
for (const f of BATCH_FILES) {
  const content = JSON.parse(fs.readFileSync(path.join(DATA_DIR, f), "utf8"));
  for (const task of content.tasks) {
    allTasks.push(task);
  }
}

console.log(`Loaded ${allTasks.length} tasks from ${BATCH_FILES.length} batches`);

// Normalize keys
for (const t of allTasks) {
  if (t.source_reference && !t.source_references) {
    t.source_references = Array.isArray(t.source_reference) ? t.source_reference : [t.source_reference];
    delete t.source_reference;
  }
  for (const key of ["acceptance_criteria", "implementation_notes", "blockers", "dependencies", "labels"]) {
    if (typeof t[key] === "string") {
      t[key] = [t[key]];
    } else if (!Array.isArray(t[key])) {
      t[key] = [];
    }
  }
}

// Dedup by normalized title
const normalizeKey = (title) => title.toLowerCase().trim().replace(/[^a-z0-9\s]/g, "");

const groups = {};
for (let i = 0; i < allTasks.length; i++) {
  const t = allTasks[i];
  const key = normalizeKey(t.title);
  if (!groups[key]) groups[key] = [];
  groups[key].push({ ...t, _idx: i });
}

const mergedTasks = [];
const localIdToLocalIdMap = {};
let taskIdCounter = 0;

for (const [key, tasks] of Object.entries(groups)) {
  tasks.sort((a, b) => Object.keys(b).length - Object.keys(a).length);
  const canonical = tasks[0];

  const mergedSourceRefs = new Set();
  const mergedAC = new Set();
  for (const t of tasks) {
    if (t.source_references) for (const sr of t.source_references) mergedSourceRefs.add(sr);
    if (t.acceptance_criteria) for (const ac of t.acceptance_criteria) mergedAC.add(ac);
  }

  const targetList = canonical.type === "definition" ? "definitions" : "backlog";

  const merged = {
    local_id: `T-${String(++taskIdCounter).padStart(3, "0")}`,
    title: canonical.title,
    type: canonical.type,
    priority: canonical.priority,
    target_list: targetList,
    description: canonical.description,
    acceptance_criteria: Array.from(mergedAC),
    implementation_notes: canonical.implementation_notes || [],
    blockers: canonical.blockers || [],
    dependencies: canonical.dependencies || [],
    labels: canonical.labels || [],
    source_references: Array.from(mergedSourceRefs),
    confidence: canonical.confidence || "high",
  };

  for (const t of tasks) {
    const origId = t.local_id;
    if (!localIdToLocalIdMap[origId]) {
      localIdToLocalIdMap[origId] = merged.local_id;
    }
  }

  mergedTasks.push(merged);
}

console.log(`After dedup: ${mergedTasks.length} tasks from ${allTasks.length} original`);

// Resolve dependencies
for (const t of mergedTasks) {
  const resolvedDeps = new Set();
  for (const dep of t.dependencies) {
    if (/^[A-Z]-\d+$/.test(dep)) {
      const mapped = localIdToLocalIdMap[dep];
      if (mapped) resolvedDeps.add(mapped);
    } else {
      resolvedDeps.add(dep);
    }
  }
  t.dependencies = Array.from(resolvedDeps);
}

// Build dependency edges
const depEdges = [];
for (const t of mergedTasks) {
  for (const dep of t.dependencies) {
    if (/^T-\d+$/.test(dep)) {
      depEdges.push({ from: dep, to: t.local_id, type: "blocks" });
    }
  }
}

// Output — writer input format
const output = {
  cards_to_move: [],
  dependency_edges: depEdges,
  output_contract: "writer",
  tasks_to_create: mergedTasks,
};

fs.writeFileSync(path.join(DATA_DIR, "merged-task-plan.json"), JSON.stringify(output, null, 2));
console.log("Written to .opencode/data/merged-task-plan.json");
console.log(`Tasks: ${mergedTasks.length}, Edges: ${depEdges.length}`);
