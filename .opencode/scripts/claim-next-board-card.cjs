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
    `You are implementing Planka card ${card.local_id || card.id}: ${card.title}.`,
    "Use the card description as the implementation contract.",
    "Rules: read handoff.md first; implement only this card; do not do out-of-scope items; preserve unrelated changes; run the verification command from the card; if blocked, stop and report the blocker.",
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
  const readyListId = board.lists.ready;
  const doingListId = board.lists.doing;
  if (!readyListId) throw new Error("Missing ready list id");
  if (!doingListId) throw new Error("Missing doing list id");

  let card = explicitCard ? findCard(cards, explicitCard) : null;
  if (!card) {
    const readyCards = cards.filter((candidate) => candidate.listId === readyListId).sort(sortByExecutionOrder);
    card = readyCards[0] || null;
  }
  if (!card) {
    const blockedListId = board.lists.blocked;
    const blockedCards = cards.filter((candidate) => candidate.listId === blockedListId).sort(sortByExecutionOrder);
    if (!explicitCard && blockedCards.length > 0) {
      const nextBlocked = normalizeCard(board, blockedCards[0]);
      throw new Error(`No Ready card available. Next blocked card: ${nextBlocked.local_id || nextBlocked.id}. Run check-blockers-agent to unblock.`);
    }
    throw new Error(explicitCard ? `Card not found: ${explicitCard}` : "No Ready card available");
  }
  if (explicitCard && card.listId !== readyListId) {
    const current = normalizeCard(board, card);
    throw new Error(`Card ${current.local_id || current.id} is in ${current.list}, not Ready`);
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
