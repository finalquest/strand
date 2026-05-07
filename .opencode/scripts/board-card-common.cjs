const fs = require("fs");
const path = require("path");
const https = require("https");
const http = require("http");

const CONFIG_FILE = path.join(__dirname, "..", "config", "spec-to-kanban.json");

function loadBoardConfig() {
  const config = JSON.parse(fs.readFileSync(CONFIG_FILE, "utf8"));
  const board = config.planka || config.board || config;
  if (!board.base_url) throw new Error("Missing board base_url");
  if (!board.board_id) throw new Error("Missing board_id");
  if (!board.lists || typeof board.lists !== "object") throw new Error("Missing board lists");
  return board;
}

function getApiKey() {
  const apiKey = process.env.BOARD_API_KEY || "";
  if (!apiKey) throw new Error("BOARD_API_KEY not set in environment");
  return apiKey;
}

function getAgent(urlStr) {
  return urlStr.startsWith("https") ? https : http;
}

function apiReq(board, method, urlPath, data) {
  return new Promise((resolve, reject) => {
    const baseUrl = board.base_url.replace(/\/+$/, "");
    const urlStr = `${baseUrl}${urlPath.startsWith("/") ? urlPath : `/${urlPath}`}`;
    const parsed = new URL(urlStr);
    const payload = data ? JSON.stringify(data) : undefined;
    const headers = {
      Accept: "application/json",
      "Content-Type": "application/json",
      "X-Api-Key": getApiKey(),
    };
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
        res.on("data", (chunk) => chunks.push(chunk));
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

async function getBoardSnapshot(board) {
  return apiReq(board, "GET", `/api/boards/${board.board_id}`);
}

function listNameById(board) {
  const result = new Map();
  for (const [name, id] of Object.entries(board.lists || {})) result.set(id, name);
  return result;
}

function boardUrl(board) {
  return `${String(board.base_url || "").replace(/\/+$/, "")}/boards/${board.board_id}`;
}

function cardUrl(board, card) {
  return `${boardUrl(board)}/cards/${card.id}`;
}

function extractLocalId(card) {
  const name = card.name || "";
  const match = name.match(/\bT-\d+\b/);
  return match ? match[0] : "";
}

function extractExecutionOrder(card) {
  const description = card.description || "";
  const match = description.match(/\*\*Execution order:\*\*\s*(\d+)/i);
  return match ? Number(match[1]) : Number.MAX_SAFE_INTEGER;
}

function normalizeCard(board, card) {
  const names = listNameById(board);
  return {
    id: card.id,
    local_id: extractLocalId(card),
    title: card.name || "",
    description: card.description || "",
    list_id: card.listId,
    list: names.get(card.listId) || card.listId || "",
    position: card.position ?? null,
    execution_order: extractExecutionOrder(card),
    url: cardUrl(board, card),
  };
}

function findCard(cards, selector) {
  const wanted = String(selector || "").trim();
  if (!wanted) return null;
  return cards.find((card) => card.id === wanted || extractLocalId(card) === wanted || (card.name || "").includes(wanted)) || null;
}

function nextPosition(cards, listId) {
  const positions = cards.filter((card) => card.listId === listId).map((card) => Number(card.position || 0));
  return positions.length ? Math.max(...positions) + 5000 : 5000;
}

function parseArgs(argv) {
  const args = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const value = argv[i];
    if (value.startsWith("--")) {
      const key = value.slice(2);
      const next = argv[i + 1];
      if (!next || next.startsWith("--")) {
        args[key] = true;
      } else {
        args[key] = next;
        i += 1;
      }
    } else {
      args._.push(value);
    }
  }
  return args;
}

module.exports = {
  apiReq,
  boardUrl,
  cardUrl,
  extractExecutionOrder,
  extractLocalId,
  findCard,
  getBoardSnapshot,
  listNameById,
  loadBoardConfig,
  nextPosition,
  normalizeCard,
  parseArgs,
};
