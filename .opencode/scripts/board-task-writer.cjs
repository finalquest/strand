#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const https = require("https");
const http = require("http");

// ─── Config ───

const CONFIG_FILE = path.join(__dirname, "..", "config", "spec-to-kanban.json");
let config = {};
try {
  config = JSON.parse(fs.readFileSync(CONFIG_FILE, "utf8"));
} catch {
  config = {};
}

function getBoardTarget(input) {
  const boardConfig = config.planka || config.board || config;
  return {
    base_url: input.board_target?.base_url || boardConfig.base_url,
    board_id: input.board_target?.board_id || boardConfig.board_id,
    lists: input.board_target?.lists || boardConfig.lists || {},
    labels: input.board_target?.labels || boardConfig.labels || {},
  };
}

function validateInput(input) {
  if (!input.output_contract || input.output_contract !== "writer") {
    throw new Error("output_contract is required and must be 'writer'");
  }
  if (!Array.isArray(input.tasks_to_create)) {
    throw new Error("tasks_to_create is required and must be an array");
  }
  if (!Array.isArray(input.dependency_edges)) {
    throw new Error("dependency_edges is required and must be an array");
  }

  const board = getBoardTarget(input);
  if (!board.base_url) throw new Error("Missing base_url (input or config)");
  if (!board.board_id) throw new Error("Missing board_id (input or config)");
  const allowedTargetLists = new Set(["definitions", "backlog", "ready", "blocked"]);

  const ids = new Set();
  for (const task of input.tasks_to_create) {
    if (!task.local_id || !/^T-\d+$/.test(task.local_id)) {
      throw new Error(`Invalid or missing local_id '${task.local_id}' for task`);
    }
    if (ids.has(task.local_id)) throw new Error(`Duplicate local_id: ${task.local_id}`);
    ids.add(task.local_id);

    if (!task.title) throw new Error(`task ${task.local_id} has no title`);
    if (!task.type) throw new Error(`task ${task.local_id} has no type`);
    if (!task.priority) throw new Error(`task ${task.local_id} has no priority`);
    if (!allowedTargetLists.has(task.target_list)) {
      throw new Error(`task ${task.local_id}: target_list must be one of ${[...allowedTargetLists].join(", ")}`);
    }

    const requiredArrays = ["acceptance_criteria", "implementation_notes", "blockers", "dependencies", "labels", "source_references"];
    for (const field of requiredArrays) {
      if (!Array.isArray(task[field])) {
        throw new Error(`task ${task.local_id}: ${field} must be an array, got ${typeof task[field]}`);
      }
    }
    for (const field of ["implementation_steps", "out_of_scope"]) {
      if (!Array.isArray(task[field])) {
        throw new Error(`task ${task.local_id}: ${field} must be an array, got ${typeof task[field]}`);
      }
    }
    if (!task.implementation_boundary || typeof task.implementation_boundary !== "object" || Array.isArray(task.implementation_boundary)) {
      throw new Error(`task ${task.local_id}: implementation_boundary must be an object`);
    }
    for (const field of ["paths", "entrypoints", "modules"]) {
      if (!Array.isArray(task.implementation_boundary[field])) {
        throw new Error(`task ${task.local_id}: implementation_boundary.${field} must be an array`);
      }
    }
    if (!task.verification || typeof task.verification !== "object" || Array.isArray(task.verification)) {
      throw new Error(`task ${task.local_id}: verification must be an object`);
    }
    if (typeof task.verification.command !== "string" || typeof task.verification.expected_result !== "string") {
      throw new Error(`task ${task.local_id}: verification.command and verification.expected_result must be strings`);
    }
  }
}

function getApiKey() {
  return process.env.BOARD_API_KEY || "";
}

// ─── Output ───

const output = {
  created_count: 0,
  created_ids: {},
  moved_ids: {},
  dependency_edges_created: [],
  failures: [],
  warnings: [],
  board_url: "",
};

