import axios from "axios";

/**
 * A saída do chat para a rede (I2 do docs/ESTUDO-frontend-organizacao.md): o endereço, a pergunta e
 * o registro de auditoria. Devolve resultados, sem mexer em estado — quem mostra a resposta e as
 * mensagens de erro é o chat. Os arquivos estáticos do acervo saem por `estaticos.ts`.
 */

/** Endpoint do chat. Override por ambiente com VITE_API_URL (ex.: um endpoint de staging);
 *  cai na produção para o app funcionar sem `.env` nenhum. */
export const API_URL = import.meta.env.VITE_API_URL ?? "https://api.auli.com.br/v1/question";

/** URL do registro de auditoria de UMA resposta — `GET /v1/log/{uuid}` (D-LOG-3).
 *
 *  **Derivada do `API_URL`, e não uma constante própria.** Duas constantes independentes
 *  divergiriam no dia em que alguém apontasse o `VITE_API_URL` para staging: o chat iria para lá e
 *  o log continuaria pedindo à produção, onde aquele UUID não existe — e o sintoma seria um 404
 *  inexplicável. Se o sufixo não casar, o resultado não é um endpoint de log e a falha aparece no
 *  modal, em vez de bater silenciosamente no lugar errado. */
const logUrl = (id: string) => `${API_URL.replace(/\/question$/, "/log")}/${id}`;

/** Aborta a pergunta se o servidor não responder nesta janela. 70 s (era 35 s) desde a triagem de
 *  serviços+FAQs (D-SF-9): a resposta pode levar DUAS chamadas ao LLM — até 20 s na triagem e 30 s
 *  na redação —, mais embed e busca. */
const LIMITE_MS = 70_000;

/** Um documento recuperado para a resposta, como a lista de fontes o mostra (D-SF-10). O `rotulo`
 *  é o mesmo da citação no texto (`"Serviço 2"`, `"FAQ 4"`), para o analista bater um com o outro.
 *  Mora aqui porque é contrato do servidor. */
export interface Fonte {
  rotulo: string;
  titulo: string;
  link: string;
  /** Veredito da triagem (D-SF-9). Ausente = o documento foi à resposta sem veredito (sem triagem,
   *  triagem que falhou, ou documento que o modelo omitiu). */
  triagem?: "aplica" | "condicional" | "descarta";
}

interface RespostaDoServidor {
  answer?: string;
  /** UUID do registro de auditoria (D-LOG-2). **Ausente quando a gravação falhou** — o chat não
   *  falha por causa do log, só deixa de oferecer o ícone. */
  log_id?: string;
  /** Fontes recuperadas (D-SF-10). Só vem em Serviços + FAQs; ausente nos demais tipos. */
  fontes?: Fonte[];
}

/** O que a pergunta deu. As falhas se distinguem porque cada uma pede uma mensagem diferente:
 *  `limite` é o HTTP 429, e o servidor manda nele um texto amigável em pt-BR (`mensagem`). */
export type Resultado =
  | { ok: true; texto?: string; logId?: string; fontes?: Fonte[] }
  | { ok: false; falha: "tempo" | "limite" | "indisponivel"; mensagem?: string };

/** Envia a pergunta. Não lança: toda falha volta como `Resultado`. `tipo` é o tipo de consulta já
 *  numérico, como o backend o lê (`QueryType::from_code`). */
export async function perguntar(
  pergunta: string,
  entityId: string | undefined,
  tipo: number,
): Promise<Resultado> {
  const controle = new AbortController();
  const relogio = setTimeout(() => controle.abort(), LIMITE_MS);
  try {
    const { data } = await axios.post<RespostaDoServidor>(
      API_URL,
      { question: pergunta, ...(entityId ? { entity: entityId } : {}), type: tipo },
      { signal: controle.signal },
    );
    return { ok: true, texto: data?.answer, logId: data?.log_id, fontes: data?.fontes };
  } catch (e) {
    // Tempo esgotado (abortado) → limite (429) → qualquer outra coisa (servidor fora, rede). Sem
    // essa distinção, quem foi limitado ouviria que o servidor está fora do ar.
    if (axios.isCancel(e)) return { ok: false, falha: "tempo" };
    const r = (e as { response?: { status?: number; data?: { error?: string } } }).response;
    if (r?.status === 429) return { ok: false, falha: "limite", mensagem: r.data?.error };
    return { ok: false, falha: "indisponivel" };
  } finally {
    clearTimeout(relogio);
  }
}

/** O registro de auditoria de uma resposta, como texto. Lança com a mensagem do servidor quando
 *  não há registro: o backend manda a mensagem amigável no corpo do 404/400, e ela é a mesma para
 *  "nunca existiu" e "podado", de propósito. */
export async function buscarLog(id: string): Promise<string> {
  const r = await fetch(logUrl(id));
  const texto = await r.text();
  if (!r.ok) throw new Error(texto);
  return texto;
}
