#!/usr/bin/env node

const {
  apiReq,
  findCard,
  getBoardSnapshot,
  loadBoardConfig,
  nextPosition,
  normalizeCard,
  parseArgs,
} = require("./board-card-common.cjs");

const output = { moved: null, failures: [], warnings: [] };

function finish() {
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const selector = args.card || args._[0];
  const targetListName = args.list || args._[1];
  if (!selector || !targetListName) throw new Error("Usage: move-board-card.cjs --card <CARD_ID|T-000> --list <ready|doing|review|blocked|done|backlog|definitions|wontDo>");

  const board = loadBoardConfig();
  const targetListId = board.lists[targetListName];
  if (!targetListId) throw new Error(`Unknown target list: ${targetListName}`);

  const snapshot = await getBoardSnapshot(board);
  const cards = snapshot.included?.cards || [];
  const card = findCard(cards, selector);
  if (!card) throw new Error(`Card not found: ${selector}`);

  const position = nextPosition(cards, targetListId);
  const response = await apiReq(board, "PATCH", `/api/cards/${card.id}`, { listId: targetListId, position });
  output.moved = normalizeCard(board, response.item || { ...card, listId: targetListId, position });
}

main()
  .catch((error) => output.failures.push({ reason: error.message }))
  .finally(finish);