function finish() {
  output.created_count = Object.keys(output.created_ids).length;
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

function fail(localId, reason, retryable = false) {
  output.failures.push({ local_id: localId, reason, retryable });
}

const POSITION_STEP = 5000;
const boardCache = new Map();
const PRIORITY_ORDER = { high: 0, medium: 1, low: 2 };

// ─── API helpers ───

function getAgent(urlStr) {
  return urlStr.startsWith("https") ? https : http;
}

function apiReq(input, key, method, urlPath, data) {
  return new Promise((resolve, reject) => {
    const board = getBoardTarget(input);
    const baseUrl = board.base_url.replace(/\/+$/, "");
    const urlStr = `${baseUrl}${urlPath.startsWith("/") ? urlPath : `/${urlPath}`}`;
    const parsed = new URL(urlStr);

    const headers = {
      "Content-Type": "application/json",
      Accept: "application/json",
    };
    if (key) headers["X-Api-Key"] = key;

    const payload = data ? JSON.stringify(data) : undefined;
    if (payload) headers["Content-Length"] = Buffer.byteLength(payload);

    const req = getAgent(urlStr).request(
      {
        hostname: parsed.hostname,
        port: parsed.port,
        path: parsed.pathname + parsed.search,
        method,
        headers,
      },
      (res) => {
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => {
          const body = Buffer.concat(chunks).toString("utf8");
          let parsedBody = {};
          try {
            parsedBody = body ? JSON.parse(body) : {};
          } catch {
            reject(new Error(`${method} ${urlPath} returned non-JSON response (${res.statusCode})`));
            return;
          }

          if (res.statusCode >= 400) {
            reject(new Error(`${method} ${urlPath} ${res.statusCode}: ${JSON.stringify(parsedBody).slice(0, 200)}`));
          } else {
            resolve(parsedBody);
          }
        });
      }
    );
    req.on("error", reject);
    if (payload) req.write(payload);
    req.end();
  });
}

// ─── Dependency helpers ───

function taskLabel(task) {
  return `${task.local_id} - ${task.title}`;
}

function buildDependencyContext(tasks, dependencyEdges) {
  const taskById = new Map(tasks.map((task) => [task.local_id, task]));
  const unblocksById = new Map(tasks.map((task) => [task.local_id, []]));

  for (const edge of dependencyEdges || []) {
    if (edge?.type !== "blocks") continue;
    if (!taskById.has(edge.from) || !taskById.has(edge.to)) continue;
    unblocksById.get(edge.from).push(edge.to);
  }

  return { taskById, unblocksById };
}

function priorityRank(task) {
  return PRIORITY_ORDER[task.priority] ?? 9;
}

function originalIndexById(tasks) {
  return new Map(tasks.map((task, index) => [task.local_id, index]));
}

function compareTasks(indexById) {
  return (a, b) => {
    const priorityDelta = priorityRank(a) - priorityRank(b);
    if (priorityDelta) return priorityDelta;
    return (indexById.get(a.local_id) ?? 0) - (indexById.get(b.local_id) ?? 0);
  };
}

function sortTasksForExecution(tasks) {
  const taskById = new Map(tasks.map((task) => [task.local_id, task]));
  const indexById = originalIndexById(tasks);
  const compare = compareTasks(indexById);
  const indegree = new Map(tasks.map((task) => [task.local_id, 0]));
  const dependents = new Map(tasks.map((task) => [task.local_id, []]));

  for (const task of tasks) {
    for (const dependencyId of task.dependencies) {
      if (!taskById.has(dependencyId)) {
        throw new Error(`task ${task.local_id}: unknown dependency ${dependencyId}`);
      }
      indegree.set(task.local_id, indegree.get(task.local_id) + 1);
      dependents.get(dependencyId).push(task.local_id);
    }
  }

  let ready = tasks.filter((task) => indegree.get(task.local_id) === 0).sort(compare);
  const ordered = [];

  while (ready.length) {
    const task = ready.shift();
    ordered.push(task);

    const newlyReady = [];
    for (const dependentId of dependents.get(task.local_id)) {
      const nextCount = indegree.get(dependentId) - 1;
      indegree.set(dependentId, nextCount);
      if (nextCount === 0) newlyReady.push(taskById.get(dependentId));
    }
    ready = ready.concat(newlyReady).sort(compare);
  }

  if (ordered.length !== tasks.length) {
    const cyclicIds = [...indegree.entries()].filter(([, count]) => count > 0).map(([id]) => id).sort();
    throw new Error(`Dependency cycle detected among: ${cyclicIds.join(", ")}`);
  }

  return ordered;
}

// ─── Description builder ───

