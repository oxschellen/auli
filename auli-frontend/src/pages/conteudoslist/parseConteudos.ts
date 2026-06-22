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
  if (!query.trim()) return categories;
  const q = query.toLowerCase();
  const result: ConteudoCategory[] = [];
  for (const cat of categories) {
    const items = cat.items.filter((item) => item.title.toLowerCase().includes(q));
    if (items.length > 0) result.push({ ...cat, items });
  }
  return result;
}
