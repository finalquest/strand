#!/usr/bin/env node

const {
  getBoardSnapshot,
  loadBoardConfig,
  extractLocalId,
} = require("./board-card-common.cjs");

const output = { next_id: null, max_id: null, failures: [], warnings: [] };

function finish() {
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

async function main() {
  const board = loadBoardConfig();
  const snapshot = await getBoardSnapshot(board);
  const cards = snapshot.included?.cards || [];

  const ids = cards
    .map((card) => extractLocalId(card))
    .filter((id) => id)
    .map((id) => {
      const match = id.match(/T-(\d+)/);
      return match ? Number(match[1]) : 0;
    })
    .filter((n) => n > 0);

  const maxId = ids.length ? Math.max(...ids) : 0;
  const nextId = maxId + 1;

  output.max_id = maxId > 0 ? `T-${String(maxId).padStart(3, "0")}` : null;
  output.next_id = `T-${String(nextId).padStart(3, "0")}`;
}

main()
  .catch((error) => output.failures.push({ reason: error.message }))
  .finally(finish);