function getCardDescription(task, context, executionIndex) {
  const blockedBy = task.dependencies.map((id) => context.taskById.get(id)).filter(Boolean).map(taskLabel);
  const unblocks = (context.unblocksById.get(task.local_id) || []).map((id) => context.taskById.get(id)).filter(Boolean).map(taskLabel);
  const boundary = task.implementation_boundary || { paths: [], entrypoints: [], modules: [] };
  const lines = [
    `**${task.type}** (priority: ${task.priority})`,
    `**Execution order:** ${executionIndex + 1}`,
    ...(boundary.paths.length ? ["", "**Paths:**", ...boundary.paths.map((p) => `- ${p}`)] : []),
    ...(boundary.entrypoints.length ? ["", "**Entrypoints:**", ...boundary.entrypoints.map((p) => `- ${p}`)] : []),
    ...(boundary.modules.length ? ["", "**Modules:**", ...boundary.modules.map((p) => `- ${p}`)] : []),
    ...(task.implementation_steps.length ? ["", "**Implementation steps:**", ...task.implementation_steps.map((step) => `- [ ] ${step}`)] : []),
    ...(task.verification.command || task.verification.expected_result ? ["", "**Verification:**", ...(task.verification.command ? [`- Command: \`${task.verification.command}\``] : []), ...(task.verification.expected_result ? [`- Expected: ${task.verification.expected_result}`] : [])] : []),
    ...(task.out_of_scope.length ? ["", "**Out of scope:**", ...task.out_of_scope.map((item) => `- ${item}`)] : []),
    "",
    "**Acceptance criteria:**",
    ...task.acceptance_criteria.map((c) => `- [ ] ${c}`),
    ...(blockedBy.length ? ["", `**Blocked by:** ${blockedBy.join(", ")}`] : []),
    ...(task.blockers.length ? ["", `**External blockers:** ${task.blockers.join(", ")}`] : []),
    ...(unblocks.length ? ["", `**Unblocks:** ${unblocks.join(", ")}`] : []),
    ...(task.implementation_notes.length ? ["", `**Notes:** ${task.implementation_notes.join(", ")}`] : []),
    ...(task.source_references.length ? ["", `**Sources:** ${task.source_references.join(", ")}`] : []),
  ].join("\n");

  return task.description
    ? task.description + "\n\n---\n\n" + lines
    : lines;
}

async function readStdin() {
  let data = "";
  for await (const chunk of process.stdin) data += chunk;
  return data.trim();
}

function responseItem(resp, method, urlPath) {
  if (!resp || typeof resp !== "object" || !resp.item) {
    throw new Error(`${method} ${urlPath} returned no item`);
  }
  return resp.item;
}

async function getBoardSnapshot(input, key) {
  const board = getBoardTarget(input);
  const cacheKey = board.board_id;
  if (!boardCache.has(cacheKey)) {
    const resp = await apiReq(input, key, "GET", `/api/boards/${board.board_id}`);
    if (!resp.included) throw new Error(`GET /api/boards/${board.board_id} returned no included data`);
    boardCache.set(cacheKey, resp);
  }
  return boardCache.get(cacheKey);
}

function invalidateBoardSnapshot(input) {
  boardCache.delete(getBoardTarget(input).board_id);
}

function getTaskLabelKeys(input) {
  return [...new Set(input.tasks_to_create.flatMap((task) => task.labels || []))];
}

function validateBoardConfig(input) {
  const board = getBoardTarget(input);
  const missingLabels = getTaskLabelKeys(input).filter((label) => !board.labels[label]);
  if (missingLabels.length) {
    throw new Error(`Missing color mapping for labels: ${missingLabels.sort().join(", ")}`);
  }
}

// ─── Card creation ───

async function createCard(input, key, task, index, context) {
  const board = getBoardTarget(input);
  const inferredList = task.target_list;
  let listId = board.lists[inferredList];
  if (!listId) {
    const boardResp = await getBoardSnapshot(input, key);
    const lists = boardResp.included?.lists || [];
    listId = lists.find((list) => normalizeListName(list.name) === inferredList.toLowerCase())?.id || lists[0]?.id || null;
  }
  if (!listId) {
    throw new Error(`No list found for target_list: ${inferredList}`);
  }

  const urlPath = `/api/lists/${listId}/cards`;
  const card = responseItem(await apiReq(input, key, "POST", urlPath, {
    name: `${task.local_id}: ${task.title}`,
    description: getCardDescription(task, context, index),
    type: "project",
    position: POSITION_STEP * (index + 1),
  }), "POST", urlPath);

  return card.id;
}

