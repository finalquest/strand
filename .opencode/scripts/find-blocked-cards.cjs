#!/usr/bin/env node

const {
  getBoardSnapshot,
  loadBoardConfig,
  normalizeCard,
  parseArgs,
} = require("./board-card-common.cjs");

const output = { blocked_cards: [], failures: [], warnings: [] };

function finish() {
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

function sortByExecutionOrder(a, b) {
  const aOrder = a.execution_order || Number.MAX_SAFE_INTEGER;
  const bOrder = b.execution_order || Number.MAX_SAFE_INTEGER;
  if (aOrder !== bOrder) return aOrder - bOrder;
  return Number(a.position || 0) - Number(b.position || 0);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const board = loadBoardConfig();
  const snapshot = await getBoardSnapshot(board);
  const cards = snapshot.included?.cards || [];
  const blockedListId = board.lists.blocked;
  
  if (!blockedListId) throw new Error("Missing blocked list id in config");

  const blockedCards = cards
    .filter((card) => card.listId === blockedListId)
    .map((card) => normalizeCard(board, card))
    .sort(sortByExecutionOrder);

  output.blocked_cards = blockedCards;
  
  if (blockedCards.length === 0) {
    output.warnings.push("No blocked cards found");
  }
}

main()
  .catch((error) => output.failures.push({ reason: error.message }))
  .finally(finish);
