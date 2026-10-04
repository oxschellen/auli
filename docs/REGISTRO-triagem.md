# Registro — Serviços + FAQs: triagem em duas chamadas, citação e lista de fontes (D-SF-1..10)

**Data:** 04/10/2026 · **Escopo:** a série D-SF-1..10 (PRs #168 e #169), a guarda contra resposta
vazia do LLM (#170) e os bumps de versão que fecham a entrega — frontend v0.1.66, servidor v0.5.0.
Só a entidade `rs`; os outros 26 estados seguem com o prompt v1 e uma chamada só.

**Onde está o resto:** as decisões D-SF-\* e o mecanismo ficam em [auli_code.md](auli_code.md)
§3.4.1; o que segue aberto, em [auli_pendencias.md](auli_pendencias.md) §40. Este documento registra
**o que entrou, como foi verificado e o que a fumaça contra o LLM real mostrou** — inclusive o
desvio de configuração que ela forçou (§3).

**Convenção deste registro:** distingo **medido** (saiu de uma chamada real, com o número ao lado),
**deduzido** (inferência com a evidência ao lado) e **não verificado** (fica como pendência).

---

## 1. O problema e a resposta, em linguagem simples

Nas consultas do tipo 1 (Serviços + FAQs), a resposta **unificava** serviços distintos e atribuía a
uma situação a regra de outra — o ICMS do regime geral e o do Simples Nacional saíam misturados. Os
serviços do portal são **alternativas** para situações excludentes; a busca densa traz justamente os
irmãos, e o chat, com 30 documentos e um prompt que pedia "a orientação completa", fundia tudo.

A série ataca isso em três frentes: **menos documentos e um prompt que proíbe a síntese** (D-SF-1..5),
**tirar da mesa, antes de redigir, o que não trata da situação** (a triagem, D-SF-9) e **tornar a
atribuição conferível** — cada afirmação cita a fonte, e a lista do que a busca trouxe aparece sempre
abaixo da resposta (D-SF-7, D-SF-10).

## 2. O que entrou

| Commit em `main` | Decisão | O que muda |
| --- | --- | --- |
| `28ac3ed` (#168) | D-SF-1..5 | `config/prompts/rs.txt` v2 (proíbe combinar documentos; irmãos viram alternativas com a condição que decide); `corpus`: 10 → 5 serviços e 20 → 10 FAQs; descrição da ferramenta MCP. Sem `auli update`: nada muda vetores |
| `142f53a` (#169) | D-SF-6 | `LLM_REASONING_EFFORT` no `.env` (ausente = não envia; inválido aborta o boot); saída do chat 4096 → 8192 tokens |
| | D-SF-8 | Marcador `{{CONTEXTO}}` no prompt: o que vem depois dele vai **depois** dos documentos (`rag::compor_system_prompt`); prompt sem marcador sai byte a byte igual |
| | D-SF-7 | `rs.txt` exige citação por afirmação: `[Serviço 2]`, `[FAQ 4]` (numeração por coleção) |
| | D-SF-9 | **Triagem**: a 1ª chamada devolve JSON com um veredito por documento (`aplica` / `condicional` / `descarta`); a 2ª redige sem os descartados. Opt-in por `prompt_triagem` no registry (só o `rs`, `config/prompts/rs-triagem.txt`); fail-open; teto de 20 s; frontend 35 s → 70 s; seção TRIAGEM no log da consulta |
| | D-SF-10 | Campo `fontes` (`rotulo`, `titulo`, `link`, `triagem`) no `POST /v1/question`; `Fontes.tsx` recolhido no chat ("Fontes consultadas: N (k descartadas na triagem)") |
| | — | `chore(frontend): v0.1.66` |
| `37740df` (#170) | — | Resposta vazia do LLM vira erro visível (§4) |
| `1a3f4b5` | — | `chore(server): v0.5.0` (`auli-cli` 0.4.0 → 0.5.0; aparece no boot e no `serverInfo` do MCP) |

**Como chegou ao `main`.** Os patches 0002–0006 vieram prontos no `TAREFA-D-SF.md` e foram aplicados
com `git am -3` sobre a branch do #168; a árvore resultante bateu com a esperada
(`23faab64…`). Como o #168 já tinha sido squash-mergeado, os seis commits novos foram rebaseados
sobre o `main` descartando o D-SF-1..5 original — **medido**: a árvore do commit original e a do
squash eram idênticas (`91fc5f8a…`), e a árvore depois do rebase era a mesma que passou nos testes.

## 3. Verificação

### 3.1 Estática e testes

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (que o ambiente de
  origem dos patches não tinha) e `cargo test --workspace`: limpos.
- Frontend: `npm ci`, typecheck, lint e build limpos; `npm test` com **252** testes (o documento da
  tarefa contava 246 — mesma árvore; a diferença não foi investigada).
- `check-registry-sync.sh` e `check-sem-navi.sh`: OK. CI do #169 e do #170: 5/5 checks verdes.

### 3.2 Fumaça contra o LLM real — o esforço `high` esvazia a resposta

Pergunta: *"Como a empresa recolhe o ICMS mensal e qual guia deve usar?"*, entidade `rs`, tipo 1,
modelo `openai/gpt-oss-120b` na Groq.

**Com `LLM_REASONING_EFFORT=high`** (o valor que a tarefa mandava pôr no `.env`):

| | resultado | tempo total |
| --- | --- | --- |
| servidor, 3 consultas | **2 respostas vazias** (`""`), 1 boa | 24,7–32,3 s |
| chamada direta da redação, 4× | **3× `finish_reason=length`** com 8190 reasoning tokens, conteúdo vazio; 1× `stop` (5556 reasoning) | 13,1–17,9 s |

A triagem funcionou (14,5 s, JSON válido, 10 de 15 descartados); o que falhava era a **2ª chamada**:
o modelo gastava os 8192 tokens raciocinando e não sobrava nada para o texto. O comentário da D-SF-6
dizia "risco previsto, não medido" — ficou medido.

**Com `medium`** — chamada direta, 4×: **4× `stop`**, 738–906 reasoning tokens, 3,1–3,7 s. Pelo
servidor, 4 consultas:

| consulta | resposta | triagem | descartados | total |
| --- | --- | --- | --- | --- |
| 1 | 1930 caracteres | 5,3 s | 9 de 15 | 9,3 s |
| 2 | 1731 | 4,6 s | 7 de 15 | 8,2 s |
| 3 | 1277 | 4,2 s | 9 de 15 | 7,1 s |
| 4 | 2183 | 4,3 s | 6 de 15 | 8,9 s |

As respostas citam `[Serviço 1]`, `[FAQ 2]` e `[FAQ 8]`, e os rótulos batem com os da lista
`fontes`. **Decisão:** o `.env` do servidor ficou com `LLM_REASONING_EFFORT=medium`.

**Não verificado na fumaça:**

- a separação **regime geral × Simples Nacional**, que é o caso que motivou a série: a pergunta não
  trouxe nenhum documento do Simples, então a resposta só contrapôs GA × GNRE. Precisa de uma
  pergunta que traga os dois regimes;
- a lista "Fontes consultadas" **no navegador** — conferida só pela API.

## 4. A guarda contra resposta vazia (#170)

O vazio passava calado: o `auli-llm` lia só `choices[0].message.content` e ignorava o
`finish_reason`. Agora:

- `auli_llm::ChatResponse` ganha `finish_reason` (`"stop"`, `"length"`…; `None` em erro de API);
- `auli-cli/src/llm.rs::texto_ou_aviso` troca conteúdo vazio por *"Erro na chamada da API do modelo
  AI: o modelo não produziu resposta (finish_reason=…). Tente novamente."* e emite um `warn!`. O
  texto chega ao usuário e fica na seção RESPOSTA do log da consulta; na triagem ele não é JSON, e
  ela falha aberta como antes.

**Por que no servidor e não no `auli-llm`:** os lotes offline (sinopse, extração) contam com o vazio
para cair na validação e re-tentar com `reasoning_effort=low`. Um texto de erro de API os faria
desistir sem o resgate (`resposta_e_erro_de_api` encerra a tentativa).

**Verificado:** 2 testes novos; com a guarda desligada (mutação), o teste do vazio falha. Contra o LLM
real **não foi exercitada** — com `medium` o vazio não ocorre.

## 5. Implantação (04/10/2026)

- `.env`: `LLM_REASONING_EFFORT=medium`. **Não** voltar para `high` sem subir o teto de 8192 e medir
  contra o orçamento de 70 s do frontend (≈ 18 s por 8k tokens de raciocínio, mais a triagem).
- Sem `auli update` nem rebuild de packs (D-SF-4).
- Servidor reiniciado com o binário v0.5.0, que contém o #170 — **medido** por `/proc/PID/exe` e
  pela presença das mensagens novas no binário.

## 6. Pendências que este registro acrescenta

1. O [.env.example](../.env.example) ainda sugere `# LLM_REASONING_EFFORT=high`, e a linha da D-SF-6
   em [auli_code.md](auli_code.md) §3.4.1 diz que o `gpt-oss` "confere melhor as próprias regras com
   esforço alto" — ambos devem passar a apontar `medium` e o resultado da §3.2.
2. Fumaça com uma pergunta que traga **os dois regimes** (geral e Simples), para ver as alternativas
   separadas com a condição que decide — o objetivo da série.
3. As medidas da triagem da [auli_pendencias.md](auli_pendencias.md) §40, item 5 (latência somada,
   taxa de `FALHOU`, descartes errados) seguem valendo — agora com `medium` como linha de base.