function normalizeListName(name) {
  return String(name || "")
    .replace(/['’]/g, "")
    .replace(/\s+/g, "")
    .replace(/^definitionsneeded$/i, "definitions")
    .replace(/^wontdo$/i, "wontdo")
    .replace(/^inprogress$/i, "doing")
    .toLowerCase();
}

// ─── Labels ───

async function getBoardLabels(input, key, boardId) {
  const boardResp = await getBoardSnapshot(input, key);
  return boardResp.included?.labels || [];
}

async function ensureLabel(input, key, boardId, labelKey) {
  const board = getBoardTarget(input);
  const color = board.labels[labelKey];
  if (!color) {
    throw new Error(`No color mapping for label: ${labelKey}`);
  }

  let matching = await getBoardLabels(input, key, boardId);
  if (!Array.isArray(matching)) {
    throw new Error(`Failed to fetch labels from board ${boardId}`);
  }

  const found = matching.find((l) => l.name === labelKey);
  if (found) return found.id;

  // Create missing label
  const urlPath = `/api/boards/${boardId}/labels`;
  const created = responseItem(await apiReq(input, key, "POST", urlPath, {
    name: labelKey,
    color,
    position: POSITION_STEP,
  }), "POST", urlPath);
  invalidateBoardSnapshot(input);
  return created.id;
}

async function applyLabels(input, key, task, cardId) {
  const board = getBoardTarget(input);
  const labelIds = [];
  for (const labelKey of task.labels) {
    const labelId = await ensureLabel(input, key, board.board_id, labelKey);
    if (labelId) labelIds.push(labelId);
  }

  if (!labelIds.length) return;

  // Apply one label at a time (API requires single labelId per POST)
  for (const labelId of [...new Set(labelIds)]) {
    await apiReq(input, key, "POST", `/api/cards/${cardId}/card-labels`, {
      labelId,
    });
  }
}

// ─── Checklists ───

async function createChecklist(input, key, task, cardId) {
  const taskListPath = `/api/cards/${cardId}/task-lists`;
  const taskList = responseItem(await apiReq(input, key, "POST", taskListPath, {
    position: POSITION_STEP,
    name: "Implementation",
    showOnFrontOfCard: true,
  }), "POST", taskListPath);

  const items = (task.acceptance_criteria || []).map((c) => ({
    position: POSITION_STEP,
    name: c,
  }));

  for (const item of items) {
    const taskPath = `/api/task-lists/${taskList.id}/tasks`;
    responseItem(await apiReq(input, key, "POST", taskPath, item), "POST", taskPath);
  }
}

// ─── Card moves ───

async function moveCards(input, key) {
  const board = getBoardTarget(input);
  for (const move of input.cards_to_move || []) {
    const moveKey = move.local_id || move.card_id;
    if (!moveKey) continue;

    const cardId = output.created_ids[move.local_id] || (move.card_id || null);
    if (!cardId) {
      fail(moveKey, "Card not found for move", false);
      continue;
    }

    const listId = board.lists[move.target_list];
    if (!listId) {
      fail(moveKey, `Unknown target_list: ${move.target_list}`, false);
      continue;
    }

    try {
      await apiReq(input, key, "PATCH", `/api/cards/${cardId}`, {
        boardId: board.board_id,
        listId,
        position: move.position || POSITION_STEP,
      });
      output.moved_ids[moveKey] = cardId;
    } catch (error) {
      fail(moveKey, `Move failed: ${error.message}`, false);
    }
  }
}

// ─── Main pipeline ───

async function writeTasks(input, key) {
  const board = getBoardTarget(input);
  output.board_url = `${board.base_url.replace(/\/+$/, "")}/boards/${board.board_id}`;
  validateBoardConfig(input);
  const orderedTasks = sortTasksForExecution(input.tasks_to_create);
  const dependencyContext = buildDependencyContext(input.tasks_to_create, input.dependency_edges);

  for (const [index, task] of orderedTasks.entries()) {
    let cardId = null;
    try {
      cardId = await createCard(input, key, task, index, dependencyContext);
      output.created_ids[task.local_id] = cardId;
      await applyLabels(input, key, task, cardId);
      await createChecklist(input, key, task, cardId);
    } catch (error) {
      if (cardId) {
        try {
          await apiReq(input, key, "DELETE", `/api/cards/${cardId}`);
          delete output.created_ids[task.local_id];
        } catch (rollbackError) {
          output.warnings.push(`${task.local_id}: rollback failed: ${rollbackError.message}`);
        }
      }
      fail(task.local_id, error.message, true);
      break;
    }
  }

  if (output.failures.length) return;
  await moveCards(input, key);
}

// ─── Main ───

async function main() {
  try {
    const raw = await readStdin();
    if (!raw) throw new Error("Empty stdin");

    const input = JSON.parse(raw);
    validateInput(input);
    validateBoardConfig(input);

    const apiKey = getApiKey();
    if (!apiKey) throw new Error("BOARD_API_KEY not set in environment");
    if (apiKey === "NONE") console.error("Warning: API key is 'NONE'");

    await writeTasks(input, apiKey);
  } catch (error) {
    const msg = error.message || String(error);
    if (!output.failures.length) {
      fail("*", msg, false);
    } else {
      output.warnings.push(msg);
    }
  }

  finish();
}

main();
