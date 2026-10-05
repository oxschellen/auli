/**
 * As seções da resposta do chat — "Resumo" e "Detalhes" — separadas do texto corrido (D-UI-5).
 *
 * O prompt pede as duas seções com o rótulo em negrito, e o modelo escreve de jeitos diferentes:
 * `**Resumo**` sozinho na linha, `**Resumo:** texto` na mesma linha, `### Resumo`. Renderizado como
 * markdown puro, o rótulo em negrito COLA no parágrafo seguinte ("**Resumo** Para apurar…") e a
 * estrutura que o prompt pediu some. Separar as seções aqui permite desenhar o Resumo como bloco
 * de destaque e o "Detalhes" como título — sem depender de o modelo acertar o markdown.
 *
 * Só é rótulo a linha em que a palavra está MARCADA (negrito, título ou dois-pontos) ou sozinha:
 * "Resumo do caso: …" no meio do texto é prosa, não seção.
 */
export interface Secoes {
  /** O que vem antes da primeira seção (normalmente vazio). */
  antes: string;
  resumo: string | null;
  detalhes: string | null;
}

const ROTULO =
  /^[ \t]*(#{1,6}[ \t]*)?(\*\*|__)?[ \t]*(resumo|detalhes)[ \t]*(:)?[ \t]*(\*\*|__)?[ \t]*(:)?[ \t]*(.*)$/i;

type Nome = "antes" | "resumo" | "detalhes";

export function dividirSecoes(texto: string): Secoes {
  const partes: Record<Nome, string[]> = { antes: [], resumo: [], detalhes: [] };
  const vistas = new Set<Nome>();
  let atual: Nome = "antes";
  let emCodigo = false;

  for (const linha of texto.split("\n")) {
    if (/^\s*```/.test(linha)) emCodigo = !emCodigo;
    const m = emCodigo ? null : ROTULO.exec(linha);
    if (m) {
      const [, titulo, negritoA, palavra, doisPontosA, negritoB, doisPontosB, resto] = m;
      const negrito = Boolean(negritoA && negritoB);
      const marcado = Boolean(titulo) || negrito || Boolean(doisPontosA || doisPontosB);
      const nome = palavra.toLowerCase() as Nome;
      // Negrito só abre de um lado ("**Resumo texto") não é rótulo; e a seção não se repete.
      if ((marcado || resto.trim() === "") && Boolean(negritoA) === Boolean(negritoB) && !vistas.has(nome)) {
        atual = nome;
        vistas.add(nome);
        if (resto.trim() !== "") partes[atual].push(resto);
        continue;
      }
    }
    partes[atual].push(linha);
  }

  const juntar = (n: Nome) => partes[n].join("\n").trim();
  return {
    antes: juntar("antes"),
    resumo: vistas.has("resumo") ? juntar("resumo") : null,
    detalhes: vistas.has("detalhes") ? juntar("detalhes") : null,
  };
}
