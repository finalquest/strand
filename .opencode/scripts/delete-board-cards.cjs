#!/usr/bin/env node

const fs = require("fs");
const path = require("path");
const https = require("https");
const http = require("http");

const CONFIG_FILE = path.join(__dirname, "..", "config", "spec-to-kanban.json");
const config = JSON.parse(fs.readFileSync(CONFIG_FILE, "utf8"));
const board = config.planka || config.board || config;
const apiKey = process.env.BOARD_API_KEY || "";

const output = {
  board_url: `${String(board.base_url || "").replace(/\/+$/, "")}/boards/${board.board_id}`,
  deleted_count: 0,
  deleted_ids: [],
  failures: [],
  warnings: [],
};

function finish() {
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

function getAgent(urlStr) {
  return urlStr.startsWith("https") ? https : http;
}

function apiReq(method, urlPath, data) {
  return new Promise((resolve, reject) => {
    const baseUrl = board.base_url.replace(/\/+$/, "");
    const urlStr = `${baseUrl}${urlPath.startsWith("/") ? urlPath : `/${urlPath}`}`;
    const parsed = new URL(urlStr);
    const payload = data ? JSON.stringify(data) : undefined;
    const headers = {
      Accept: "application/json",
      "Content-Type": "application/json",
      "X-Api-Key": apiKey,
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

async function main() {
  if (!board.base_url) throw new Error("Missing board base_url");
  if (!board.board_id) throw new Error("Missing board_id");
  if (!apiKey) throw new Error("BOARD_API_KEY not set in environment");

  const snapshot = await apiReq("GET", `/api/boards/${board.board_id}`);
  const boardListIds = new Set(Object.values(board.lists || {}));
  const cards = (snapshot.included?.cards || []).filter((card) => boardListIds.has(card.listId));

  for (const card of cards) {
    try {
      await apiReq("DELETE", `/api/cards/${card.id}`);
      output.deleted_ids.push(card.id);
      output.deleted_count += 1;
    } catch (error) {
      output.failures.push({ card_id: card.id, name: card.name || "", reason: error.message });
    }
  }
}

main()
  .catch((error) => {
    output.failures.push({ card_id: "*", name: "*", reason: error.message });
  })
  .finally(finish);
