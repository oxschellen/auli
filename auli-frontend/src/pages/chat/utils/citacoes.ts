/**
 * Citações por afirmação (`[Serviço 2]`, `[FAQ 4]` — D-SF-7) viram selos clicáveis (D-UI-7).
 *
 * O texto do modelo é markdown; a citação crua é um "link de referência" sem definição, que o
 * react-markdown desenha como texto com colchetes. Aqui ela vira um link de verdade para uma âncora
 * `#fonte-servico-2`, e o `SystemMessage` troca esse link pelo selo. Só os rótulos de serviços e
 * FAQs: são os únicos que o prompt pede e os únicos que a lista de fontes (D-SF-10) tem.
 */
const CITACAO = /\[(Servi[çc]o|FAQ)\s+(\d+)\](?!\()/gi;
const PREFIXO = "#fonte-";

/** "Servico 2" / "faq 4" → o rótulo canônico da lista de fontes: "Serviço 2", "FAQ 4". */
function canonico(tipo: string, n: string): string {
  return `${/^faq$/i.test(tipo) ? "FAQ" : "Serviço"} ${Number(n)}`;
}

/** "Serviço 2" → "fonte-servico-2" (sem acento nem espaço: vale como id de elemento e como URL). */
export function ancora(rotulo: string): string {
  return (
    "fonte-" +
    rotulo
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/\s+/g, "-")
  );
}

/** O inverso de `ancora`, a partir do href do link: "#fonte-servico-2" → "Serviço 2". */
export function rotuloDoHref(href: string | undefined): string | null {
  if (!href?.startsWith(PREFIXO)) return null;
  const m = /^#fonte-(servico|faq)-(\d+)$/.exec(href);
  return m ? canonico(m[1], m[2]) : null;
}

/** Troca cada citação por um link para a âncora da fonte. Citação já seguida de `(` é link do
 *  próprio modelo e fica como está. */
export function marcarCitacoes(md: string): string {
  return md.replace(CITACAO, (_, tipo: string, n: string) => {
    const rotulo = canonico(tipo, n);
    return `[${rotulo}](#${ancora(rotulo)})`;
  });
}

/** Os rótulos citados no texto, canônicos. */
export function rotulosCitados(md: string): Set<string> {
  const vistos = new Set<string>();
  for (const m of md.matchAll(CITACAO)) vistos.add(canonico(m[1], m[2]));
  return vistos;
}
