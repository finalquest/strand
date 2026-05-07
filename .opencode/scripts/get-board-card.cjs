#!/usr/bin/env node

const {
  findCard,
  getBoardSnapshot,
  loadBoardConfig,
  normalizeCard,
  parseArgs,
} = require("./board-card-common.cjs");

const output = { card: null, failures: [], warnings: [] };

function finish() {
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const selector = args.card || args._[0];
  if (!selector) throw new Error("Usage: get-board-card.cjs --card <CARD_ID|T-000|title-fragment>");

  const board = loadBoardConfig();
  const snapshot = await getBoardSnapshot(board);
  const cards = snapshot.included?.cards || [];
  const card = findCard(cards, selector);
  if (!card) throw new Error(`Card not found: ${selector}`);
  output.card = normalizeCard(board, card);
}

main()
  .catch((error) => output.failures.push({ reason: error.message }))
  .finally(finish);
