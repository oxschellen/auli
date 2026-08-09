/** Endpoint do chat. Override por ambiente com VITE_API_URL (ex.: um endpoint de staging);
 *  cai na produção para o app funcionar sem `.env` nenhum. */
export const API_URL =
  import.meta.env.VITE_API_URL ?? "https://api.auli.com.br/v1/question";

/** URL do registro de auditoria de UMA resposta — `GET /v1/log/{uuid}` (D-LOG-3).
 *
 *  **Derivada do `API_URL`, e não uma constante própria.** Duas constantes independentes
 *  divergiriam no dia em que alguém apontasse o `VITE_API_URL` para staging: o chat iria para lá e
 *  o log continuaria pedindo à produção, onde aquele UUID não existe — e o sintoma seria um 404
 *  inexplicável. Se o sufixo não casar, o resultado não é um endpoint de log e a falha aparece no
 *  modal, em vez de bater silenciosamente no lugar errado. */
export const logUrl = (id: string) => `${API_URL.replace(/\/question$/, "/log")}/${id}`;
