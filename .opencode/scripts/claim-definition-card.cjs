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

const output = { claimed: null, prompt_context: null, failures: [], warnings: [] };

function finish() {
  process.stdout.write(`${JSON.stringify(output)}\n`);
}

function sortByExecutionOrder(a, b) {
  const aOrder = normalizeCard({ base_url: "", board_id: "", lists: {} }, a).execution_order;
  const bOrder = normalizeCard({ base_url: "", board_id: "", lists: {} }, b).execution_order;
  if (aOrder !== bOrder) return aOrder - bOrder;
  return Number(a.position || 0) - Number(b.position || 0);
}

function buildPromptContext(card) {
  return [
    `You are defining Planka card ${card.local_id || card.id}: ${card.title}.`,
    "Use the card description as the definition contract.",
    "Rules: read handoff.md first; define only this card; do not implement code; do not do out-of-scope items; preserve unrelated changes; run the verification command from the card; if blocked, stop and report the blocker.",
    `Card URL: ${card.url}`,
    "",
    card.description,
  ].join("\n");
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const explicitCard = args.card || args._[0] || "";
  const dryRun = Boolean(args["dry-run"]);

  const board = loadBoardConfig();
  const snapshot = await getBoardSnapshot(board);
  const cards = snapshot.included?.cards || [];
  const definitionsListId = board.lists.definitions;
  const doingListId = board.lists.doing;
  if (!definitionsListId) throw new Error("Missing definitions list id");
  if (!doingListId) throw new Error("Missing doing list id");

  let card = explicitCard ? findCard(cards, explicitCard) : null;
  if (!card) {
    const definitionCards = cards.filter((candidate) => candidate.listId === definitionsListId).sort(sortByExecutionOrder);
    card = definitionCards[0] || null;
  }
  if (!card) {
    throw new Error(explicitCard ? `Card not found: ${explicitCard}` : "No definition card available");
  }
  if (explicitCard && card.listId !== definitionsListId) {
    const current = normalizeCard(board, card);
    throw new Error(`Card ${current.local_id || current.id} is in ${current.list}, not definitions`);
  }

  if (!dryRun) {
    const position = nextPosition(cards, doingListId);
    const response = await apiReq(board, "PATCH", `/api/cards/${card.id}`, { listId: doingListId, position });
    card = response.item || { ...card, listId: doingListId, position };
  }

  const normalized = normalizeCard(board, card);
  output.claimed = normalized;
  output.prompt_context = buildPromptContext(normalized);
  if (dryRun) output.warnings.push("dry-run: card was not moved to Doing");
}

main()
  .catch((error) => output.failures.push({ reason: error.message }))
  .finally(finish);
