# TAREFA-LOG-DA-PERGUNTA — Ícone no chat que abre o log de auditoria daquela pergunta

**Decisões já tomadas com o Carlos (não rediscutir):** visível para TODOS os usuários (opção A);
ícone do MESMO tamanho e estilo do ícone de cópia que já existe na mensagem; conteúdo num MODAL
sobre o chat (não rota nova); segurança por **capability-UUID** — cada pergunta ganha um UUID
aleatório que só volta a quem perguntou; sem endpoint de listagem.

**Topologia (para não haver dúvida):** os arquivos de log vivem no disco do Desktop Server — e a
rota nova mora NO MESMO LUGAR: é um handler do `auli-cli/api`, no mesmo binário que grava o log.
O frontend (estático no VPS) só consome `api.auli.com.br` via o túnel cloudflared, exatamente como
já faz para o chat. Nenhum log é copiado ao VPS; nada muda no deploy do estático; cada log sai da
máquina apenas um por vez, sob demanda, para quem possui o UUID daquela pergunta.

**Premissa de segurança preservada (ler antes de codar):** o log de auditoria é deliberadamente
ÍNTEGRO — contém a pergunta original (pré-anonimização) — e a proteção dele é "por acesso".
Esta TAREFA muda o acesso de "filesystem da máquina" para "quem possui o UUID daquela pergunta",
o que mantém a premissa: o UUID é aleatório, não listável, e só é entregue na resposta ao próprio
autor. Ninguém alcança log alheio; cada um vê apenas o que ele mesmo digitou + mecânica interna
sobre acervo público (contexto RAG, distâncias, aderência).

**Sinergia registrada:** esta tela é a interface de leitura da base de calibragem do
`docs/ESTUDO-busca-hibrida.md` §9 — cada pergunta real vira um caso inspecionável a um clique.

---

## Decisões

- **D-LOG-1 (identidade):** **UUID v7** gerado **dentro do `log_question`**, que passa a devolvê-lo,
  para as **duas faces** (chat e MCP). O nome do arquivo passa a incluí-lo — manter o prefixo atual
  (ordenação humana por data preservada) e acrescentar `_<uuid>` antes da extensão. **Logs antigos
  ficam como estão**: sem UUID, não são linkáveis; nenhuma migração ou renomeação retroativa.
  - *Revisto em 09/08 (era "v4, gerado no handler do chat").* **v7** porque seus 48 bits mais
    significativos são o timestamp em ms: o nome ordena cronologicamente até o milissegundo,
    desempatando dentro do mesmo segundo sem mexer no prefixo. Custa entropia (74 bits contra 122
    do v4) — irrelevante para uma capability, e o instante que ele revela já está no nome.
    **No `log_question`, e não no handler**, porque é ele quem grava (`rag.rs`) e quem sabe se
    deu certo; o handler nem conhece o caminho do arquivo.
  - **Nas duas faces porque isto conserta um bug**: hoje o nome tem resolução de SEGUNDO e o arquivo
    é aberto em `append` — duas perguntas no mesmo segundo (rate limit tem burst 2, IPs distintos
    não competem, e o MCP faz várias chamadas por conversa) viram **um arquivo com dois registros**,
    podendo até intercalar sob `O_APPEND`. Com a rota nova isso deixaria um glob por UUID devolver
    a pergunta e a PII de outra pessoa. O UUID no nome elimina a colisão por construção.
- **D-LOG-2 (DTO):** a resposta do chat ganha o campo **opcional** `log_id: Option<String>` —
  presente quando o log foi gravado com sucesso, ausente se a gravação falhou (o chat nunca falha
  por causa do log; o frontend simplesmente não mostra o ícone). Campo novo opcional ⇒ clientes
  antigos não quebram. MCP e /v1/retrieve ficam FORA do escopo (não ganham log_id).
  - *Pré-requisito descoberto em 09/08:* **hoje o chat FALHA por causa do log.** O `rag.rs` faz
    `log_question(…)?` e o handler converte qualquer `Err` em texto de resposta
    (`unwrap_or_else(|e| e.to_string())`) — com o disco cheio, o usuário recebe
    "Permission denied (os error 13)" como se fosse a resposta, depois de o LLM já ter rodado.
    D-LOG-2 descreve o estado desejado, não o atual: **a gravação passa a ser não-fatal** (erro
    vai ao `tracing`, a resposta segue sem `log_id`). Aprovado pelo Carlos em 09/08.
