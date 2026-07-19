import { parseQuery, buildHaystack, haystackMatches } from "../../shared/textSearch";

/** A single linkable item within a content category. */
export interface ConteudoItem {
  title: string;
  url?: string;
  type?: string;
}

/** A named group of content items, as stored in conteudo_site_tree.json. */
export interface ConteudoCategory {
  name: string;
  items: ConteudoItem[];
}

/** Root of conteudo_site_tree.json. */
export interface ConteudoTree {
  categories: ConteudoCategory[];
}

export function searchCategories(
  categories: ConteudoCategory[],
  query: string,
): ConteudoCategory[] {
  const terms = parseQuery(query);
  if (terms.length === 0) return categories;
  const result: ConteudoCategory[] = [];
  for (const cat of categories) {
    const items = cat.items.filter((item) => haystackMatches(buildHaystack([item.title]), terms));
    if (items.length > 0) result.push({ ...cat, items });
  }
  return result;
}
