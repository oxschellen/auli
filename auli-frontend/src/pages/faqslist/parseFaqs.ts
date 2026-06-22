/** A single Q&A pair as stored in faqs.json. */
interface RawFaqItem {
  pergunta: string;
  resposta: string;
}

/** Raw node shape as it appears in faqs.json (and its recursive children). */
export interface RawFaqNode {
  title?: string;
  url?: string | null;
  page_type?: string;
  children?: RawFaqNode[];
  faq_items?: RawFaqItem[];
}

/** Normalized tree node consumed by the FAQ UI. */
export interface FaqNode {
  id: string;
  text: string;
  url: string | null;
  children: FaqNode[];
}

/** A node plus the chain of ancestors that led to it (used by search results). */
export interface FaqSearchHit {
  node: FaqNode;
  ancestors: FaqNode[];
}

let _id = 0;

function makeId(): string {
  return `node-${_id++}`;
}

export function buildNodesFromJson(json: RawFaqNode): FaqNode[] {
  _id = 0;
  return (json.children || []).map(convertJsonNode);
}

function convertJsonNode(jsonNode: RawFaqNode): FaqNode {
  const children: FaqNode[] = [];

  for (const child of jsonNode.children || []) {
    children.push(convertJsonNode(child));
  }

  for (const item of jsonNode.faq_items || []) {
    children.push({
      id: makeId(),
      text: item.pergunta,
      url: jsonNode.url || null,
      children: [],
    });
  }

  return {
    id: makeId(),
    text: jsonNode.title ?? "",
    url: jsonNode.url || null,
    children,
  };
}

export function searchNodes(
  nodes: FaqNode[],
  query: string,
  ancestors: FaqNode[] = [],
): FaqSearchHit[] {
  const results: FaqSearchHit[] = [];
  const q = query.toLowerCase();

  for (const node of nodes) {
    if (node.text.toLowerCase().includes(q)) {
      results.push({ node, ancestors });
    }
    if (node.children.length > 0) {
      results.push(...searchNodes(node.children, query, [...ancestors, node]));
    }
  }

  return results;
}

export function getEffectiveUrl(node: FaqNode, ancestors: FaqNode[]): string | null {
  if (node.url) return node.url;
  for (let i = ancestors.length - 1; i >= 0; i--) {
    if (ancestors[i].url) return ancestors[i].url;
  }
  return null;
}

export function buildPageTypeMap(
  jsonNode: RawFaqNode,
  map: Map<string, string> = new Map(),
): Map<string, string> {
  if (jsonNode.url && jsonNode.page_type) {
    map.set(jsonNode.url.replace(/\/$/, ""), jsonNode.page_type);
  }
  if (Array.isArray(jsonNode.children)) {
    for (const child of jsonNode.children) {
      buildPageTypeMap(child, map);
    }
  }
  return map;
}

export function buildAnswerMap(
  jsonNode: RawFaqNode,
  map: Map<string, string> = new Map(),
): Map<string, string> {
  if (Array.isArray(jsonNode.faq_items)) {
    for (const item of jsonNode.faq_items) {
      map.set(item.pergunta.trim().toLowerCase(), item.resposta);
    }
  }
  if (Array.isArray(jsonNode.children)) {
    for (const child of jsonNode.children) {
      buildAnswerMap(child, map);
    }
  }
  return map;
}