- **D-LOG-3 (rota):** `GET /v1/log/{uuid}`:
  - validação ESTRITA do formato UUID antes de tocar o filesystem (é ela que elimina path
    traversal por construção — nada de sanitizar caminho: input que não é UUID é 400);
  - lookup por glob `*_{uuid}.*` no diretório de logs; **zero** rotas de listagem;
  - `200 text/plain; charset=utf-8` com o arquivo como está;
  - `404` com corpo amigável ("log expirado ou inexistente") — a MESMA mensagem para
    nunca-existiu e para podado-pela-retenção, sem vazar qual dos dois;
  - rate limit e CORS iguais às rotas existentes da API.
- **D-LOG-4 (frontend):** no `SystemMessage.tsx`, o ícone entra AO LADO do ícone de cópia,
  mesmo tamanho, mesmo estilo de botão, tooltip "Ver o log desta resposta"; renderizado apenas
  quando `log_id` está presente (o que já o exclui da saudação e de respostas cujo log falhou).
  Clique abre modal Chakra: título "Log da pergunta", conteúdo em `<pre>` monoespaçado com
  scroll (o log já é texto estruturado por seções — v1 NÃO parseia nada), botão de copiar o log
  inteiro, estados de loading e de erro (a mensagem do 404 do backend).
- **D-LOG-5 (integridade na exibição):** o conteúdo é mostrado COMO ESTÁ — sem filtrar, editar ou
  resumir. Isso inclui o par pergunta original / versão anonimizada enviada ao LLM: mostrar o
  mascaramento funcionando é feature de privacidade visível, não vazamento (o leitor é o próprio
  autor da pergunta).
- **D-LOG-6 (retenção):** nenhuma mudança na política de retenção dos logs. Log podado ⇒ 404
  amigável do D-LOG-3.

## Fases

**L1 — Backend.** UUID no `log_question` + nome do arquivo (D-LOG-1), campo no DTO (D-LOG-2), rota
(D-LOG-3). Testes: formato inválido ⇒ 400 sem tocar o disco (incluindo tentativas de traversal
como `../`, que falham na validação de formato); UUID válido inexistente ⇒ 404; feliz ⇒ 200
text/plain byte a byte igual ao arquivo; resposta do chat carrega `log_id` e ele casa com um
arquivo recém-gravado; falha simulada de gravação ⇒ resposta sem `log_id` e sem erro ao usuário;
**duas gravações no mesmo milissegundo ⇒ dois arquivos distintos** (a colisão da D-LOG-1).

**L2 — Frontend.** Ícone + modal (D-LOG-4). Conferir: tamanho idêntico ao de cópia lado a lado;
ausente na saudação; loading e erro renderizam; o modal fecha e reabre sem re-fetch desnecessário
(cache simples por uuid na sessão é bem-vindo, não obrigatório).

**L3 — Portão humano (Carlos).** Uma pergunta real em cada uma das três fontes do chat rs:
abrir o log, conferir as seções (pergunta, versão anonimizada, contexto `## documento`, resposta,
ADERÊNCIA, distâncias), conferir que a URL do modal não é adivinhável a partir de outra, e que um
UUID inventado dá o 404 amigável. Aprovado ⇒ deploy (API + frontend).

## Critérios de aceite

- `cargo test` e `clippy -D warnings` verdes; testes de frontend existentes verdes.
- Nenhuma rota lista, conta ou enumera logs; `grep` confirma que o diretório de logs só é lido
  pela rota nova via glob de UUID validado.
- Cliente antigo (sem conhecer `log_id`) continua funcionando sem mudança.
- Registro no PR: uma captura do modal aberto e o resultado do L3 com data.
