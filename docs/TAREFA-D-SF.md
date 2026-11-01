# TAREFA — Respostas de Serviços + FAQs: aplicar a série D-SF-1..10

> Documento único para o Claude Code. Contém o contexto, o passo a passo e, no fim (Anexo), os seis
> patches na íntegra. Trabalhe **um passo de cada vez** e confirme comigo (Carlos) antes de qualquer
> push ou merge.

## 1. Contexto

O problema observado no uso real: nas consultas do tipo 1 (Serviços + FAQs), a indicação do serviço
funciona quando ele está bem descrito, mas a resposta **unifica** informações de serviços distintos e
atribui a uma situação a regra de outra — o ICMS do regime geral e o do Simples Nacional saíam
misturados. A causa é de desenho: os serviços são **alternativas** para situações excludentes (a
escolha sempre coube a quem usa o portal), a busca densa traz justamente os irmãos, e o chat — com 30
documentos e um prompt que pedia "a orientação completa" — fundia tudo.

A série ataca isso em seis commits, sobre o `cb9645d` (último commit do `main` no GitHub em
04/10/2026). As decisões completas ficam em `docs/auli_code.md` §3.4.1 e o que fica aberto em
`docs/auli_pendencias.md` §40 — ambos fazem parte dos patches.

| Patch | Decisão | O que muda |
| ----- | ------- | ---------- |
| 0001 | D-SF-1..5 | `rs.txt` v2 (proíbe síntese entre documentos); `corpus.rs`: 10→5 serviços e 20→10 FAQs; descrição da ferramenta MCP; docs |
| 0002 | D-SF-6 | `LLM_REASONING_EFFORT` no `.env` (ausente = não envia; valor inválido aborta o boot); saída do chat 4096→8192 tokens |
| 0003 | D-SF-8 | Marcador `{{CONTEXTO}}` no prompt: o que vem depois dele vai depois dos documentos (`rag::compor_system_prompt`); prompt sem marcador sai byte a byte igual |
| 0004 | D-SF-7 | `rs.txt` exige citação por afirmação no formato `[Serviço 2]` / `[FAQ 4]` |
| 0005 | D-SF-9 | Triagem em **duas chamadas** ao LLM (`triagem.rs`), opt-in pelo campo `prompt_triagem` do registry (só o `rs`); fail-open; teto de 20 s; frontend 35 s→70 s; seção TRIAGEM no log |
| 0006 | D-SF-10 | Lista fixa de fontes na resposta de Serviços + FAQs: campo `fontes` no `POST /v1/question` e `Fontes.tsx` (recolhida) no chat |

A numeração dos patches segue a ordem de aplicação; a das decisões segue a ordem em que foram
tomadas (por isso 0003 é D-SF-8 e 0004 é D-SF-7).

## 2. O que já foi verificado fora daqui

Num clone limpo, os seis patches aplicados sobre o `cb9645d` com `git am` produzem a árvore
`23faab64001f75b0c0ff4781bdb5f7f2930eecb2`. Nesse estado passaram: `cargo fmt --all -- --check`;
`cargo test -p auli-cli -p auli-core -p auli-contract -p auli-llm` (137 testes no `auli-cli`); no
frontend, `tsc --noEmit`, `npm run lint`, `npm run build` e `vitest run` (246 testes);
`scripts/tools/check-registry-sync.sh`, `scripts/tools/check-sem-navi.sh` e markdownlint.

**Não verificado:** `cargo clippy` (o ambiente não tinha) e qualquer chamada real ao LLM — a triagem
e o esforço de raciocínio nunca rodaram contra a Groq.

## 3. Passo a passo

### Passo 1 — Extrair os patches deste documento

Os patches estão no Anexo, cada um num bloco cercado por `~~~~~patch <nome>`. Não copie à mão:
extraia com o script abaixo (ajuste o caminho deste arquivo) e confira os hashes.

```bash
mkdir -p /tmp/dsf && python3 - <<'PY'
import re, pathlib, hashlib
doc = pathlib.Path("TAREFA-D-SF.md").read_text(encoding="utf-8")
for nome, corpo in re.findall(r"^~~~~~patch (\S+)\n(.*?)^~~~~~$", doc, re.S | re.M):
    p = pathlib.Path("/tmp/dsf") / nome
    p.write_text(corpo, encoding="utf-8")
    print(hashlib.sha256(corpo.encode("utf-8")).hexdigest(), nome)
PY
```

Os hashes têm de ser exatamente estes — se algum divergir, pare e me avise:

```text
687ffadc531be933de64394a71114d05b40028f4fcb08d11159fc6e824007455  0001-D-SF-1-a-5.patch
7adcebaa423edf5806ad4b4a1965cd6cd6e6de133dfa69b877d088658d3d2174  0002-D-SF-6.patch
34c313d547c31112deb57bb9b8d1c36292d2cea8bbf7a6441b9b4c9e330ff422  0003-D-SF-8.patch
f87dbed466efa4e07b4d17624148528eb673fccde6e2123fe70e63a343fa59b0  0004-D-SF-7.patch
c263762b4efd3a48eeac4095203f90a1c37951ab9fa7137e653cd3c8524503cd  0005-D-SF-9.patch
66bacfccd851fc3791fa7793d1711387f220650c494238da2974fe2e5b8a4a4a  0006-D-SF-10.patch
```

### Passo 2 — Ver onde o repositório local está

```bash
cd ~/Desktop/auli
git status --short          # tem de estar limpo; se não estiver, pare e me pergunte
git log --oneline -5
git log --oneline --grep='D-SF' | head
```

- Se o `git log --grep='D-SF'` já mostrar o commit **D-SF-1..5**, o 0001 já está aplicado: pule-o.
- O local pode estar **à frente** do `cb9645d` (trabalho de setembro que não foi ao GitHub). Por
  isso o `-3` no `git am` abaixo.

### Passo 3 — Aplicar numa branch

```bash
git switch -c servicos-faqs-alternativas
git am -3 /tmp/dsf/0001-D-SF-1-a-5.patch   # pular se já aplicado (Passo 2)
git am -3 /tmp/dsf/0002-D-SF-6.patch /tmp/dsf/0003-D-SF-8.patch /tmp/dsf/0004-D-SF-7.patch \
          /tmp/dsf/0005-D-SF-9.patch /tmp/dsf/0006-D-SF-10.patch
```

Se o `git am` parar em conflito: resolva **preservando as duas intenções** (a do commit local e a do
patch), `git add` nos arquivos e `git am --continue`. Os pontos mais prováveis de conflito, se o local
mudou depois de 22/08, são `rag.rs`, `docs/auli_code.md` e `config/registry.toml`. Na dúvida sobre a
intenção, pare e me pergunte — não use `git am --skip`.

Se o local estiver exatamente no `cb9645d` (+ 0001), confira a árvore final:

```bash
git rev-parse HEAD^{tree}   # esperado: 23faab64001f75b0c0ff4781bdb5f7f2930eecb2
```

### Passo 4 — Verificar

```bash
cd auli-server
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings   # o único que não rodou fora daqui
cargo test --workspace
cd ../auli-frontend
npm ci && npm run typecheck && npm run lint && npm test && npm run build
cd ..
bash scripts/tools/check-registry-sync.sh
bash scripts/tools/check-sem-navi.sh
```

Se o clippy apontar algo, corrija num commit separado (`fix(...)`), sem reescrever os commits da série.

### Passo 5 — Configurar e subir

1. No `.env` do servidor, acrescente `LLM_REASONING_EFFORT=high`. Sem essa linha o item D-SF-6 fica
   desligado (comportamento antigo). Valor inválido aborta o boot de propósito.
2. **Não** rode `auli update` nem rebuild de packs: nada aqui muda vetores (D-SF-4).
3. Recompile e reinicie o servidor; gere o build novo do frontend.

### Passo 6 — Teste de fumaça (com o LLM real)

Faça no chat, entidade `rs`, tipo Serviços + FAQs, uma pergunta em que serviços do regime geral e do
Simples Nacional concorram (ex.: algo sobre ICMS de empresa sem dizer o regime). Confira:

- a resposta cita `[Serviço N]` / `[FAQ N]` e apresenta as alternativas separadas, dizendo o que
  decide entre elas;
- abaixo da resposta aparece "Fontes consultadas: N (…)" — recolhida — e os rótulos batem com as
  citações;
- no log da consulta (ícone de histórico), existe a seção **TRIAGEM (primeira chamada)** com um JSON
  de vereditos, e o `tempo:` dela está bem abaixo de 20 s;
- o tempo total da resposta fica abaixo de 70 s.

Se a triagem aparecer como `FALHOU` em toda pergunta, pare e me mostre o log: o mais provável é o
modelo não estar devolvendo JSON puro, e o ajuste é no `config/prompts/rs-triagem.txt`.

### Passo 7 — Fechar

- Bump de versão do frontend (`0.1.65` → `0.1.66`) num commit `chore(frontend): v0.1.66` separado,
  como é a convenção do repositório.
- **Não** faça push nem merge sem a minha confirmação.

## 4. O que NÃO fazer

- Não replicar o prompt v2, a triagem nem o marcador para os outros 26 estados — isso é deliberado e
  espera a validação (`docs/auli_pendencias.md` §40, itens 1, 2 e 5).
- Não mexer nas bandas de proximidade (`SVC_BAND`/`FAQ_BAND`): seguem ∞/piso 0 até haver dado.
- Não mencionar o nome da unidade de atendimento em nenhum arquivo (o `check-sem-navi.sh` guarda
  isso).

---

## Anexo — os patches

Cada bloco abaixo é um arquivo `git format-patch` íntegro. Extraia com o script do Passo 1.

### 0001-D-SF-1-a-5.patch — D-SF-1..5 — prompt v2 e 5 + 10 documentos

~~~~~patch 0001-D-SF-1-a-5.patch
From b8a4beef47344ff886904a75cb766c559e6c34d5 Mon Sep 17 00:00:00 2001
From: ubuntu <carlos.schellenberger@gmail.com>
Date: Sun, 4 Oct 2026 11:03:24 +0000
Subject: [PATCH] =?UTF-8?q?feat(chat):=20servi=C3=A7os=20e=20FAQs=20s?=
 =?UTF-8?q?=C3=A3o=20alternativas,=20n=C3=A3o=20pe=C3=A7as=20=E2=80=94=20p?=
 =?UTF-8?q?rompt=20v2=20do=20rs=20e=205=20+=2010=20documentos=20(D-SF-1..5?=
 =?UTF-8?q?)?=
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

O sintoma veio do uso real: a indicação do serviço funciona quando ele está bem
descrito; o que falha é a resposta que UNIFICA informações de muitos serviços e
atribui a uma situação a regra de outra — o ICMS do regime geral e o do Simples
Nacional saíam misturados.

O mecanismo é uma premissa errada do desenho para esta coleção. O RAG trata o
contexto como peças a sintetizar; os serviços são uma árvore de casos
excludentes, em que a escolha sempre coube a quem usa o portal — e por isso a
condição que separa um serviço do irmão muitas vezes nem está escrita. A busca
densa agrava por construção: os irmãos ficam colados no espaço vetorial e
voltam juntos. Com 30 documentos e um prompt que pedia "a orientação completa",
fundir era o comportamento pedido.

Duas mudanças, em par:

- `config/prompts/rs.txt` v2 (D-SF-2): ancoragem pela fonte, não pelo assunto;
  proibido combinar documentos ou transferir requisito/prazo de um para outro;
  irmãos excludentes viram alternativas com a condição que decide; condição
  não escrita não se deduz; sem documento aplicável, "o Portal não traz essa
  informação". Só o rs nesta leva — replicar depois de medir.
- `corpus::SERVICES` 10 → 5 e `corpus::FAQS` 20 → 10 (D-SF-3). Menos irmãos e
  menos ruído, mas não resolve sozinho: o irmão costuma estar no topo. Bandas
  seguem ∞/piso 0 — é teto, não calibragem.

Sem bump de STRATEGY_VERSION nem de PACK_FORMAT, e sem `auli update` (D-SF-4):
`corpus` é parâmetro de consulta. Vale para as três faces de uma vez — chat,
MCP `consultar_servicos_faqs` (descrição atualizada) e o default de `top_k` do
`/v1/retrieve` (D-SF-5). O parity-replay vai acusar divergência contra logs
anteriores; é a linha de base nova, não regressão.

Decisões em docs/auli_code.md §3.4.1; o que fica aberto (validação v1 × v2,
replicação do prompt, condição de aplicação por serviço) em
docs/auli_pendencias.md §40.

Verificado: markdownlint limpo, check-sem-navi limpo. NÃO compilado aqui (sem
toolchain Rust no ambiente) — rodar cargo fmt --check e cargo test antes do merge.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
---
 auli-server/crates/auli-cli/src/mcp.rs     |  2 +-
 auli-server/crates/auli-core/src/corpus.rs | 12 +++++--
 config/prompts/rs.txt                      | 37 ++++++++++++----------
 docs/auli_code.md                          | 32 +++++++++++++++++--
 docs/auli_pendencias.md                    | 27 ++++++++++++++++
 5 files changed, 86 insertions(+), 24 deletions(-)

diff --git a/auli-server/crates/auli-cli/src/mcp.rs b/auli-server/crates/auli-cli/src/mcp.rs
index d1df9da..49a2638 100644
--- a/auli-server/crates/auli-cli/src/mcp.rs
+++ b/auli-server/crates/auli-cli/src/mcp.rs
@@ -343,7 +343,7 @@ impl AuliMcp {
     #[tool(
         description = "Consulta os serviços de atendimento e as perguntas frequentes (FAQs) \
         de uma UF. Devolve UM bloco de texto com os serviços e FAQs mais próximos da pergunta \
-        (até 10 serviços e até 20 FAQs) — o MESMO contexto que o chat do portal Auli usa para \
+        (até 5 serviços e até 10 FAQs) — o MESMO contexto que o chat do portal Auli usa para \
         responder. Ideal para dúvidas de 'como fazer' do contribuinte (guias, cadastros, \
         certidões, parcelamentos, prazos de atendimento); para fundamentação tributária \
         (interpretação da legislação), prefira `buscar_pareceres`."
diff --git a/auli-server/crates/auli-core/src/corpus.rs b/auli-server/crates/auli-core/src/corpus.rs
index f28103a..032efe4 100644
--- a/auli-server/crates/auli-core/src/corpus.rs
+++ b/auli-server/crates/auli-core/src/corpus.rs
@@ -16,13 +16,19 @@ pub struct Collection {
     pub n_results: usize, // how many documents to retrieve for RAG
 }
 
+// Serviços e FAQs (D-SF-3, 04/10/2026): 5 + 10, eram 10 + 20. Os documentos desta coleção são
+// ALTERNATIVAS entre si — cada um vale para uma situação específica (regime geral × Simples
+// Nacional, cidadão × empresa) —, e a busca densa traz justamente os irmãos. Com 30 no contexto a
+// resposta fundia regras de situações excludentes. Cortar o número reduz os irmãos e o ruído, mas
+// NÃO resolve sozinho (o irmão costuma estar no topo): o par é o prompt v2 (D-SF-2), que proíbe a
+// síntese entre documentos. As bandas seguem ∞/piso 0 — isto é teto, não calibragem.
 pub const SERVICES: Collection = Collection {
     kind: "servicos",
-    n_results: 10,
+    n_results: 5,
 };
 pub const FAQS: Collection = Collection {
     kind: "faqs",
-    n_results: 20,
+    n_results: 10,
 };
 // Pares P/R de legislação (D-LEG-10): mesma política dos serviços na v1 — coleção pequena e
 // autorada, em que cada documento já é uma resposta curta. Como nas demais, a banda é ∞/piso 0: a
@@ -75,7 +81,7 @@ mod tests {
     fn from_kind_resolves_known_and_rejects_unknown() {
         // `servicos` is the one vocabulary — route param, vector-collection suffix, and UI label.
         assert_eq!(from_kind("servicos").unwrap().kind, "servicos");
-        assert_eq!(from_kind("faqs").unwrap().n_results, 20);
+        assert_eq!(from_kind("faqs").unwrap().n_results, 10);
         // The old English spelling `services` is no longer a kind.
         assert!(from_kind("services").is_err());
     }
diff --git a/config/prompts/rs.txt b/config/prompts/rs.txt
index 3bcf2d8..deba8b2 100644
--- a/config/prompts/rs.txt
+++ b/config/prompts/rs.txt
@@ -1,19 +1,22 @@
 
 '''
-### Instructions
-### Voce é um modelo de linguagem treinado para auxiliar o Analista de Tributação a indicar o serviço adequado no Portal de Atendimento .
-### Sua tarefa é responder à questão proposta indicando os serviços adequados disponíveis no Portal de Atendimento
-### Indique também os links das Perguntas Frequentes que se apliquem ao assunto.
-### Responda sempre no idioma português do brasil.
-### Não responda perguntas fora do assunto relativo a tributos
-### Faça uma resposta com no máximo 400 palavras,
-### Estruture a resposta em duas partes, nesta ordem:
-### 1. Comece com a seção "**Resumo**": no máximo 3 frases, indo direto ao serviço ou procedimento adequado, com o link principal. Sem preâmbulo.
-### 2. Em seguida, a seção "**Detalhes**": a orientação completa, com os demais serviços, FAQs e links aplicáveis.
-### Se a resposta inteira couber no Resumo, omita a seção Detalhes.
-### Para responder use as informações apresentadas na lista de serviços e nas perguntas frequentes (Faq) apresentados abaixo.
-### Cada documento do contexto inicia com o marcador: ## documento (seguido do número e do tipo — Serviço ou FAQ).
-### Dentro de cada documento, o título vem após o marcador ## pergunta e o conteúdo após o marcador ## resposta.
-### Sempre apresente os links de chamadas https
-### Apresente cada link no formato markdown [Título](URL), usando como texto o título exato do serviço ou da pergunta frequente — nunca exiba a URL crua no texto da resposta.
-### Se a pergunta não puder ser respondida com as informações disponíveis, responda que não é possível responder
+### Instruções
+### Você auxilia o Analista de Tributação a identificar, no Portal de Atendimento, o serviço ou a pergunta frequente que se aplica à situação do contribuinte.
+### Responda sempre em português do Brasil, em no máximo 400 palavras. Perguntas fora do tema tributário: diga que não pode ajudar.
+###
+### Natureza dos documentos
+### Cada documento do contexto começa com o marcador ## documento, seguido do número e do tipo (Serviço ou FAQ). O título vem após ## pergunta e o conteúdo após ## resposta.
+### Cada serviço e cada FAQ vale para uma situação específica (por exemplo: regime geral ou Simples Nacional; cidadão ou empresa; um tipo de operação). Documentos parecidos costumam ser ALTERNATIVAS entre si, não partes de uma mesma resposta.
+###
+### Regras de fidelidade
+### 1. Use apenas o que está escrito nos documentos. Não complete com conhecimento próprio sobre legislação, prazos, valores, alíquotas ou documentos exigidos.
+### 2. Nunca combine informações de documentos diferentes numa mesma orientação. Um requisito, prazo ou procedimento de um documento nunca deve ser atribuído a outro.
+### 3. Se mais de um documento puder se aplicar e eles se distinguem por uma condição do contribuinte (regime, público, tipo de operação), apresente-os separadamente, cada um com a condição que o distingue, e diga qual informação do contribuinte decide entre eles. Não escolha pelo analista.
+### 4. Se a condição de aplicação de um documento não estiver escrita nele, não a deduza: diga que a aplicabilidade deve ser conferida no link.
+### 5. Ignore os documentos que não tratam da situação perguntada. Não é preciso citar todos.
+### 6. Se nenhum documento tratar da situação perguntada, responda que o Portal não traz essa informação. Não oriente com base em conhecimento próprio.
+###
+### Formato
+### Seção "**Resumo**": no máximo 3 frases, com o serviço ou FAQ aplicável e o link. Se houver alternativas pela regra 3, diga quais são e o que decide entre elas. Sem preâmbulo.
+### Seção "**Detalhes**": um bloco por documento citado, encabeçado pelo título em link, contendo apenas o que aquele documento diz. Omita a seção se o Resumo bastar.
+### Apresente cada link no formato markdown [Título](URL), usando o título exato do documento — nunca a URL crua.
diff --git a/docs/auli_code.md b/docs/auli_code.md
index 45902c0..73d599b 100644
--- a/docs/auli_code.md
+++ b/docs/auli_code.md
@@ -184,7 +184,8 @@ Acionado por `POST /v1/question`. Assinatura:
 4. **O `query_type` decide o caminho** — é o seletor de fonte da caixa de mensagem, que chega como
    inteiro no campo `type`:
    - **`ServicosFaqs`** (código 1, o default): consulta **duas** coleções de forma **concorrente**
-     (`tokio::try_join!`), `<id>-servicos` (`n_results = 10`) e `<id>-faqs` (`n_results = 20`).
+     (`tokio::try_join!`), `<id>-servicos` (`n_results = 5`) e `<id>-faqs` (`n_results = 10`) —
+     eram 10 + 20 até 04/10/2026; ver §3.4.1.
    - **`Jurisprudencia(Kind)`** (código 2 = `pareceres`, 3 = `tarf`): uma coleção só,
      `n_results = 10`. Nos **pareceres** há ainda a expansão por grafo — pareceres que citam os
      mesmos dispositivos —, exclusiva deles (o `dispositivos-index.json` é derivado da árvore de
@@ -212,6 +213,30 @@ Acionado por `POST /v1/question`. Assinatura:
 prompt de sistema. Só `notas` fica de fora: tem rota de listagem, mas não tem fonte struct nem
 pack — ver §3.13.
 
+### 3.4.1 Serviços + FAQs: alternativas, não peças — as cinco decisões (D-SF-\*)
+
+**O sintoma (04/10/2026, uso real):** a indicação do serviço funciona quando o serviço está bem
+descrito; o que falha é a resposta que **unifica** informações de muitos serviços distintos e
+atribui a uma situação a regra de outra — o ICMS de uma empresa no regime geral tem regras
+diferentes das do Simples Nacional, e o texto saía misturando as duas. Não é invenção do nada: o
+modelo compõe peças verdadeiras de documentos que não se aplicam juntos.
+
+**O mecanismo:** o RAG trata o contexto como peças de um quebra-cabeça a sintetizar, e os serviços
+são o oposto — uma árvore de casos pontuais e excludentes, em que a **escolha** sempre coube a quem
+usa o portal, e por isso a condição que separa um serviço do irmão muitas vezes nem está escrita na
+descrição. A busca densa agrava por construção: "ICMS regime geral X" e "ICMS Simples Nacional X"
+ficam colados no espaço vetorial, então o que volta junto são justamente os irmãos. Com 30
+documentos no contexto e um prompt que pedia "a orientação completa", a fusão era o comportamento
+pedido.
+
+| Decisão    | O quê | Por quê |
+| ---------- | ----- | ------- |
+| **D-SF-1** | Na coleção serviços+FAQs, documentos parecidos são tratados como **alternativas**: o chat não sintetiza entre eles, e a ambiguidade entre situações vira apresentação das alternativas com a condição que decide | É a premissa que estava errada. A seleção que o portal deixa com o usuário volta para quem atende, em vez de ser resolvida pelo modelo sem o dado que a decide. |
+| **D-SF-2** | `config/prompts/rs.txt` **v2**: ancoragem pela **fonte**, não pelo assunto; proibição de combinar documentos e de transferir requisito/prazo de um para outro; condição de aplicação não escrita não se deduz; documentos irrelevantes são ignorados; sem documento aplicável, "o Portal não traz essa informação"; **Detalhes** passa a ser um bloco por documento citado | O v1 delimitava o escopo por assunto ("não responda fora de tributos") — pergunta tributária sem resposta no contexto era "dentro do escopo" e o modelo completava com conhecimento próprio. E a seção Detalhes pedia "a orientação completa", que é literalmente a instrução de fundir. O frontend não depende dos títulos Resumo/Detalhes (conferido), então o formato é livre. **Só o `rs`** nesta leva: o `sc.txt` é idêntico ao v1 e os demais diferem em ~9 linhas; replicar depois de medir (§40 das pendências). Prompt é catálogo: reiniciar o servidor basta. |
+| **D-SF-3** | `corpus::SERVICES.n_results` 10 → **5**; `corpus::FAQS.n_results` 20 → **10** | Menos irmãos e menos ruído no contexto, e cerca de metade dos tokens do prompt. **Não resolve sozinho** — o irmão excludente costuma estar entre os primeiros —, por isso vem em par com a D-SF-2. As bandas continuam ∞/piso 0: isto é teto, não calibragem; a calibragem segue adiada e com base acumulando no log. |
+| **D-SF-4** | Sem bump de `STRATEGY_VERSION` nem de `PACK_FORMAT`; nenhum `auli update` | `corpus` é parâmetro de **consulta** (o próprio módulo diz que nada ali afeta o que foi embedado). Recompilar e reiniciar basta; os packs ficam como estão. |
+| **D-SF-5** | A mudança vale para as **três faces** de uma vez: chat (tipo 1), ferramenta MCP `consultar_servicos_faqs` (descrição atualizada para "até 5 serviços e até 10 FAQs") e o default de `top_k` do `/v1/retrieve` para `servicos`/`faqs` | É a paridade por construção da §3.12 — as faces leem as mesmas constantes. Consequência esperada: o `parity-replay.py` acusa divergência contra logs anteriores a esta mudança, porque o contexto ficou menor. Não é regressão; a linha de base nova começa aqui. |
+
 ### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)
 
 Toda a parte de embeddings/busca vetorial é **in-process** (sem Ollama nem ChromaDB):
@@ -835,8 +860,9 @@ mantém um espelho **gerado** (não mais divergente) do registro. Pendências em
   na raiz do repo) e `GET /v1/{kind}/list` (leitura). Escuta configurável (`--bind`, default `0.0.0.0`).
   Público, **sem auth nem banco**; CORS; configuração por `.env` (`config()`); logging via `tracing`.
   Vetorização separada pelo `auli update`.
-- **Doze estados ativos** (rs/sc/sp/pr/mg/pe/ba/rj/ce/ms/mt/go). RAG consulta efetivamente apenas `servicos` (10) +
-  `faqs` (20); estreitamento por proximidade presente mas em modo paridade (`band=∞`) até calibração.
+- **Doze estados ativos** (rs/sc/sp/pr/mg/pe/ba/rj/ce/ms/mt/go). RAG consulta efetivamente apenas `servicos` (5) +
+  `faqs` (10) — eram 10 + 20 até 04/10/2026, ver §3.4.1; estreitamento por proximidade presente mas
+  em modo paridade (`band=∞`) até calibração.
 - `auli-frontend`: SPA com seleção de entidade (as 12), chat contra `POST /v1/question` com
   timeout de 25s, abas de referência lendo `public/<id>/` (arquivos prefixados `<id>-`), tema
   claro/escuro, testes Vitest.
diff --git a/docs/auli_pendencias.md b/docs/auli_pendencias.md
index 029f497..381f1ee 100644
--- a/docs/auli_pendencias.md
+++ b/docs/auli_pendencias.md
@@ -1557,6 +1557,33 @@ público raspado. Nada disso é o mantenedor — e uma varredura cega por `carlo
 
 ---
 
+## 40. Serviços + FAQs: a resposta fundia serviços excludentes — prompt v2 e 5 + 10 (aberta — 2026-10-04)
+
+Decisões e mecanismo em [docs/auli_code.md](docs/auli_code.md) §3.4.1 (D-SF-1..5). Em resumo: os
+serviços são alternativas para situações excludentes (regime geral × Simples Nacional, cidadão ×
+empresa), a busca densa traz justamente os irmãos, e o chat — com 30 documentos e um prompt que
+pedia "a orientação completa" — fundia regras que não se aplicam juntas. Entraram o `rs.txt` v2,
+que proíbe a síntese entre documentos, e o corte para 5 serviços + 10 FAQs.
+
+**O que fica aberto:**
+
+1. **Validação v1 × v2.** Reenviar ao servidor local as perguntas reais em que a mistura foi vista
+   e comparar as respostas. Critério: nenhuma afirmação de um documento atribuída a outro; quando
+   houver irmãos excludentes, as alternativas aparecem separadas com a condição que decide. O
+   `parity-replay.py` reenvia a partir dos logs, mas descarta a resposta — precisa de uma variante
+   que a guarde. Atenção: contra logs anteriores a 04/10 ele acusa divergência de contexto por
+   construção (D-SF-5).
+2. **Replicar o prompt** para os outros 26 estados, **depois** da validação. O `sc.txt` é idêntico
+   ao `rs.txt` v1; os demais diferem em ~9 linhas e precisam de merge à mão.
+3. **Se o par prompt + corte não bastar:** extrair offline uma **condição de aplicação** por
+   serviço, no molde do `extracao-servicos.txt` (só o que estiver explícito no texto). Os serviços
+   em que ela sair vazia são exatamente os mal descritos — e essa lista tem valor por si, para
+   quem cuida do conteúdo do portal.
+4. **Não reabrir sem dado:** as bandas continuam ∞/piso 0. O corte 5 + 10 é teto; a calibragem
+   segue a doutrina do `ESTUDO-busca-hibrida.md` §9.
+
+---
+
 ## D-NAMING (pendência separada — MG, NÃO é do GO)
 
 Política da frota: separador sigla–UF sempre `-`. Normalizar o `orgao` do **MG** `"SEF/MG"` →
-- 
2.43.0

~~~~~

### 0002-D-SF-6.patch — D-SF-6 — esforço de raciocínio e 8192 tokens

~~~~~patch 0002-D-SF-6.patch
From 1fe9aa39d89ef87aea5c6a49fd88cc949a827520 Mon Sep 17 00:00:00 2001
From: ubuntu <carlos.schellenberger@gmail.com>
Date: Sun, 4 Oct 2026 11:33:04 +0000
Subject: [PATCH 2/6] =?UTF-8?q?feat(llm):=20o=20esfor=C3=A7o=20de=20racioc?=
 =?UTF-8?q?=C3=ADnio=20do=20chat=20vem=20do=20.env,=20e=20a=20sa=C3=ADda?=
 =?UTF-8?q?=20sobe=20para=208192=20(D-SF-6)?=
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

O `gpt-oss` é modelo de raciocínio, e o chat nunca enviava o `reasoning_effort`:
valia o default do provedor. O `auli-llm` já suportava o campo — faltava só o
chat passá-lo.

`LLM_REASONING_EFFORT` = low | medium | high. Ausente, o campo não é enviado,
porque modelos que não são de raciocínio o recusam: trocar de modelo não pode
exigir recompilar. Valor desconhecido ABORTA o boot, ao contrário do
`parse_opt` — erro de digitação aqui não pode virar default em silêncio.

`max_completion_tokens` do chat 4096 → 8192: nesses modelos a saída inclui o
raciocínio, e com esforço alto a cota antiga arriscaria resposta vazia. Risco
previsto, não medido.

Documentado em .env.example, auli_operations.md §4.4 e auli_code.md §3.4.1.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
---
 .env.example                              |  3 ++
 auli-server/crates/auli-cli/src/config.rs | 49 +++++++++++++++++++++++
 auli-server/crates/auli-cli/src/llm.rs    | 11 +++--
 docs/auli_code.md                         |  3 +-
 docs/auli_operations.md                   |  1 +
 5 files changed, 62 insertions(+), 5 deletions(-)

diff --git a/.env.example b/.env.example
index 3d74fbb..9e0b174 100644
--- a/.env.example
+++ b/.env.example
@@ -5,6 +5,9 @@
 LLM_API_URL=https://api.groq.com/openai/v1/chat/completions
 LLM_API_KEY=your-llm-api-key
 LLM_API_MODEL=openai/gpt-oss-120b
+# Esforço de raciocínio do chat: low | medium | high. Ausente = o campo não é enviado e vale o
+# default do provedor (use assim com modelos que não são de raciocínio). Valor inválido aborta o boot.
+# LLM_REASONING_EFFORT=high
 
 # Local embeddings (fastembed / BGE-M3, in-process — no external service)
 # EMBED_CACHE_DIR — NÃO defina aqui: os lançadores (start_server.sh, build-packs.sh)
diff --git a/auli-server/crates/auli-cli/src/config.rs b/auli-server/crates/auli-cli/src/config.rs
index c692415..6fa28c3 100644
--- a/auli-server/crates/auli-cli/src/config.rs
+++ b/auli-server/crates/auli-cli/src/config.rs
@@ -14,6 +14,9 @@ pub struct Config {
     pub llm_api_url: String,
     pub llm_api_key: String,
     pub llm_api_model: String,
+    // Esforço de raciocínio do chat (D-SF-6). `None` = o campo não é enviado e vale o default do
+    // provedor — é o que mantém o chat funcionando com um modelo que não aceita o parâmetro.
+    pub llm_reasoning_effort: Option<auli_llm::ReasoningEffort>,
 
     // Local embeddings (fastembed / BGE-M3, in-process)
     pub embed_cache_dir: String,
@@ -38,6 +41,7 @@ impl Config {
             llm_api_url: req("LLM_API_URL"),
             llm_api_key: req("LLM_API_KEY"),
             llm_api_model: req("LLM_API_MODEL"),
+            llm_reasoning_effort: reasoning_effort("LLM_REASONING_EFFORT"),
 
             embed_cache_dir: opt("EMBED_CACHE_DIR", "./models"),
             embed_threads: parse_opt("EMBED_THREADS", 16),
@@ -51,6 +55,11 @@ impl Config {
         println!("🔧 Configuração:");
         println!("LLM_API_URL: {}", self.llm_api_url);
         println!("LLM_API_MODEL: {}", self.llm_api_model);
+        println!(
+            "LLM_REASONING_EFFORT: {}",
+            self.llm_reasoning_effort
+                .map_or("(default do provedor)", |e| e.as_str())
+        );
         println!("EMBED_CACHE_DIR: {}", self.embed_cache_dir);
         println!("EMBED_THREADS: {}", self.embed_threads);
         println!("AULI_ANONIMIZAR_LLM: {}", self.anonimizar_llm);
@@ -85,3 +94,43 @@ fn opt_bool(key: &str, default: bool) -> bool {
         _ => default,
     }
 }
+
+// Esforço de raciocínio (D-SF-6). Ausente ou vazio ⇒ `None` (não envia o campo). Valor presente e
+// desconhecido ⇒ PÂNICO no boot, ao contrário do `parse_opt`: um erro de digitação aqui não pode
+// virar "default do provedor" em silêncio, porque o efeito (resposta menos aderente) só apareceria
+// muito depois, e sem causa visível.
+fn reasoning_effort(key: &str) -> Option<auli_llm::ReasoningEffort> {
+    let v = std::env::var(key).ok()?;
+    parse_reasoning_effort(&v).unwrap_or_else(|| {
+        panic!("{key} inválido: '{v}' — use low, medium ou high (ou deixe ausente)")
+    })
+}
+
+/// `Some(None)` = vazio (o mesmo que ausente); `None` = valor desconhecido.
+fn parse_reasoning_effort(v: &str) -> Option<Option<auli_llm::ReasoningEffort>> {
+    use auli_llm::ReasoningEffort::*;
+    match v.trim().to_ascii_lowercase().as_str() {
+        "" => Some(None),
+        "low" => Some(Some(Low)),
+        "medium" => Some(Some(Medium)),
+        "high" => Some(Some(High)),
+        _ => None,
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use auli_llm::ReasoningEffort::*;
+
+    #[test]
+    fn reasoning_effort_aceita_os_tres_niveis_e_vazio_e_recusa_o_resto() {
+        assert_eq!(parse_reasoning_effort("high"), Some(Some(High)));
+        assert_eq!(parse_reasoning_effort(" Medium "), Some(Some(Medium)));
+        assert_eq!(parse_reasoning_effort("LOW"), Some(Some(Low)));
+        assert_eq!(parse_reasoning_effort(""), Some(None));
+        // O erro de digitação é recusado, não promovido a default.
+        assert_eq!(parse_reasoning_effort("hihg"), None);
+        assert_eq!(parse_reasoning_effort("max"), None);
+    }
+}
diff --git a/auli-server/crates/auli-cli/src/llm.rs b/auli-server/crates/auli-cli/src/llm.rs
index 58fcadd..a8ec728 100644
--- a/auli-server/crates/auli-cli/src/llm.rs
+++ b/auli-server/crates/auli-cli/src/llm.rs
@@ -8,17 +8,20 @@ use crate::error::Result;
 /// Chama o LLM do chat do RAG. Os parâmetros são decisão do servidor:
 /// - `temperature 0.1`, sem `top_p`: RAG tributário exige fidelidade ao contexto recuperado, não
 ///   diversidade — sampling controlado só pela temperature (top_p omitido = 1.0).
-/// - `timeout 30 s`: mantido abaixo do budget de 35 s do frontend (callServerAPI.ts) — ver o
-///   comentário completo do invariante em `auli-llm`.
+/// - `timeout 30 s` por chamada: ver o comentário completo do invariante em `auli-llm`.
+/// - `max_completion_tokens 8192` (era 4096) e `reasoning_effort` do `.env` (D-SF-6): nos modelos de
+///   raciocínio, os tokens de saída incluem o raciocínio. Com esforço alto, 4096 arriscaria o
+///   modelo gastar a cota pensando e devolver conteúdo vazio — o mesmo defeito que o `Low` do lote
+///   de sinopses existe para quebrar. Risco previsto, não medido.
 pub async fn chat(system_prompt: &str, user_message: &str) -> Result<String> {
     let params = auli_llm::LlmParams {
         api_url: config().llm_api_url.clone(),
         api_key: config().llm_api_key.clone(),
         model: config().llm_api_model.clone(),
         temperature: 0.1,
-        max_completion_tokens: 4096,
+        max_completion_tokens: 8192,
         timeout: Duration::from_secs(30),
-        reasoning_effort: None,
+        reasoning_effort: config().llm_reasoning_effort,
     };
     // O chat do RAG não usa o headroom de rate-limit (isso é do lote de sinopses offline).
     Ok(auli_llm::chat(&params, system_prompt, user_message)
diff --git a/docs/auli_code.md b/docs/auli_code.md
index 73d599b..56a7b57 100644
--- a/docs/auli_code.md
+++ b/docs/auli_code.md
@@ -213,7 +213,7 @@ Acionado por `POST /v1/question`. Assinatura:
 prompt de sistema. Só `notas` fica de fora: tem rota de listagem, mas não tem fonte struct nem
 pack — ver §3.13.
 
-### 3.4.1 Serviços + FAQs: alternativas, não peças — as cinco decisões (D-SF-\*)
+### 3.4.1 Serviços + FAQs: alternativas, não peças — as decisões (D-SF-\*)
 
 **O sintoma (04/10/2026, uso real):** a indicação do serviço funciona quando o serviço está bem
 descrito; o que falha é a resposta que **unifica** informações de muitos serviços distintos e
@@ -236,6 +236,7 @@ pedido.
 | **D-SF-3** | `corpus::SERVICES.n_results` 10 → **5**; `corpus::FAQS.n_results` 20 → **10** | Menos irmãos e menos ruído no contexto, e cerca de metade dos tokens do prompt. **Não resolve sozinho** — o irmão excludente costuma estar entre os primeiros —, por isso vem em par com a D-SF-2. As bandas continuam ∞/piso 0: isto é teto, não calibragem; a calibragem segue adiada e com base acumulando no log. |
 | **D-SF-4** | Sem bump de `STRATEGY_VERSION` nem de `PACK_FORMAT`; nenhum `auli update` | `corpus` é parâmetro de **consulta** (o próprio módulo diz que nada ali afeta o que foi embedado). Recompilar e reiniciar basta; os packs ficam como estão. |
 | **D-SF-5** | A mudança vale para as **três faces** de uma vez: chat (tipo 1), ferramenta MCP `consultar_servicos_faqs` (descrição atualizada para "até 5 serviços e até 10 FAQs") e o default de `top_k` do `/v1/retrieve` para `servicos`/`faqs` | É a paridade por construção da §3.12 — as faces leem as mesmas constantes. Consequência esperada: o `parity-replay.py` acusa divergência contra logs anteriores a esta mudança, porque o contexto ficou menor. Não é regressão; a linha de base nova começa aqui. |
+| **D-SF-6** | Esforço de raciocínio do chat vem do `.env` (`LLM_REASONING_EFFORT` = `low`/`medium`/`high`); ausente = o campo não é enviado. `max_completion_tokens` do chat 4096 → **8192** | O `gpt-oss` confere melhor as próprias regras com esforço alto, e o `auli-llm` já suportava o campo — faltava o chat passá-lo. Variável, e não constante, porque o parâmetro é recusado por modelos que não são de raciocínio: trocar de modelo não pode exigir recompilar. Valor inválido **aborta o boot** (o erro de digitação não vira default em silêncio). Os 8192 existem porque, nesses modelos, a saída inclui o raciocínio. Vale para todos os tipos de consulta do chat. |
 
 ### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)
 
diff --git a/docs/auli_operations.md b/docs/auli_operations.md
index 441c784..cca6328 100644
--- a/docs/auli_operations.md
+++ b/docs/auli_operations.md
@@ -334,6 +334,7 @@ para o server rodando em `auli-server/`. Variáveis:
 | Variável                                        | Obrigatória?                                                 | Uso                                             |
 | ----------------------------------------------- | ------------------------------------------------------------ | ----------------------------------------------- |
 | `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` | sim                                                          | LLM externo (Groq-compat) que redige a resposta |
+| `LLM_REASONING_EFFORT`                          | não (ausente = default do provedor)                          | `low`/`medium`/`high`; inválido aborta o boot   |
 | `EMBED_CACHE_DIR`                               | não (lançadores: `<raiz>/models`; def. do código `./models`) | cache do modelo                                 |
 | `EMBED_THREADS`                                 | não (def. `16`)                                              | threads do ONNX Runtime                         |
 | `AULI_LOG_DIR`                                  | não (lançadores: `<raiz>/logs`; def. do código `./logs`)     | dir dos logs de Q&A do RAG (§7)                 |
-- 
2.43.0

~~~~~

### 0003-D-SF-8.patch — D-SF-8 — marcador {{CONTEXTO}}

~~~~~patch 0003-D-SF-8.patch
From 11e7ea4a874a2322f97adb20df507f3f0d6e1e7d Mon Sep 17 00:00:00 2001
From: ubuntu <carlos.schellenberger@gmail.com>
Date: Sun, 4 Oct 2026 11:34:57 +0000
Subject: [PATCH 3/6] =?UTF-8?q?feat(rag):=20o=20marcador=20{{CONTEXTO}}=20?=
 =?UTF-8?q?p=C3=B5e=20a=20regra=20central=20DEPOIS=20dos=20documentos=20(D?=
 =?UTF-8?q?-SF-8)?=
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Com 15 documentos entre as regras e a pergunta, a regra que mais importa ficava
longe do ponto em que o modelo redige. O prompt de sistema passa a aceitar o
marcador `{{CONTEXTO}}`: o que vem antes dele abre, o contexto e o `'''` vêm no
meio, e o que vem depois fecha.

`rag::compor_system_prompt` é pura e testada. Sem o marcador ela devolve o
`format!("{base}{rag}'''")` de sempre, byte a byte — é o caso dos outros 26
estados, e um teste trava isso. Divide na primeira ocorrência; o marcador nunca
chega ao modelo.

O QUE repetir é decisão do prompt, não do código: o `rs.txt` repete a regra
central (alternativas, não peças; nada de completar com conhecimento próprio).

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
---
 auli-server/crates/auli-cli/src/rag.rs | 46 ++++++++++++++++++++++++--
 config/prompts/rs.txt                  |  2 ++
 docs/auli_code.md                      |  4 ++-
 3 files changed, 49 insertions(+), 3 deletions(-)

diff --git a/auli-server/crates/auli-cli/src/rag.rs b/auli-server/crates/auli-cli/src/rag.rs
index 481ba74..1447fb4 100644
--- a/auli-server/crates/auli-cli/src/rag.rs
+++ b/auli-server/crates/auli-cli/src/rag.rs
@@ -197,6 +197,25 @@ fn envolver(rotulo: &str, i: usize, conteudo: &str) -> String {
     format!("\n## documento {i}: {rotulo}\n{conteudo}\n")
 }
 
+/// O marcador que divide um prompt de sistema em "antes do contexto" e "depois do contexto".
+pub(crate) const MARCADOR_CONTEXTO: &str = "{{CONTEXTO}}";
+
+/// **Monta o prompt de sistema**: prompt da entidade + contexto RAG + o delimitador `'''`.
+///
+/// Sem o [`MARCADOR_CONTEXTO`], é exatamente o `format!("{base}{rag}'''")` de sempre — byte a byte,
+/// e é o caso de todos os prompts escritos antes da D-SF-8. Com ele, o prompt se divide na PRIMEIRA
+/// ocorrência: a parte de antes abre, o contexto e o `'''` vêm no meio, e a parte de depois fecha.
+///
+/// Existe porque, com o contexto inteiro entre as regras e a pergunta, a regra central fica longe
+/// do ponto em que o modelo redige. O que vai depois do marcador é tipicamente um lembrete curto da
+/// regra que mais importa — a decisão de O QUE repetir é do prompt, não do código.
+pub(crate) fn compor_system_prompt(base: &str, rag: &str) -> String {
+    match base.split_once(MARCADOR_CONTEXTO) {
+        Some((antes, depois)) => format!("{antes}{rag}'''{depois}"),
+        None => format!("{base}{rag}'''"),
+    }
+}
+
 /// O rótulo do documento que veio por **expansão de grafo**, não por busca vetorial.
 ///
 /// Fica aqui, e não no `Kind`, porque não é uma coleção: é um PAPEL que um parecer assume neste
@@ -508,11 +527,12 @@ pub async fn exec_all_question(
     tempos.retrieve_ms = t.elapsed().as_millis() as u64;
 
     // System prompt = base prompt (per query type) + RAG context, closed with the original delimiter.
+    // Com o marcador `{{CONTEXTO}}` no prompt, o que vem depois dele vai DEPOIS do contexto (D-SF-8).
     let base_prompt = match query_type {
         QueryType::ServicosFaqs => &cfg.system_prompt,
         QueryType::ColecaoUnica(kind) => cfg.prompt_de(kind),
     };
-    let system_prompt = format!("{}{}'''", base_prompt, rag);
+    let system_prompt = compor_system_prompt(base_prompt, &rag);
     trace!("System instructions with RAG: {}", system_prompt);
 
     // Fronteira do LLM: com o flag ligado (default), envia a pergunta ANONIMIZADA e restaura a
@@ -775,7 +795,8 @@ pub(crate) fn log_question(reg: &RegistroConsulta) -> std::io::Result<Uuid> {
 #[cfg(test)]
 mod tests {
     use super::{
-        FAQ_BAND, FAQ_FLOOR, LEG_BAND, LEG_FLOOR, PAR_BAND, PAR_FLOOR, QueryType, RegistroConsulta,
+        FAQ_BAND, FAQ_FLOOR, LEG_BAND, LEG_FLOOR, MARCADOR_CONTEXTO, PAR_BAND, PAR_FLOOR, QueryType,
+        RegistroConsulta, compor_system_prompt,
         SVC_BAND, SVC_FLOOR, TemposConsulta, Uuid, aderencia, bandas, bloco_documento, envolver,
         format_log_record, log_question, mes_do_log, montar_aderencia, montar_rag_colecao_unica,
         montar_rag_servicos_faqs,
@@ -805,6 +826,27 @@ mod tests {
         .unwrap()
     }
 
+    #[test]
+    fn compor_system_prompt_sem_marcador_e_o_formato_de_sempre() {
+        // A trava da D-SF-8: os 26 prompts sem marcador não podem mudar um byte.
+        let base = "\n'''\n### Regras\n";
+        let rag = "\n## documento 1: Serviço\nX\n";
+        assert_eq!(
+            compor_system_prompt(base, rag),
+            format!("{base}{rag}'''"),
+        );
+    }
+
+    #[test]
+    fn compor_system_prompt_com_marcador_poe_o_resto_depois_do_contexto() {
+        let base = format!("ANTES\n{MARCADOR_CONTEXTO}\nLEMBRETE");
+        let rag = "\n## documento 1: Serviço\nX\n";
+        let p = compor_system_prompt(&base, rag);
+        assert_eq!(p, format!("ANTES\n{rag}'''\nLEMBRETE"));
+        // O marcador em si nunca chega ao modelo.
+        assert!(!p.contains(MARCADOR_CONTEXTO));
+    }
+
     #[test]
     fn bloco_documento_le_corpo_da_arvore_e_monta_o_bloco() {
         // A raiz agora é `<docs_root>/<entity_id>/<doc_path>` (o motor resolve o `<id>`), então a
diff --git a/config/prompts/rs.txt b/config/prompts/rs.txt
index deba8b2..47ce4f8 100644
--- a/config/prompts/rs.txt
+++ b/config/prompts/rs.txt
@@ -20,3 +20,5 @@
 ### Seção "**Resumo**": no máximo 3 frases, com o serviço ou FAQ aplicável e o link. Se houver alternativas pela regra 3, diga quais são e o que decide entre elas. Sem preâmbulo.
 ### Seção "**Detalhes**": um bloco por documento citado, encabeçado pelo título em link, contendo apenas o que aquele documento diz. Omita a seção se o Resumo bastar.
 ### Apresente cada link no formato markdown [Título](URL), usando o título exato do documento — nunca a URL crua.
+{{CONTEXTO}}
+### Lembrete antes de responder: cada documento acima vale para uma situação específica, e documentos parecidos são alternativas, não partes de uma mesma resposta. Nunca combine informações de documentos diferentes nem complete com conhecimento próprio. Se nenhum documento tratar da situação perguntada, diga que o Portal não traz essa informação.
diff --git a/docs/auli_code.md b/docs/auli_code.md
index 56a7b57..d5ded9e 100644
--- a/docs/auli_code.md
+++ b/docs/auli_code.md
@@ -202,7 +202,8 @@ Acionado por `POST /v1/question`. Assinatura:
    jurisprudência; um aviso, em serviços e FAQs) e **nunca** derruba a query.
 7. Envolve cada bloco em `## documento {i}: {rótulo}` e concatena; o system prompt = prompt da
    entidade **para aquele tipo** (`system_prompt`, `pareceres_prompt` ou `tarf_prompt`) + contexto +
-   delimitador `'''`.
+   delimitador `'''` — ou, com o marcador `{{CONTEXTO}}` no prompt, com o contexto no lugar dele
+   (§3.4.1, D-SF-8).
 8. Chama o LLM externo — com a pergunta **anonimizada** se o flag estiver ligado (default), e
    restaura os placeholders na resposta antes de devolvê-la.
 9. Retorna `{ question, answer }` e **anexa o diálogo a `$AULI_LOG_DIR/<timestamp>.txt`** (default
@@ -237,6 +238,7 @@ pedido.
 | **D-SF-4** | Sem bump de `STRATEGY_VERSION` nem de `PACK_FORMAT`; nenhum `auli update` | `corpus` é parâmetro de **consulta** (o próprio módulo diz que nada ali afeta o que foi embedado). Recompilar e reiniciar basta; os packs ficam como estão. |
 | **D-SF-5** | A mudança vale para as **três faces** de uma vez: chat (tipo 1), ferramenta MCP `consultar_servicos_faqs` (descrição atualizada para "até 5 serviços e até 10 FAQs") e o default de `top_k` do `/v1/retrieve` para `servicos`/`faqs` | É a paridade por construção da §3.12 — as faces leem as mesmas constantes. Consequência esperada: o `parity-replay.py` acusa divergência contra logs anteriores a esta mudança, porque o contexto ficou menor. Não é regressão; a linha de base nova começa aqui. |
 | **D-SF-6** | Esforço de raciocínio do chat vem do `.env` (`LLM_REASONING_EFFORT` = `low`/`medium`/`high`); ausente = o campo não é enviado. `max_completion_tokens` do chat 4096 → **8192** | O `gpt-oss` confere melhor as próprias regras com esforço alto, e o `auli-llm` já suportava o campo — faltava o chat passá-lo. Variável, e não constante, porque o parâmetro é recusado por modelos que não são de raciocínio: trocar de modelo não pode exigir recompilar. Valor inválido **aborta o boot** (o erro de digitação não vira default em silêncio). Os 8192 existem porque, nesses modelos, a saída inclui o raciocínio. Vale para todos os tipos de consulta do chat. |
+| **D-SF-8** | Marcador `{{CONTEXTO}}` no prompt de sistema: o que vem antes dele fica antes dos documentos, o que vem depois vai **depois** do contexto e do `'''` (`rag::compor_system_prompt`). O `rs.txt` usa o espaço para repetir a regra central | Com 15 documentos entre as regras e a pergunta, a regra que mais importa ficava longe do ponto em que o modelo redige. A decisão de O QUE repetir é do prompt (catálogo), não do código. Prompt **sem** marcador sai byte a byte como antes — é o caso dos outros 26 estados, e há teste que trava isso. Divide na primeira ocorrência; o marcador nunca chega ao modelo. |
 
 ### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)
 
-- 
2.43.0

~~~~~

### 0004-D-SF-7.patch — D-SF-7 — citação por afirmação

~~~~~patch 0004-D-SF-7.patch
From afbe0fa221eededdebc0274f88d3aad7ba2602c6 Mon Sep 17 00:00:00 2001
From: ubuntu <carlos.schellenberger@gmail.com>
Date: Sun, 4 Oct 2026 11:35:12 +0000
Subject: [PATCH 4/6] =?UTF-8?q?feat(prompt):=20o=20rs.txt=20exige=20cita?=
 =?UTF-8?q?=C3=A7=C3=A3o=20por=20afirma=C3=A7=C3=A3o=20=E2=80=94=20[Servi?=
 =?UTF-8?q?=C3=A7o=202],=20[FAQ=204]=20(D-SF-7)?=
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Cada afirmação da resposta passa a terminar com o documento de onde veio, no
formato do marcador do contexto. Isso amarra a frase à fonte — fundir dois
serviços numa orientação só fica visível — e torna a conferência mecânica.

O tipo vai junto do número porque a numeração do contexto é POR COLEÇÃO
(D-MARC-3): "[doc 3]" seria ambíguo entre o Serviço 3 e a FAQ 3. A seção
Detalhes passa a abrir cada bloco com a citação e o título em link.

Só prompt: nenhum código muda.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
---
 config/prompts/rs.txt | 3 ++-
 docs/auli_code.md     | 1 +
 2 files changed, 3 insertions(+), 1 deletion(-)

diff --git a/config/prompts/rs.txt b/config/prompts/rs.txt
index 47ce4f8..5f56def 100644
--- a/config/prompts/rs.txt
+++ b/config/prompts/rs.txt
@@ -15,10 +15,11 @@
 ### 4. Se a condição de aplicação de um documento não estiver escrita nele, não a deduza: diga que a aplicabilidade deve ser conferida no link.
 ### 5. Ignore os documentos que não tratam da situação perguntada. Não é preciso citar todos.
 ### 6. Se nenhum documento tratar da situação perguntada, responda que o Portal não traz essa informação. Não oriente com base em conhecimento próprio.
+### 7. Termine cada afirmação citando, entre colchetes, o documento de onde ela veio, com o tipo e o número exatos do marcador — por exemplo [Serviço 2] ou [FAQ 4]. A numeração é separada por tipo: Serviço 1 e FAQ 1 são documentos diferentes. Afirmação sem documento para citar não deve ser escrita.
 ###
 ### Formato
 ### Seção "**Resumo**": no máximo 3 frases, com o serviço ou FAQ aplicável e o link. Se houver alternativas pela regra 3, diga quais são e o que decide entre elas. Sem preâmbulo.
-### Seção "**Detalhes**": um bloco por documento citado, encabeçado pelo título em link, contendo apenas o que aquele documento diz. Omita a seção se o Resumo bastar.
+### Seção "**Detalhes**": um bloco por documento citado, encabeçado pela citação e pelo título em link (por exemplo: [Serviço 2] [Título](URL)), contendo apenas o que aquele documento diz. Omita a seção se o Resumo bastar.
 ### Apresente cada link no formato markdown [Título](URL), usando o título exato do documento — nunca a URL crua.
 {{CONTEXTO}}
 ### Lembrete antes de responder: cada documento acima vale para uma situação específica, e documentos parecidos são alternativas, não partes de uma mesma resposta. Nunca combine informações de documentos diferentes nem complete com conhecimento próprio. Se nenhum documento tratar da situação perguntada, diga que o Portal não traz essa informação.
diff --git a/docs/auli_code.md b/docs/auli_code.md
index d5ded9e..bde1dd3 100644
--- a/docs/auli_code.md
+++ b/docs/auli_code.md
@@ -238,6 +238,7 @@ pedido.
 | **D-SF-4** | Sem bump de `STRATEGY_VERSION` nem de `PACK_FORMAT`; nenhum `auli update` | `corpus` é parâmetro de **consulta** (o próprio módulo diz que nada ali afeta o que foi embedado). Recompilar e reiniciar basta; os packs ficam como estão. |
 | **D-SF-5** | A mudança vale para as **três faces** de uma vez: chat (tipo 1), ferramenta MCP `consultar_servicos_faqs` (descrição atualizada para "até 5 serviços e até 10 FAQs") e o default de `top_k` do `/v1/retrieve` para `servicos`/`faqs` | É a paridade por construção da §3.12 — as faces leem as mesmas constantes. Consequência esperada: o `parity-replay.py` acusa divergência contra logs anteriores a esta mudança, porque o contexto ficou menor. Não é regressão; a linha de base nova começa aqui. |
 | **D-SF-6** | Esforço de raciocínio do chat vem do `.env` (`LLM_REASONING_EFFORT` = `low`/`medium`/`high`); ausente = o campo não é enviado. `max_completion_tokens` do chat 4096 → **8192** | O `gpt-oss` confere melhor as próprias regras com esforço alto, e o `auli-llm` já suportava o campo — faltava o chat passá-lo. Variável, e não constante, porque o parâmetro é recusado por modelos que não são de raciocínio: trocar de modelo não pode exigir recompilar. Valor inválido **aborta o boot** (o erro de digitação não vira default em silêncio). Os 8192 existem porque, nesses modelos, a saída inclui o raciocínio. Vale para todos os tipos de consulta do chat. |
+| **D-SF-7** | O `rs.txt` exige **citação por afirmação** no formato do marcador do contexto — `[Serviço 2]`, `[FAQ 4]` —, e afirmação sem documento para citar não se escreve | Amarra cada frase à fonte, o que dificulta a fusão entre documentos e torna a conferência mecânica (o rótulo citado existe no contexto? a frase está nele?). O tipo vai junto do número porque a numeração é **por coleção** (D-MARC-3): "[doc 3]" seria ambíguo entre o Serviço 3 e a FAQ 3. A lista de fontes da resposta usa os mesmos rótulos, para o analista bater uma com a outra. |
 | **D-SF-8** | Marcador `{{CONTEXTO}}` no prompt de sistema: o que vem antes dele fica antes dos documentos, o que vem depois vai **depois** do contexto e do `'''` (`rag::compor_system_prompt`). O `rs.txt` usa o espaço para repetir a regra central | Com 15 documentos entre as regras e a pergunta, a regra que mais importa ficava longe do ponto em que o modelo redige. A decisão de O QUE repetir é do prompt (catálogo), não do código. Prompt **sem** marcador sai byte a byte como antes — é o caso dos outros 26 estados, e há teste que trava isso. Divide na primeira ocorrência; o marcador nunca chega ao modelo. |
 
 ### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)
-- 
2.43.0

~~~~~

### 0005-D-SF-9.patch — D-SF-9 — triagem em duas chamadas

~~~~~patch 0005-D-SF-9.patch
From 5b141289ea07dd9f354a820a8280f7f977636e56 Mon Sep 17 00:00:00 2001
From: ubuntu <carlos.schellenberger@gmail.com>
Date: Sun, 4 Oct 2026 12:29:13 +0000
Subject: [PATCH 5/6] =?UTF-8?q?feat(chat):=20a=20triagem=20em=20duas=20cha?=
 =?UTF-8?q?madas=20tira=20da=20mesa=20o=20que=20n=C3=A3o=20trata=20da=20si?=
 =?UTF-8?q?tua=C3=A7=C3=A3o=20(D-SF-9)?=
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

A resposta de serviços+FAQs passa a sair de duas chamadas ao LLM. A primeira
recebe a pergunta e os documentos recuperados e devolve só JSON, um veredito
por rótulo: `aplica`, `condicional` ou `descarta`. A segunda redige vendo o
contexto sem os descartados. É a forma explícita da D-SF-1: o que não trata da
situação perguntada nem deveria estar na mesa quando o texto é escrito.

Decisões que o código carrega:

- `condicional` FICA: é o irmão que a resposta deve apresentar como
  alternativa. Na dúvida, o prompt manda escolher `condicional`.
- A numeração original é preservada (`montar_rag_servicos_faqs_sem`), para
  que citação, ADERÊNCIA do log e lista de fontes falem dos mesmos rótulos.
  Com nada descartado, o contexto sai byte a byte o de sempre — há teste.
- Tudo descartado ⇒ resposta fixa, sem segunda chamada: contexto vazio é
  convite a alucinação.
- Fail-open: rede, teto de tempo, JSON inválido ou rótulo inexistente ⇒ segue
  com o contexto integral, que é a resposta de uma chamada só. Documento
  omitido no JSON fica. A triagem é filtro de qualidade, não guarda.
- Teto de 20 s para o PASSO inteiro, com `tokio::time::timeout` por fora: o
  `auli-llm` repete até três vezes em timeout, e sem o teto externo a
  triagem poderia consumir 60 s antes de a resposta começar.
- Frontend: 35 s → 70 s (20 s de triagem + 30 s de redação + embed e busca).
  As menções antigas de "25 s" no README, auli_code e auli_features — já
  defasadas — passam a dizer 70 s.

Opt-in por entidade: campo `prompt_triagem` no registry, sem default global.
Só o rs tem (`config/prompts/rs-triagem.txt`); os outros 26 estados seguem com
uma chamada só. O MCP não tem triagem — entrega o contexto integral.

O log ganha a seção TRIAGEM (tempo, descartados, saída crua com o `motivo` de
cada veredito). O CONTEXTO RAG continua sendo o recuperado inteiro, o que a
triagem viu — preserva o sentido do parity-replay.

Decisão em docs/auli_code.md §3.4.1; o que medir (latência somada, taxa de
falha, descartes errados) em docs/auli_pendencias.md §40, item 5.

Verificado: cargo fmt --check, 134 testes do auli-cli (6 novos da triagem, 4
do rag), check-registry-sync, check-sem-navi, markdownlint, 49 testes do chat
no frontend.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
---
 README.md                                     |   2 +-
 .../src/pages/chat/utils/callServerAPI.ts     |   6 +-
 auli-server/crates/auli-cli/src/entities.rs   |  34 +++
 auli-server/crates/auli-cli/src/lib.rs        |   1 +
 auli-server/crates/auli-cli/src/llm.rs        |  25 +-
 auli-server/crates/auli-cli/src/mcp.rs        |   1 +
 auli-server/crates/auli-cli/src/rag.rs        | 227 ++++++++++++++++--
 auli-server/crates/auli-cli/src/triagem.rs    | 153 ++++++++++++
 auli-server/crates/auli-llm/src/lib.rs        |   2 +-
 config/prompts/rs-triagem.txt                 |  14 ++
 config/registry.toml                          |   5 +
 docs/auli_code.md                             |   5 +-
 docs/auli_features.md                         |   2 +-
 docs/auli_pendencias.md                       |   8 +-
 14 files changed, 450 insertions(+), 35 deletions(-)
 create mode 100644 auli-server/crates/auli-cli/src/triagem.rs
 create mode 100644 config/prompts/rs-triagem.txt

diff --git a/README.md b/README.md
index 4d98768..ce33f67 100644
--- a/README.md
+++ b/README.md
@@ -183,7 +183,7 @@ and only ever embeds the incoming question.
 A single-page app (no router; **tab navigation**) built with React 19, Vite, and Chakra UI v3.
 
 - **State selection** with an interactive map of Brazil; choice persisted in `localStorage`.
-- **Chat** against `POST /v1/question` (25 s timeout, friendly errors, copy button, markdown), with
+- **Chat** against `POST /v1/question` (70 s timeout, friendly errors, copy button, markdown), with
   a **query-type selector**: serviços+FAQs, pareceres or TARF acórdãos.
 - **Reference tabs** — Serviços, FAQs, Pareceres, Notas, Conteúdos — each reading static files from
   `public/<id>/`; "coming soon" placeholders for collections a state doesn't have yet.
diff --git a/auli-frontend/src/pages/chat/utils/callServerAPI.ts b/auli-frontend/src/pages/chat/utils/callServerAPI.ts
index 30072a1..eda475f 100644
--- a/auli-frontend/src/pages/chat/utils/callServerAPI.ts
+++ b/auli-frontend/src/pages/chat/utils/callServerAPI.ts
@@ -24,8 +24,10 @@ interface QuestionResponse {
   log_id?: string;
 }
 
-/** Abort the request if the server hasn't responded within this window. */
-const REQUEST_TIMEOUT_MS = 35_000;
+/** Abort the request if the server hasn't responded within this window.
+ *  70 s (era 35 s) desde a triagem de serviços+FAQs (D-SF-9): a resposta pode levar DUAS chamadas
+ *  ao LLM — até 20 s na triagem e 30 s na redação —, mais embed e busca. */
+const REQUEST_TIMEOUT_MS = 70_000;
 
 /** User-facing copy (pt-BR). Kept here so the wording lives in one place. */
 const MESSAGES = {
diff --git a/auli-server/crates/auli-cli/src/entities.rs b/auli-server/crates/auli-cli/src/entities.rs
index 759874d..2bf43f4 100644
--- a/auli-server/crates/auli-cli/src/entities.rs
+++ b/auli-server/crates/auli-cli/src/entities.rs
@@ -94,6 +94,11 @@ struct RegistryEntity {
     // (global, não o de pareceres da entidade — ver o doc da const).
     #[serde(default)]
     prompt_legislacao: String,
+    // Optional per-entity prompt for the TRIAGEM step of serviços+faqs (D-SF-9). Vazio ⇒ sem
+    // triagem: a resposta sai de uma chamada só, como antes. Não há default global de propósito —
+    // a triagem é opt-in por entidade.
+    #[serde(default)]
+    prompt_triagem: String,
 }
 
 #[derive(Debug, Clone)]
@@ -108,6 +113,8 @@ pub struct EntityConfig {
     // Prompt used for legislação queries (type=4); the entity's `prompt_legislacao` or the global
     // legislação default.
     pub legislacao_prompt: String,
+    // Prompt da triagem de serviços+faqs (D-SF-9); `None` = resposta em uma chamada só.
+    pub triagem_prompt: Option<String>,
 }
 
 impl EntityConfig {
@@ -203,6 +210,30 @@ fn load_prompt(
     })
 }
 
+/// Como [`load_prompt`], mas para um passo OPCIONAL, sem texto padrão: caminho vazio ⇒ `None` (o
+/// passo não existe para a entidade); arquivo ilegível ⇒ aviso e `None` — o passo cai fora, e a
+/// consulta segue pelo caminho de sempre em vez de rodar com um prompt que ninguém escreveu.
+fn load_prompt_opcional(
+    base: &std::path::Path,
+    id: &str,
+    rel_path: &str,
+    kind: &str,
+) -> Option<String> {
+    if rel_path.is_empty() {
+        return None;
+    }
+    match fs::read_to_string(base.join(rel_path)) {
+        Ok(p) => Some(p),
+        Err(_) => {
+            eprintln!(
+                "⚠️  prompt de {} ausente para a entidade '{}' ({}), passo desligado.",
+                kind, id, rel_path
+            );
+            None
+        }
+    }
+}
+
 fn load_entities() -> HashMap<String, EntityConfig> {
     let mut map = HashMap::new();
     // Os caminhos de prompt do registry (`prompts/rs.txt`) são relativos à pasta DO REGISTRY, e
@@ -236,6 +267,7 @@ fn load_entities() -> HashMap<String, EntityConfig> {
             "legislacao",
             DEFAULT_LEGISLACAO_PROMPT,
         );
+        let triagem_prompt = load_prompt_opcional(&base, &ent.id, &ent.prompt_triagem, "triagem");
         map.insert(
             ent.id.clone(),
             EntityConfig {
@@ -245,6 +277,7 @@ fn load_entities() -> HashMap<String, EntityConfig> {
                 pareceres_prompt,
                 tarf_prompt,
                 legislacao_prompt,
+                triagem_prompt,
             },
         );
     }
@@ -292,6 +325,7 @@ mod tests {
             pareceres_prompt: "PARECERES".into(),
             tarf_prompt: "TARF".into(),
             legislacao_prompt: legislacao_prompt.into(),
+            triagem_prompt: None,
         }
     }
 
diff --git a/auli-server/crates/auli-cli/src/lib.rs b/auli-server/crates/auli-cli/src/lib.rs
index 9a90ea7..2254cb8 100644
--- a/auli-server/crates/auli-cli/src/lib.rs
+++ b/auli-server/crates/auli-cli/src/lib.rs
@@ -22,6 +22,7 @@ pub mod packs;
 pub mod rag;
 pub mod remocoes;
 pub mod state;
+mod triagem;
 pub mod update;
 mod util;
 
diff --git a/auli-server/crates/auli-cli/src/llm.rs b/auli-server/crates/auli-cli/src/llm.rs
index a8ec728..d00137c 100644
--- a/auli-server/crates/auli-cli/src/llm.rs
+++ b/auli-server/crates/auli-cli/src/llm.rs
@@ -8,19 +8,40 @@ use crate::error::Result;
 /// Chama o LLM do chat do RAG. Os parâmetros são decisão do servidor:
 /// - `temperature 0.1`, sem `top_p`: RAG tributário exige fidelidade ao contexto recuperado, não
 ///   diversidade — sampling controlado só pela temperature (top_p omitido = 1.0).
-/// - `timeout 30 s` por chamada: ver o comentário completo do invariante em `auli-llm`.
+/// - `timeout 30 s` (20 s na triagem): ver o comentário completo do invariante em `auli-llm`.
 /// - `max_completion_tokens 8192` (era 4096) e `reasoning_effort` do `.env` (D-SF-6): nos modelos de
 ///   raciocínio, os tokens de saída incluem o raciocínio. Com esforço alto, 4096 arriscaria o
 ///   modelo gastar a cota pensando e devolver conteúdo vazio — o mesmo defeito que o `Low` do lote
 ///   de sinopses existe para quebrar. Risco previsto, não medido.
 pub async fn chat(system_prompt: &str, user_message: &str) -> Result<String> {
+    chamar(system_prompt, user_message, Duration::from_secs(30)).await
+}
+
+/// A chamada da TRIAGEM de serviços+faqs (D-SF-9): mesmos parâmetros do [`chat`], teto menor.
+///
+/// O orçamento é o das duas chamadas somadas: 20 s aqui + 30 s na resposta ficam abaixo dos 70 s
+/// do frontend (`callServerAPI.ts`). A triagem devolve um JSON curto, e se estourar o tempo a
+/// consulta segue sem ela (fail-open — ver `triagem.rs`), então o teto mais apertado é dela.
+///
+/// Os 20 s são o teto do passo INTEIRO, não só de cada tentativa: o `auli-llm` repete até três
+/// vezes em timeout, e sem o `tokio::time::timeout` por fora a triagem poderia consumir 60 s antes
+/// de a resposta começar. Na triagem, desistir cedo é o certo — o contexto integral está à mão.
+pub async fn triagem(system_prompt: &str, user_message: &str) -> Result<String> {
+    let teto = Duration::from_secs(20);
+    match tokio::time::timeout(teto, chamar(system_prompt, user_message, teto)).await {
+        Ok(r) => r,
+        Err(_) => Err(format!("excedeu o teto de {} s", teto.as_secs()).into()),
+    }
+}
+
+async fn chamar(system_prompt: &str, user_message: &str, timeout: Duration) -> Result<String> {
     let params = auli_llm::LlmParams {
         api_url: config().llm_api_url.clone(),
         api_key: config().llm_api_key.clone(),
         model: config().llm_api_model.clone(),
         temperature: 0.1,
         max_completion_tokens: 8192,
-        timeout: Duration::from_secs(30),
+        timeout,
         reasoning_effort: config().llm_reasoning_effort,
     };
     // O chat do RAG não usa o headroom de rate-limit (isso é do lote de sinopses offline).
diff --git a/auli-server/crates/auli-cli/src/mcp.rs b/auli-server/crates/auli-cli/src/mcp.rs
index 49a2638..7b67fee 100644
--- a/auli-server/crates/auli-cli/src/mcp.rs
+++ b/auli-server/crates/auli-cli/src/mcp.rs
@@ -193,6 +193,7 @@ impl AuliMcp {
             sanitizada: &anonimizada,
             answer: None, // sem LLM neste caminho
             aderencia,
+            triagem: None, // a triagem é só do chat
             rag: devolvido,
             tempos,
         }) {
diff --git a/auli-server/crates/auli-cli/src/rag.rs b/auli-server/crates/auli-cli/src/rag.rs
index 1447fb4..d3d0887 100644
--- a/auli-server/crates/auli-cli/src/rag.rs
+++ b/auli-server/crates/auli-cli/src/rag.rs
@@ -20,6 +20,7 @@ use crate::config::config;
 use crate::entities::get_entity;
 use crate::error::Result;
 use crate::llm;
+use crate::triagem;
 use crate::util::run_blocking;
 
 // Per-kind adaptive selection. `score` is a cosine DISTANCE — lower is closer. `floor` is the
@@ -234,6 +235,45 @@ pub(crate) fn montar_rag_servicos_faqs(svc_docs: &[String], faq_docs: &[String])
     format!("{}\n{}", rag_service, rag_faq)
 }
 
+/// Os rótulos dos documentos de serviços+faqs, na grafia em que o prompt manda citá-los e em que a
+/// triagem os devolve: `"Serviço 1".."Serviço n"`, depois `"FAQ 1".."FAQ m"` — numeração por
+/// coleção, como a do bloco (D-MARC-3).
+pub(crate) fn rotulos_servicos_faqs(n_svc: usize, n_faq: usize) -> Vec<String> {
+    let svc = (1..=n_svc).map(|i| format!("{} {i}", Kind::Servicos.rotulo()));
+    let faq = (1..=n_faq).map(|i| format!("{} {i}", Kind::Faqs.rotulo()));
+    svc.chain(faq).collect()
+}
+
+/// [`montar_rag_servicos_faqs`] sem os documentos de `fora` (rótulos de [`rotulos_servicos_faqs`]),
+/// **mantendo a numeração original**: o que sobra continua sendo o "Serviço 4" que a triagem
+/// avaliou, e a ADERÊNCIA do log continua batendo com ele. Com `fora` vazio, sai byte a byte igual
+/// ao [`montar_rag_servicos_faqs`] — há teste para isso.
+pub(crate) fn montar_rag_servicos_faqs_sem(
+    svc_docs: &[String],
+    faq_docs: &[String],
+    fora: &std::collections::BTreeSet<String>,
+) -> String {
+    let manter = |rotulo: &str, docs: &[String]| -> String {
+        docs.iter()
+            .enumerate()
+            .map(|(i, d)| (i + 1, d))
+            .filter(|(i, _)| !fora.contains(&format!("{rotulo} {i}")))
+            .map(|(i, d)| envolver(rotulo, i, d))
+            .collect()
+    };
+    format!(
+        "{}\n{}",
+        manter(Kind::Servicos.rotulo(), svc_docs),
+        manter(Kind::Faqs.rotulo(), faq_docs)
+    )
+}
+
+/// A resposta quando a triagem descarta TODOS os documentos (D-SF-9). Fixa, sem segunda chamada.
+pub(crate) const SEM_DOCUMENTO_APLICAVEL: &str = "Nenhum dos serviços e perguntas frequentes \
+recuperados do Portal trata da situação perguntada, então não há orientação a dar com base neles. \
+Reformule a pergunta com mais detalhes da situação do contribuinte, ou consulte o Portal \
+diretamente.";
+
 /// Contexto do tipo `ColecaoUnica`: um bloco numerado por documento recuperado, e — quando a
 /// expansão por grafo devolve algo — os pareceres que citam os mesmos dispositivos, rotulados como
 /// relacionados. Com `relacionados` vazio (default/sem grafo, SEMPRE no TARF — D-A4 — e SEMPRE na
@@ -397,6 +437,9 @@ pub async fn exec_all_question(
     // `aderencia` acompanha `rag` em paralelo: mesmos documentos, mesma ordem — mas só o `rag` vai
     // ao prompt do LLM.
     let mut itens_aderencia: Vec<Aderencia> = Vec::new();
+    // Os blocos de serviços e de FAQs em separado, guardados para a TRIAGEM (D-SF-9): ela precisa
+    // remontar o contexto sem os descartados, mantendo a numeração original. `None` nos outros tipos.
+    let mut blocos_sf: Option<(Vec<String>, Vec<String>)> = None;
     let rag = match query_type {
         QueryType::ServicosFaqs => {
             // Retrieve this entity's servicos + faqs concurrently, both through the engine.
@@ -429,10 +472,11 @@ pub async fn exec_all_question(
             itens_aderencia.extend(aderencia("faq", faq_hits.iter().map(|h| Some(h.score))));
             // Pack v2: o payload é leve, então o bloco de CADA serviço e de CADA faq é montado
             // aqui, lendo a árvore. Antes eles vinham prontos de dentro do pack.
-            montar_rag_servicos_faqs(
-                &blocos(&svc_hits, engine.docs_root(), &cfg.id),
-                &blocos(&faq_hits, engine.docs_root(), &cfg.id),
-            )
+            let svc_docs = blocos(&svc_hits, engine.docs_root(), &cfg.id);
+            let faq_docs = blocos(&faq_hits, engine.docs_root(), &cfg.id);
+            let rag = montar_rag_servicos_faqs(&svc_docs, &faq_docs);
+            blocos_sf = Some((svc_docs, faq_docs));
+            rag
         }
         QueryType::ColecaoUnica(kind) => {
             let colecao = corpus::from_kind(kind.as_str())
@@ -526,26 +570,90 @@ pub async fn exec_all_question(
     };
     tempos.retrieve_ms = t.elapsed().as_millis() as u64;
 
-    // System prompt = base prompt (per query type) + RAG context, closed with the original delimiter.
-    // Com o marcador `{{CONTEXTO}}` no prompt, o que vem depois dele vai DEPOIS do contexto (D-SF-8).
-    let base_prompt = match query_type {
-        QueryType::ServicosFaqs => &cfg.system_prompt,
-        QueryType::ColecaoUnica(kind) => cfg.prompt_de(kind),
-    };
-    let system_prompt = compor_system_prompt(base_prompt, &rag);
-    trace!("System instructions with RAG: {}", system_prompt);
-
     // Fronteira do LLM: com o flag ligado (default), envia a pergunta ANONIMIZADA e restaura a
     // resposta antes de devolvê-la ao usuário; com o flag desligado, envia a original (comportamento
-    // anterior). Os documentos do RAG são conteúdo público e NÃO passam por anonimização.
+    // anterior). Os documentos do RAG são conteúdo público e NÃO passam por anonimização. Vale para as
+    // DUAS chamadas quando há triagem.
     let anonimizar = config().anonimizar_llm;
     let entrada_llm = if anonimizar {
         &pergunta_anon
     } else {
         &question
     };
+
+    // `llm_ms` cobre as duas chamadas quando há triagem; o tempo só da triagem vai na seção dela.
     let t = Instant::now();
-    let resposta_llm = llm::chat(&system_prompt, entrada_llm).await?;
+
+    // TRIAGEM (D-SF-9) — só serviços+faqs, só com `prompt_triagem` na entidade. Primeira chamada:
+    // um veredito por documento. A segunda vê o contexto sem os descartados. Fail-open: triagem
+    // que falha segue com o contexto integral, que é a resposta de uma chamada só.
+    let mut rag_resposta = rag.clone();
+    let mut resposta_fixa: Option<String> = None;
+    let mut registro_triagem: Option<String> = None;
+    if let (Some((svc, faq)), Some(prompt_triagem)) = (&blocos_sf, &cfg.triagem_prompt)
+        && !(svc.is_empty() && faq.is_empty())
+    {
+        let rotulos = rotulos_servicos_faqs(svc.len(), faq.len());
+        let t_tri = Instant::now();
+        let (texto, resultado) =
+            match llm::triagem(&compor_system_prompt(prompt_triagem, &rag), entrada_llm).await {
+                Ok(txt) => {
+                    let r = triagem::descartados(&txt, &rotulos);
+                    (txt, r)
+                }
+                Err(e) => (String::new(), Err(format!("a chamada falhou: {e}"))),
+            };
+        let ms = t_tri.elapsed().as_millis();
+        registro_triagem = Some(match resultado {
+            Ok(fora) => {
+                info!(
+                    "Triagem: {} de {} documentos descartados",
+                    fora.len(),
+                    rotulos.len()
+                );
+                if fora.len() == rotulos.len() {
+                    // Nada trata da situação: a segunda chamada não teria o que ler. A resposta é
+                    // fixa, e não um contexto vazio — que é convite a alucinação.
+                    resposta_fixa = Some(SEM_DOCUMENTO_APLICAVEL.to_string());
+                } else {
+                    rag_resposta = montar_rag_servicos_faqs_sem(svc, faq, &fora);
+                }
+                let lista = fora.iter().cloned().collect::<Vec<_>>().join(", ");
+                format!(
+                    "tempo: {ms} ms · descartados: {} de {}{}\n{texto}",
+                    fora.len(),
+                    rotulos.len(),
+                    if lista.is_empty() {
+                        String::new()
+                    } else {
+                        format!(" ({lista})")
+                    },
+                )
+            }
+            Err(e) => {
+                warn!("triagem ignorada, contexto integral: {e}");
+                format!(
+                    "tempo: {ms} ms · FALHOU ({e}) — a resposta usou o contexto integral\n{texto}"
+                )
+            }
+        });
+    }
+
+    let resposta_llm = match resposta_fixa {
+        Some(fixa) => fixa,
+        None => {
+            // System prompt = base prompt (per query type) + RAG context, closed with the original
+            // delimiter. Com o marcador `{{CONTEXTO}}` no prompt, o que vem depois dele vai DEPOIS
+            // do contexto (D-SF-8).
+            let base_prompt = match query_type {
+                QueryType::ServicosFaqs => &cfg.system_prompt,
+                QueryType::ColecaoUnica(kind) => cfg.prompt_de(kind),
+            };
+            let system_prompt = compor_system_prompt(base_prompt, &rag_resposta);
+            trace!("System instructions with RAG: {}", system_prompt);
+            llm::chat(&system_prompt, entrada_llm).await?
+        }
+    };
     tempos.llm_ms = t.elapsed().as_millis() as u64;
 
     // Restaura os placeholders (`[CNPJ_1]` → valor original) antes de devolver — o usuário vê o valor
@@ -583,6 +691,7 @@ pub async fn exec_all_question(
         sanitizada: &pergunta_anon,
         answer: Some(&answer),
         aderencia: &montar_aderencia(&itens_aderencia),
+        triagem: registro_triagem.as_deref(),
         rag: &rag,
         tempos,
     })
@@ -610,7 +719,11 @@ pub(crate) struct RegistroConsulta<'a> {
     pub answer: Option<&'a str>,
     /// Seção de proximidade já montada por [`montar_aderencia`]; vazia = sem busca vetorial.
     pub aderencia: &'a str,
-    /// O contexto RAG — a MESMA string que foi ao prompt do LLM.
+    /// Seção da TRIAGEM (D-SF-9): tempo, descartados e a saída crua do modelo. `None` = não houve
+    /// triagem, e a seção é OMITIDA (o registro sai byte a byte como antes).
+    pub triagem: Option<&'a str>,
+    /// O contexto RAG recuperado — a MESMA string que foi ao prompt do LLM. Com triagem, é o que a
+    /// TRIAGEM viu; a resposta viu este contexto menos os descartados que a seção TRIAGEM lista.
     pub rag: &'a str,
     pub tempos: TemposConsulta,
 }
@@ -642,6 +755,7 @@ fn format_log_record(stamp: &str, reg: &RegistroConsulta) -> String {
         sanitizada,
         answer,
         aderencia,
+        triagem,
         rag,
         tempos,
     } = reg;
@@ -678,6 +792,11 @@ fn format_log_record(stamp: &str, reg: &RegistroConsulta) -> String {
         ),
         true => String::new(),
     };
+    // Mesma disciplina: sem triagem, a seção não existe (nem vazia).
+    let triagem = match triagem {
+        Some(t) => format!("{}\n{t}\n\n", secao("TRIAGEM (primeira chamada)")),
+        None => String::new(),
+    };
     format!(
         "{regua}\n\
          CONSULTA · {stamp} · entidade: {entidade} · tipo: {tipo}\n\
@@ -687,6 +806,7 @@ fn format_log_record(stamp: &str, reg: &RegistroConsulta) -> String {
          {}\n{original}\n\n\
          {}\n{sanitizada}\n\n\
          {resposta}\
+         {triagem}\
          {aderencia}\
          {}\n{rag}\n\
          {regua}",
@@ -795,11 +915,11 @@ pub(crate) fn log_question(reg: &RegistroConsulta) -> std::io::Result<Uuid> {
 #[cfg(test)]
 mod tests {
     use super::{
-        FAQ_BAND, FAQ_FLOOR, LEG_BAND, LEG_FLOOR, MARCADOR_CONTEXTO, PAR_BAND, PAR_FLOOR, QueryType,
-        RegistroConsulta, compor_system_prompt,
-        SVC_BAND, SVC_FLOOR, TemposConsulta, Uuid, aderencia, bandas, bloco_documento, envolver,
-        format_log_record, log_question, mes_do_log, montar_aderencia, montar_rag_colecao_unica,
-        montar_rag_servicos_faqs,
+        FAQ_BAND, FAQ_FLOOR, LEG_BAND, LEG_FLOOR, MARCADOR_CONTEXTO, PAR_BAND, PAR_FLOOR,
+        QueryType, RegistroConsulta, SVC_BAND, SVC_FLOOR, TemposConsulta, Uuid, aderencia, bandas,
+        bloco_documento, compor_system_prompt, envolver, format_log_record, log_question,
+        mes_do_log, montar_aderencia, montar_rag_colecao_unica, montar_rag_servicos_faqs,
+        montar_rag_servicos_faqs_sem, rotulos_servicos_faqs,
     };
     use auli_contract::{DocumentoPack, Kind, mddoc};
     use std::path::Path;
@@ -831,10 +951,7 @@ mod tests {
         // A trava da D-SF-8: os 26 prompts sem marcador não podem mudar um byte.
         let base = "\n'''\n### Regras\n";
         let rag = "\n## documento 1: Serviço\nX\n";
-        assert_eq!(
-            compor_system_prompt(base, rag),
-            format!("{base}{rag}'''"),
-        );
+        assert_eq!(compor_system_prompt(base, rag), format!("{base}{rag}'''"),);
     }
 
     #[test]
@@ -1023,6 +1140,7 @@ mod tests {
             sanitizada: "p",
             answer: Some("r"),
             aderencia: "",
+            triagem: None,
             rag: "",
             tempos: TemposConsulta {
                 embed_ms: 1,
@@ -1044,6 +1162,7 @@ mod tests {
                 sanitizada: "CNPJ [CNPJ_1] pode aderir?",
                 answer: Some("Sim, o CNPJ 11.222.333/0001-81 atende."),
                 aderencia: "parecer 1 · aderência 0.700 · distância 0.300\n",
+                triagem: None,
                 rag: "## documento 1: Parecer\n...",
                 tempos: TemposConsulta {
                     embed_ms: 12,
@@ -1102,6 +1221,7 @@ mod tests {
                 sanitizada: "como parcelar ICMS em atraso?",
                 answer: None,
                 aderencia: "servico 1 · aderência 0.550 · distância 0.450\n",
+                triagem: None,
                 rag: "\n## documento 1: Serviço\n...",
                 tempos: TemposConsulta {
                     embed_ms: 14,
@@ -1128,6 +1248,61 @@ mod tests {
         assert!(rec.contains("llm: 0 ms"), "{rec}");
     }
 
+    /// A seção TRIAGEM (D-SF-9) entra entre RESPOSTA e ADERÊNCIA quando existe — e só então.
+    #[test]
+    fn log_record_com_triagem_tem_a_secao_no_lugar_certo() {
+        let rec = format_log_record(
+            "2026-10-04 10:00:00",
+            &RegistroConsulta {
+                entidade: "rs",
+                tipo: "servicos+faqs",
+                original: "como pago o ICMS?",
+                sanitizada: "como pago o ICMS?",
+                answer: Some("Resposta."),
+                aderencia: "servico 1 · aderência 0.550 · distância 0.450\n",
+                triagem: Some("tempo: 900 ms · descartados: 1 de 2 (FAQ 1)\n{}"),
+                rag: "\n## documento 1: Serviço\n...",
+                tempos: TemposConsulta::default(),
+            },
+        );
+        let i_resp = rec.find("----- RESPOSTA").expect("resposta");
+        let i_tri = rec.find("----- TRIAGEM").expect("triagem");
+        let i_ader = rec.find("----- ADERÊNCIA").expect("aderência");
+        assert!(i_resp < i_tri && i_tri < i_ader, "{rec}");
+        assert!(rec.contains("descartados: 1 de 2 (FAQ 1)"), "{rec}");
+    }
+
+    #[test]
+    fn rotulos_seguem_a_numeracao_por_colecao() {
+        assert_eq!(
+            rotulos_servicos_faqs(2, 1),
+            vec!["Serviço 1", "Serviço 2", "FAQ 1"]
+        );
+        assert!(rotulos_servicos_faqs(0, 0).is_empty());
+    }
+
+    #[test]
+    fn contexto_sem_descartados_e_byte_a_byte_o_de_sempre() {
+        let svc = vec!["S1".to_string(), "S2".to_string()];
+        let faq = vec!["F1".to_string()];
+        assert_eq!(
+            montar_rag_servicos_faqs_sem(&svc, &faq, &Default::default()),
+            montar_rag_servicos_faqs(&svc, &faq),
+        );
+    }
+
+    #[test]
+    fn contexto_sem_descartados_mantem_a_numeracao_original() {
+        let svc = vec!["S1".to_string(), "S2".to_string(), "S3".to_string()];
+        let faq = vec!["F1".to_string(), "F2".to_string()];
+        let fora = std::collections::BTreeSet::from(["Serviço 2".to_string(), "FAQ 1".to_string()]);
+        let rag = montar_rag_servicos_faqs_sem(&svc, &faq, &fora);
+        assert!(rag.contains("## documento 1: Serviço\nS1"), "{rag}");
+        assert!(rag.contains("## documento 3: Serviço\nS3"), "{rag}");
+        assert!(rag.contains("## documento 2: FAQ\nF2"), "{rag}");
+        assert!(!rag.contains("S2") && !rag.contains("F1"), "{rag}");
+    }
+
     #[test]
     fn aderencia_numera_como_o_bloco_rag_e_converte_a_distancia() {
         // Índice 1-based, igual ao do bloco RAG — é o que permite cruzar as duas seções do log.
@@ -1165,6 +1340,7 @@ mod tests {
                 sanitizada: "CONSULTA COPAT nº 0091/17",
                 answer: None,
                 aderencia: "",
+                triagem: None,
                 rag: "{\"numero\":\"CONSULTA COPAT nº 0091/17\"}",
                 tempos: TemposConsulta {
                     retrieve_ms: 1,
@@ -1332,6 +1508,7 @@ mod tests {
             sanitizada: "pergunta",
             answer: Some("resposta"),
             aderencia: "",
+            triagem: None,
             rag: "contexto",
             tempos: TemposConsulta::default(),
         }
diff --git a/auli-server/crates/auli-cli/src/triagem.rs b/auli-server/crates/auli-cli/src/triagem.rs
new file mode 100644
index 0000000..07fd326
--- /dev/null
+++ b/auli-server/crates/auli-cli/src/triagem.rs
@@ -0,0 +1,153 @@
+//! Triagem dos documentos de serviços+FAQs antes da resposta (D-SF-9).
+//!
+//! A resposta do chat de atendimento passa a ser feita em **duas chamadas** ao LLM. A primeira — esta
+//! — recebe a pergunta e os documentos recuperados e devolve só um veredito por documento; a segunda
+//! redige a resposta vendo apenas os que sobreviveram. É a forma explícita da regra central da D-SF-1:
+//! os serviços são alternativas para situações excludentes, e o que não trata da situação perguntada
+//! nem deveria estar na mesa quando o texto é escrito.
+//!
+//! **Fail-open de propósito.** Triagem que falha (rede, JSON inválido, rótulo que não existe no
+//! contexto) devolve `Err`, e o chamador segue com o contexto INTEGRAL — exatamente a resposta de uma
+//! chamada só. A triagem é um filtro de qualidade, não uma guarda: perdê-la piora a resposta, mas
+//! não pode derrubá-la. Pela mesma razão, documento que o modelo **omitiu** do JSON é mantido: só
+//! sai o que foi explicitamente descartado.
+//!
+//! Este módulo é PURO (sem rede, sem I/O): interpreta o texto que o modelo devolveu. A chamada vive
+//! no `rag.rs`, junto da outra.
+
+use std::collections::BTreeSet;
+
+use serde::Deserialize;
+
+/// O que o modelo diz de UM documento.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
+#[serde(rename_all = "lowercase")]
+pub(crate) enum Veredito {
+    /// Trata da situação perguntada.
+    Aplica,
+    /// Trata da situação, mas depende de uma condição que a pergunta não informa (o regime, o
+    /// público). **Fica** no contexto: é justamente o irmão que a resposta deve apresentar como
+    /// alternativa, com a condição que decide.
+    Condicional,
+    /// Não trata da situação perguntada. Sai do contexto da segunda chamada.
+    Descarta,
+}
+
+#[derive(Debug, Deserialize)]
+struct Item {
+    documento: String,
+    veredito: Veredito,
+    // O `motivo` é pedido pelo prompt e lido só no log de auditoria (texto cru). Aceito e ignorado
+    // aqui, para que a ausência dele não derrube a triagem.
+    #[serde(default)]
+    #[allow(dead_code)]
+    motivo: String,
+}
+
+#[derive(Debug, Deserialize)]
+struct Resposta {
+    documentos: Vec<Item>,
+}
+
+/// Interpreta a resposta da triagem e devolve os rótulos **descartados** (`"Serviço 3"`, `"FAQ 7"`).
+///
+/// `rotulos` são os que existem no contexto enviado. Um rótulo desconhecido na resposta é `Err`:
+/// o modelo está falando de um documento que não viu, e nada do que ele disse sobre os outros
+/// merece confiança.
+pub(crate) fn descartados(resposta: &str, rotulos: &[String]) -> Result<BTreeSet<String>, String> {
+    let json = extrair_objeto(resposta).ok_or("a resposta não contém um objeto JSON")?;
+    let r: Resposta = serde_json::from_str(json).map_err(|e| format!("JSON inválido: {e}"))?;
+    let mut fora = BTreeSet::new();
+    for item in r.documentos {
+        let rotulo = normalizar(&item.documento);
+        if !rotulos.contains(&rotulo) {
+            return Err(format!("rótulo fora do contexto: '{}'", item.documento));
+        }
+        if item.veredito == Veredito::Descarta {
+            fora.insert(rotulo);
+        }
+    }
+    Ok(fora)
+}
+
+/// O trecho entre a primeira `{` e a última `}`. Cobre a cerca markdown (```` ```json ````) e o
+/// texto antes ou depois que o modelo às vezes acrescenta apesar do "JSON puro".
+fn extrair_objeto(s: &str) -> Option<&str> {
+    let ini = s.find('{')?;
+    let fim = s.rfind('}')?;
+    (fim > ini).then(|| &s[ini..=fim])
+}
+
+/// `"[Serviço 3]"`, `" serviço  3 "` → `"Serviço 3"`. Tolera colchetes, caixa e espaços; o tipo
+/// volta à grafia canônica do rótulo para casar com a lista do contexto.
+fn normalizar(s: &str) -> String {
+    let limpo: String = s
+        .trim()
+        .trim_matches(|c| c == '[' || c == ']')
+        .split_whitespace()
+        .collect::<Vec<_>>()
+        .join(" ");
+    let mut partes = limpo.splitn(2, ' ');
+    let tipo = partes.next().unwrap_or_default();
+    let resto = partes.next().unwrap_or_default();
+    let tipo = match tipo.to_lowercase().as_str() {
+        "serviço" | "servico" => "Serviço",
+        "faq" => "FAQ",
+        _ => tipo,
+    };
+    format!("{tipo} {resto}").trim().to_string()
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn rotulos() -> Vec<String> {
+        ["Serviço 1", "Serviço 2", "FAQ 1", "FAQ 2"]
+            .iter()
+            .map(|s| s.to_string())
+            .collect()
+    }
+
+    #[test]
+    fn so_o_descartado_sai_e_condicional_fica() {
+        let r = r#"{"documentos": [
+            {"documento": "Serviço 1", "veredito": "aplica", "motivo": "x"},
+            {"documento": "Serviço 2", "veredito": "condicional", "motivo": "regime"},
+            {"documento": "FAQ 1", "veredito": "descarta", "motivo": "outro tema"}
+        ]}"#;
+        let fora = descartados(r, &rotulos()).unwrap();
+        // A FAQ 2 foi omitida pelo modelo: fica (fail-open por documento).
+        assert_eq!(fora, BTreeSet::from(["FAQ 1".to_string()]));
+    }
+
+    #[test]
+    fn cerca_markdown_e_texto_em_volta_sao_tolerados() {
+        let r = "Segue:\n```json\n{\"documentos\": [{\"documento\": \"FAQ 2\", \"veredito\": \"descarta\"}]}\n```";
+        let fora = descartados(r, &rotulos()).unwrap();
+        assert_eq!(fora, BTreeSet::from(["FAQ 2".to_string()]));
+    }
+
+    #[test]
+    fn rotulo_com_colchete_e_caixa_diferente_e_normalizado() {
+        let r = r#"{"documentos": [{"documento": "[servico 2]", "veredito": "descarta"}]}"#;
+        let fora = descartados(r, &rotulos()).unwrap();
+        assert_eq!(fora, BTreeSet::from(["Serviço 2".to_string()]));
+    }
+
+    #[test]
+    fn rotulo_fora_do_contexto_invalida_a_triagem_inteira() {
+        let r = r#"{"documentos": [
+            {"documento": "FAQ 1", "veredito": "descarta"},
+            {"documento": "Serviço 9", "veredito": "aplica"}
+        ]}"#;
+        assert!(descartados(r, &rotulos()).is_err());
+    }
+
+    #[test]
+    fn veredito_desconhecido_e_resposta_sem_json_sao_erro() {
+        let r = r#"{"documentos": [{"documento": "FAQ 1", "veredito": "talvez"}]}"#;
+        assert!(descartados(r, &rotulos()).is_err());
+        assert!(descartados("não sei", &rotulos()).is_err());
+    }
+}
diff --git a/auli-server/crates/auli-llm/src/lib.rs b/auli-server/crates/auli-llm/src/lib.rs
index f37d8fb..2ff3401 100644
--- a/auli-server/crates/auli-llm/src/lib.rs
+++ b/auli-server/crates/auli-llm/src/lib.rs
@@ -120,7 +120,7 @@ pub async fn chat(
         for attempt in 1u32..=3 {
             let outcome = async {
                 // Timeout POR REQUISIÇÃO, mantido pelo chamador abaixo do seu próprio budget — no
-                // servidor, os 35 s do frontend (callServerAPI.ts): um LLM pendurado vira timeout
+                // servidor, os 70 s do frontend (callServerAPI.ts): um LLM pendurado vira timeout
                 // retryável aqui em vez de o cliente desistir com a chamada paga ainda aberta.
                 let resp = CLIENT
                     .post(params.api_url.as_str())
diff --git a/config/prompts/rs-triagem.txt b/config/prompts/rs-triagem.txt
new file mode 100644
index 0000000..d18cfaa
--- /dev/null
+++ b/config/prompts/rs-triagem.txt
@@ -0,0 +1,14 @@
+
+'''
+### Triagem
+### Você recebe a pergunta de um Analista de Tributação e uma lista de documentos do Portal de Atendimento (serviços e perguntas frequentes). Sua única tarefa é classificar cada documento. NÃO responda à pergunta.
+### Cada documento começa com o marcador ## documento, seguido do número e do tipo (Serviço ou FAQ). O rótulo de um documento é o tipo seguido do número, por exemplo "Serviço 2" ou "FAQ 4". A numeração é separada por tipo: Serviço 1 e FAQ 1 são documentos diferentes.
+### Cada serviço e cada FAQ vale para uma situação específica (por exemplo: regime geral ou Simples Nacional; cidadão ou empresa; um tipo de operação). Classifique cada documento em um destes vereditos:
+### - "aplica": o documento trata da situação perguntada.
+### - "condicional": o documento trata do assunto, mas vale para uma situação específica que a pergunta não informa (o regime, o público, o tipo de operação).
+### - "descarta": o documento não trata da situação perguntada, ou vale para uma situação que a pergunta exclui.
+### Na dúvida entre "condicional" e "descarta", escolha "condicional".
+{{CONTEXTO}}
+### Responda SOMENTE com um objeto JSON válido, sem markdown e sem texto antes ou depois, neste formato exato:
+### {"documentos": [{"documento": "Serviço 1", "veredito": "aplica", "motivo": "frase curta"}]}
+### Inclua TODOS os documentos acima, um item por documento, com o rótulo exato. O "motivo" diz em uma frase curta por que o documento recebeu aquele veredito.
diff --git a/config/registry.toml b/config/registry.toml
index 4370cf5..7bd9dc6 100644
--- a/config/registry.toml
+++ b/config/registry.toml
@@ -51,6 +51,11 @@ prompt_tarf = "prompts/rs-tarf.txt"
 # sem prompt próprio ainda é jurisprudência, e legislação não é: cair no de pareceres mandaria o
 # modelo citar "o número do parecer" sobre um texto de lei.
 prompt_legislacao = "prompts/rs-legislacao.txt"
+# Prompt da TRIAGEM das consultas de serviços+FAQs (query type=1, D-SF-9). Opcional e SEM default:
+# com ele, a resposta sai de DUAS chamadas ao LLM — a primeira classifica cada documento recuperado
+# (aplica / condicional / descarta) e a segunda redige vendo só os que não foram descartados. Sem
+# ele, uma chamada só, como antes. A triagem que falha segue com o contexto integral (fail-open).
+prompt_triagem = "prompts/rs-triagem.txt"
 # `tarf` é a coleção dos acórdãos do Tribunal Administrativo de Recursos Fiscais. Ela dirige DUAS
 # coisas: a aba de listagem (que lê o `rs-tarf-index.json`) e a fonte "Acórdãos TARF" do chat (que lê
 # o pack vetorial `rs-tarf`).
diff --git a/docs/auli_code.md b/docs/auli_code.md
index bde1dd3..90ac5bb 100644
--- a/docs/auli_code.md
+++ b/docs/auli_code.md
@@ -240,6 +240,7 @@ pedido.
 | **D-SF-6** | Esforço de raciocínio do chat vem do `.env` (`LLM_REASONING_EFFORT` = `low`/`medium`/`high`); ausente = o campo não é enviado. `max_completion_tokens` do chat 4096 → **8192** | O `gpt-oss` confere melhor as próprias regras com esforço alto, e o `auli-llm` já suportava o campo — faltava o chat passá-lo. Variável, e não constante, porque o parâmetro é recusado por modelos que não são de raciocínio: trocar de modelo não pode exigir recompilar. Valor inválido **aborta o boot** (o erro de digitação não vira default em silêncio). Os 8192 existem porque, nesses modelos, a saída inclui o raciocínio. Vale para todos os tipos de consulta do chat. |
 | **D-SF-7** | O `rs.txt` exige **citação por afirmação** no formato do marcador do contexto — `[Serviço 2]`, `[FAQ 4]` —, e afirmação sem documento para citar não se escreve | Amarra cada frase à fonte, o que dificulta a fusão entre documentos e torna a conferência mecânica (o rótulo citado existe no contexto? a frase está nele?). O tipo vai junto do número porque a numeração é **por coleção** (D-MARC-3): "[doc 3]" seria ambíguo entre o Serviço 3 e a FAQ 3. A lista de fontes da resposta usa os mesmos rótulos, para o analista bater uma com a outra. |
 | **D-SF-8** | Marcador `{{CONTEXTO}}` no prompt de sistema: o que vem antes dele fica antes dos documentos, o que vem depois vai **depois** do contexto e do `'''` (`rag::compor_system_prompt`). O `rs.txt` usa o espaço para repetir a regra central | Com 15 documentos entre as regras e a pergunta, a regra que mais importa ficava longe do ponto em que o modelo redige. A decisão de O QUE repetir é do prompt (catálogo), não do código. Prompt **sem** marcador sai byte a byte como antes — é o caso dos outros 26 estados, e há teste que trava isso. Divide na primeira ocorrência; o marcador nunca chega ao modelo. |
+| **D-SF-9** | **Triagem em duas chamadas** (`triagem.rs` + `rag.rs`), opt-in por entidade pelo campo `prompt_triagem` do registry (só o `rs`, com `config/prompts/rs-triagem.txt`). A 1ª chamada recebe a pergunta e os documentos e devolve JSON com um veredito por rótulo — `aplica`, `condicional` ou `descarta`; a 2ª redige vendo o contexto **sem os descartados**, com a **numeração original** preservada (`montar_rag_servicos_faqs_sem`). Tudo descartado ⇒ resposta fixa, sem 2ª chamada. Triagem que falha (rede, teto de tempo, JSON inválido, rótulo inexistente) ⇒ segue com o contexto **integral** (fail-open); documento omitido no JSON **fica**. Teto de **20 s para o passo inteiro** (`tokio::time::timeout` por fora, porque o `auli-llm` repete até 3 vezes em timeout); o frontend sobe de 35 s para **70 s** | É a forma explícita da D-SF-1: o que não trata da situação perguntada nem deveria estar na mesa quando o texto é escrito. Escolhida em vez da classificação dentro do raciocínio de uma chamada só, **ciente de que dobra a latência**. `condicional` fica no contexto porque é justamente o irmão que a resposta deve apresentar como alternativa — e, na dúvida, o prompt manda escolher `condicional`, não `descarta`. A numeração original é o que mantém citação (D-SF-7), ADERÊNCIA do log e lista de fontes falando dos mesmos rótulos. Fail-open porque a triagem é filtro de qualidade, não guarda: perdê-la volta à resposta de uma chamada só, nunca derruba a consulta. O log ganha a seção **TRIAGEM** (tempo, descartados e a saída crua do modelo, incluindo o `motivo` de cada veredito), e o `CONTEXTO RAG` continua sendo o **recuperado inteiro** — o que a triagem viu —, o que preserva o sentido do `parity-replay.py`. O MCP não tem triagem: entrega o contexto integral e o assistente externo faz a dele. |
 
 ### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)
 
@@ -660,7 +661,7 @@ A versão exibida e um `__BUILD_ID__` para cache-busting são injetados em build
   `import.meta.env.VITE_API_URL ?? "https://api.auli.com.br/v1/question"`. Passa
   `entityId: entity.id` ao backend.
 - [pages/chat/utils/callServerAPI.ts](auli-frontend/src/pages/chat/utils/callServerAPI.ts):
-  `POST` via axios do corpo `{ question, entity? }`, com **timeout de 25s** via
+  `POST` via axios do corpo `{ question, entity? }`, com **timeout de 70 s** via
   `AbortController`; mensagens de erro/timeout em pt-BR. Lê `res.data.answer`.
 - [pages/chat/utils/prompt.ts](auli-frontend/src/pages/chat/utils/prompt.ts): validação do
   prompt (mínimo de 10 caracteres) — fonte única usada pelo guard de envio, botão e contador.
@@ -868,7 +869,7 @@ mantém um espelho **gerado** (não mais divergente) do registro. Pendências em
   `faqs` (10) — eram 10 + 20 até 04/10/2026, ver §3.4.1; estreitamento por proximidade presente mas
   em modo paridade (`band=∞`) até calibração.
 - `auli-frontend`: SPA com seleção de entidade (as 12), chat contra `POST /v1/question` com
-  timeout de 25s, abas de referência lendo `public/<id>/` (arquivos prefixados `<id>-`), tema
+  timeout de 70 s, abas de referência lendo `public/<id>/` (arquivos prefixados `<id>-`), tema
   claro/escuro, testes Vitest.
 - **Scrapers por entidade** (`auli-scraper-{rs,sc,sp,pr,mg,pe,ba,rj,ce,ms,mt,go}` sobre `auli-scraper-kit`): FAQs (rs) e
   serviços (rs API JSON, sc JSON, sp SharePoint, pr Drupal, mg ServiceNow, pe/ba/rj/ms HTML, ce/mt/go JSON — go via curl por WAF JA3), cache + `--usecache`,
diff --git a/docs/auli_features.md b/docs/auli_features.md
index 54ec83a..b1482eb 100644
--- a/docs/auli_features.md
+++ b/docs/auli_features.md
@@ -82,7 +82,7 @@ Os conteúdos oficiais são previamente **coletados do portal** da secretaria (s
   em destaque) e cards das secretarias; a escolha é **persistida** no navegador e pode ser
   trocada pelo cabeçalho.
 - **Chat conversacional** com balões diferenciados para usuário e assistente, mensagem de
-  "pensando", **botão de copiar** mensagens e **timeout amigável** (25s) quando a API demora.
+  "pensando", **botão de copiar** mensagens e **timeout amigável** (70 s) quando a API demora.
 - **Navegação por abas** (sem recarga de página), em três grupos: **Acervo** (Serviços, FAQs,
   Pareceres, Acórdãos TARF, Notas, Conteúdos), **Integrações** (Conectar sua IA, Downloads) e o
   Chat, mais o Sobre no rodapé — com o estado de cada aba preservado ao alternar. Aba de coleção que
diff --git a/docs/auli_pendencias.md b/docs/auli_pendencias.md
index 381f1ee..1c6bb3a 100644
--- a/docs/auli_pendencias.md
+++ b/docs/auli_pendencias.md
@@ -1559,7 +1559,7 @@ público raspado. Nada disso é o mantenedor — e uma varredura cega por `carlo
 
 ## 40. Serviços + FAQs: a resposta fundia serviços excludentes — prompt v2 e 5 + 10 (aberta — 2026-10-04)
 
-Decisões e mecanismo em [docs/auli_code.md](docs/auli_code.md) §3.4.1 (D-SF-1..5). Em resumo: os
+Decisões e mecanismo em [docs/auli_code.md](docs/auli_code.md) §3.4.1 (D-SF-\*). Em resumo: os
 serviços são alternativas para situações excludentes (regime geral × Simples Nacional, cidadão ×
 empresa), a busca densa traz justamente os irmãos, e o chat — com 30 documentos e um prompt que
 pedia "a orientação completa" — fundia regras que não se aplicam juntas. Entraram o `rs.txt` v2,
@@ -1581,6 +1581,12 @@ que proíbe a síntese entre documentos, e o corte para 5 serviços + 10 FAQs.
    quem cuida do conteúdo do portal.
 4. **Não reabrir sem dado:** as bandas continuam ∞/piso 0. O corte 5 + 10 é teto; a calibragem
    segue a doutrina do `ESTUDO-busca-hibrida.md` §9.
+5. **A triagem (D-SF-9) precisa de medida própria.** Três números a ler nos logs depois de uma
+   semana de uso: a latência somada (a seção TEMPOS e o `tempo:` da TRIAGEM — o frontend corta em
+   70 s); a taxa de falha (linhas `FALHOU` na seção TRIAGEM — cada uma é uma resposta que voltou ao
+   contexto integral); e os **descartes errados** — documento descartado que era o certo. Este
+   último é o que decide se a triagem fica: ela troca ruído por risco de recall, e o `motivo` de
+   cada veredito está no log justamente para essa leitura.
 
 ---
 
-- 
2.43.0

~~~~~

### 0006-D-SF-10.patch — D-SF-10 — lista fixa de fontes

~~~~~patch 0006-D-SF-10.patch
From 2d52864a2e475d5b9853ecaf01eb6c0b39bf572e Mon Sep 17 00:00:00 2001
From: ubuntu <carlos.schellenberger@gmail.com>
Date: Sun, 4 Oct 2026 17:13:57 +0000
Subject: [PATCH 6/6] =?UTF-8?q?feat(chat):=20lista=20fixa=20de=20fontes=20?=
 =?UTF-8?q?na=20resposta=20de=20Servi=C3=A7os=20+=20FAQs=20(D-SF-10)?=
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

O analista passa a ver sempre o que a busca trouxe, independentemente do que o
texto do LLM decidiu citar — o núcleo do Auli é achar a informação, e o texto
gerado é complemento.

Backend:
- `POST /v1/question` ganha o campo `fontes` (`rotulo`, `titulo`, `link`,
  `triagem`), montado por `rag::fontes_servicos_faqs` a partir dos payloads.
  O `rotulo` é exatamente o da citação (D-SF-7), na ordem do contexto.
- Um item por documento, mesmo com payload ilegível (título de aviso, sem
  link): sem isso a numeração deixaria de bater com a citação.
- O veredito vem da triagem pelo mesmo rótulo. Para isso o `triagem.rs` passa
  a devolver o mapa completo (`vereditos`), e os descartados saem dele.
- Campo vazio não serializa: cliente antigo e os tipos 2–4 não veem diferença
  (teste do contrato de fio no dto.rs).
- `exec_all_question` devolve `RespostaChat` em vez da tupla.

Frontend:
- `Fontes.tsx`: `<details>` RECOLHIDO, "Fontes consultadas: 15 (3 descartadas
  na triagem)". Aberta, empurraria 15 linhas abaixo de cada resposta no
  celular. Descartada aparece esmaecida e riscada, não some — o descarte
  errado tem que ser visível. Condicional ganha o selo "depende de condição".
- Só em Serviços + FAQs, por decisão; os demais tipos seguem sem lista.

Decisão em docs/auli_code.md §3.4.1; auli_features.md atualizado.

Verificado: cargo fmt --check; 137 testes do auli-cli (3 novos); frontend com
tsc, lint e build limpos e 246 testes (5 novos). Sem clippy no ambiente.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
---
 auli-frontend/src/pages/chat/Fontes.test.tsx  |  50 ++++++
 auli-frontend/src/pages/chat/Fontes.tsx       |  72 +++++++++
 auli-frontend/src/pages/chat/Messages.tsx     |   1 +
 .../src/pages/chat/SystemMessage.tsx          |   8 +-
 .../pages/chat/utils/callServerAPI.test.ts    |  11 ++
 .../src/pages/chat/utils/callServerAPI.ts     |   5 +-
 auli-frontend/src/shared/markdown.tsx         |   2 +-
 auli-frontend/src/types/chat.ts               |  13 ++
 auli-server/crates/auli-cli/src/api/dto.rs    |  41 ++++-
 .../auli-cli/src/api/handlers/question.rs     |  11 +-
 auli-server/crates/auli-cli/src/rag.rs        | 142 ++++++++++++++++--
 auli-server/crates/auli-cli/src/triagem.rs    |  70 +++++++--
 docs/auli_code.md                             |   6 +-
 docs/auli_features.md                         |   2 +
 14 files changed, 395 insertions(+), 39 deletions(-)
 create mode 100644 auli-frontend/src/pages/chat/Fontes.test.tsx
 create mode 100644 auli-frontend/src/pages/chat/Fontes.tsx

diff --git a/auli-frontend/src/pages/chat/Fontes.test.tsx b/auli-frontend/src/pages/chat/Fontes.test.tsx
new file mode 100644
index 0000000..ce202b4
--- /dev/null
+++ b/auli-frontend/src/pages/chat/Fontes.test.tsx
@@ -0,0 +1,50 @@
+// @vitest-environment jsdom
+import { describe, it, expect } from "vitest";
+import { screen } from "@testing-library/react";
+import { renderWithProvider } from "../../test/render";
+import { Fontes, resumoFontes } from "./Fontes";
+import { SystemMessage } from "./SystemMessage";
+import type { Fonte } from "../../types/chat";
+
+const fontes: Fonte[] = [
+  { rotulo: "Serviço 1", titulo: "Emitir guia", link: "https://s/1", triagem: "aplica" },
+  { rotulo: "Serviço 2", titulo: "Emitir guia (Simples)", link: "https://s/2", triagem: "condicional" },
+  { rotulo: "FAQ 1", titulo: "Como pagar?", link: "https://f/1", triagem: "descarta" },
+  { rotulo: "FAQ 2", titulo: "Prazo", link: "" },
+];
+
+describe("resumoFontes", () => {
+  it("conta o total e, quando houver, os descartados — com o plural certo", () => {
+    expect(resumoFontes(fontes)).toBe("Fontes consultadas: 4 (1 descartada na triagem)");
+    expect(resumoFontes([fontes[0]])).toBe("Fontes consultadas: 1");
+    const duas = fontes.map((f) => ({ ...f, triagem: "descarta" as const }));
+    expect(resumoFontes(duas)).toBe("Fontes consultadas: 4 (4 descartadas na triagem)");
+  });
+});
+
+describe("Fontes", () => {
+  it("mostra rótulo da citação, título em link e o selo do veredito", () => {
+    const { container } = renderWithProvider(<Fontes fontes={fontes} />);
+    // Recolhida por padrão.
+    expect(container.querySelector("details")?.hasAttribute("open")).toBe(false);
+    expect(screen.getByText("[Serviço 2]")).toBeInTheDocument();
+    const link = screen.getByText("Emitir guia (Simples)").closest("a");
+    expect(link).toHaveAttribute("href", "https://s/2");
+    expect(link).toHaveAttribute("target", "_blank");
+    expect(screen.getByText(/depende de condição/)).toBeInTheDocument();
+    expect(screen.getByText(/descartada na triagem$/)).toBeInTheDocument();
+    // Sem link: o título aparece como texto, não como âncora.
+    expect(screen.getByText("Prazo").closest("a")).toBeNull();
+  });
+});
+
+describe("SystemMessage + Fontes", () => {
+  it("desenha a lista só quando há fontes", () => {
+    const { rerender } = renderWithProvider(
+      <SystemMessage messageText="Resposta" showButton fontes={fontes} />,
+    );
+    expect(screen.getByText(/Fontes consultadas: 4/)).toBeInTheDocument();
+    rerender(<SystemMessage messageText="Resposta" showButton />);
+    expect(screen.queryByText(/Fontes consultadas/)).toBeNull();
+  });
+});
diff --git a/auli-frontend/src/pages/chat/Fontes.tsx b/auli-frontend/src/pages/chat/Fontes.tsx
new file mode 100644
index 0000000..6b630a3
--- /dev/null
+++ b/auli-frontend/src/pages/chat/Fontes.tsx
@@ -0,0 +1,72 @@
+import { Box } from "@chakra-ui/react";
+import type { Fonte } from "../../types/chat";
+import { toAbsoluteHref } from "../../shared/markdown";
+
+/** Como o analista lê o veredito da triagem (D-SF-9). `aplica` não ganha selo: é o caso normal. */
+const VEREDITO: Record<NonNullable<Fonte["triagem"]>, string> = {
+  aplica: "",
+  condicional: "depende de condição",
+  descarta: "descartada na triagem",
+};
+
+/** A linha do resumo: "Fontes consultadas: 15 (3 descartadas na triagem)". */
+export function resumoFontes(fontes: Fonte[]): string {
+  const descartadas = fontes.filter((f) => f.triagem === "descarta").length;
+  const base = `Fontes consultadas: ${fontes.length}`;
+  if (descartadas === 0) return base;
+  return `${base} (${descartadas} ${descartadas === 1 ? "descartada" : "descartadas"} na triagem)`;
+}
+
+interface FontesProps {
+  fontes: Fonte[];
+}
+
+/**
+ * A lista FIXA dos documentos que a busca trouxe (D-SF-10), abaixo da resposta de Serviços + FAQs.
+ *
+ * Existe para que o analista veja sempre as fontes, independentemente do que o texto do LLM decidiu
+ * citar — o núcleo do Auli é achar a informação, e o texto gerado é complemento. Os rótulos são os
+ * mesmos da citação no texto (`[Serviço 2]`), na ordem do contexto. Recolhida por padrão: com até 15
+ * itens, aberta empurraria a próxima pergunta para longe no celular.
+ */
+export const Fontes = ({ fontes }: FontesProps) => (
+  <Box as="details" mt={3} fontSize="13px" lineHeight="1.5" color="fg">
+    <Box as="summary" cursor="pointer" color="fg.muted" userSelect="none">
+      {resumoFontes(fontes)}
+    </Box>
+    <Box as="ul" listStyleType="none" pl={0} mt={2} mb={0}>
+      {fontes.map((f) => {
+        const descartada = f.triagem === "descarta";
+        const selo = f.triagem ? VEREDITO[f.triagem] : "";
+        return (
+          <Box as="li" key={f.rotulo} py="2px" opacity={descartada ? 0.6 : 1}>
+            <Box as="span" fontWeight="600" mr={2} whiteSpace="nowrap">
+              [{f.rotulo}]
+            </Box>
+            {f.link ? (
+              <a
+                href={toAbsoluteHref(f.link)}
+                target="_blank"
+                rel="noopener noreferrer"
+                style={{
+                  color: "var(--chakra-colors-accent)",
+                  textDecoration: descartada ? "line-through" : "underline",
+                  textUnderlineOffset: "2px",
+                }}
+              >
+                {f.titulo}
+              </a>
+            ) : (
+              <span>{f.titulo}</span>
+            )}
+            {selo && (
+              <Box as="span" ml={2} color="fg.muted" fontSize="11px">
+                · {selo}
+              </Box>
+            )}
+          </Box>
+        );
+      })}
+    </Box>
+  </Box>
+);
diff --git a/auli-frontend/src/pages/chat/Messages.tsx b/auli-frontend/src/pages/chat/Messages.tsx
index 422e458..776eef0 100644
--- a/auli-frontend/src/pages/chat/Messages.tsx
+++ b/auli-frontend/src/pages/chat/Messages.tsx
@@ -31,6 +31,7 @@ export const Messages = ({ messages, setPrompt }: MessagesProps) => {
               messageText={messageText}
               showButton={item.showButton}
               logId={item.logId}
+              fontes={item.fontes}
             />
           );
         }
diff --git a/auli-frontend/src/pages/chat/SystemMessage.tsx b/auli-frontend/src/pages/chat/SystemMessage.tsx
index b8ca6d4..f900cf5 100644
--- a/auli-frontend/src/pages/chat/SystemMessage.tsx
+++ b/auli-frontend/src/pages/chat/SystemMessage.tsx
@@ -6,12 +6,16 @@ import ReactMarkdown from "react-markdown";
 import { utilsCopyTextToClipboard } from "./utils/utils";
 import { compactMarkdownComponents, markdownPlugins } from "../../shared/markdown";
 import { LogModal } from "./LogModal";
+import { Fontes } from "./Fontes";
+import type { Fonte } from "../../types/chat";
 
 interface SystemMessageProps {
   messageText: string;
   showButton: boolean;
   /** Quando presente, oferece o ícone que abre o registro de auditoria desta resposta. */
   logId?: string;
+  /** Lista de fontes recuperadas (D-SF-10); ausente ou vazia = nada é desenhado. */
+  fontes?: Fonte[];
 }
 
 /** Aviso exibido no rodapé de toda resposta gerada (não aparece na saudação,
@@ -21,7 +25,7 @@ const DISCLAIMER =
   "automaticamente, podem variar e conter imprecisões, e têm caráter apenas informativo — " +
   "confira sempre as fontes e os links indicados.";
 
-export const SystemMessage = ({ messageText, showButton, logId }: SystemMessageProps) => {
+export const SystemMessage = ({ messageText, showButton, logId, fontes }: SystemMessageProps) => {
   const [logAberto, setLogAberto] = useState(false);
 
   return (
@@ -49,6 +53,8 @@ export const SystemMessage = ({ messageText, showButton, logId }: SystemMessageP
           </ReactMarkdown>
         </Box>
 
+        {showButton && fontes && fontes.length > 0 && <Fontes fontes={fontes} />}
+
         {showButton && (
           <Box
             mt={2}
diff --git a/auli-frontend/src/pages/chat/utils/callServerAPI.test.ts b/auli-frontend/src/pages/chat/utils/callServerAPI.test.ts
index bafc2b3..ccc5bf5 100644
--- a/auli-frontend/src/pages/chat/utils/callServerAPI.test.ts
+++ b/auli-frontend/src/pages/chat/utils/callServerAPI.test.ts
@@ -69,6 +69,17 @@ describe("callServerAPI", () => {
     expect(firstUpdate[2].text).toMatch(/pensando/i);
   });
 
+  it("leva as fontes da resposta para a mensagem (D-SF-10)", async () => {
+    const fontes = [{ rotulo: "Serviço 1", titulo: "Emitir guia", link: "https://s/1" }];
+    mockedPost.mockResolvedValue({ data: { answer: "ok", fontes } });
+    const { setMessages, args } = makeArgs();
+
+    await callServerAPI(args);
+
+    const final = finalMessages(setMessages);
+    expect(final[final.length - 1].fontes).toEqual(fontes);
+  });
+
   it("replaces the placeholder with the server answer and clears the prompt", async () => {
     mockedPost.mockResolvedValue({ data: { answer: "Resposta do servidor" } });
     const { setMessages, setPrompt, args } = makeArgs();
diff --git a/auli-frontend/src/pages/chat/utils/callServerAPI.ts b/auli-frontend/src/pages/chat/utils/callServerAPI.ts
index eda475f..d750581 100644
--- a/auli-frontend/src/pages/chat/utils/callServerAPI.ts
+++ b/auli-frontend/src/pages/chat/utils/callServerAPI.ts
@@ -1,6 +1,6 @@
 import axios from "axios";
 import type { Dispatch, SetStateAction } from "react";
-import type { Message } from "../../../types/chat";
+import type { Fonte, Message } from "../../../types/chat";
 import type { QuestionType } from "./useQuestionType";
 
 interface CallServerAPIArgs {
@@ -22,6 +22,8 @@ interface QuestionResponse {
   /** UUID do registro de auditoria (D-LOG-2). **Ausente quando a gravação falhou** — o chat não
    *  falha por causa do log, só deixa de oferecer o ícone. */
   log_id?: string;
+  /** Fontes recuperadas (D-SF-10). Só vem em Serviços + FAQs; ausente nos demais tipos. */
+  fontes?: Fonte[];
 }
 
 /** Abort the request if the server hasn't responded within this window.
@@ -91,6 +93,7 @@ export const callServerAPI = async ({
       text: serverMessage,
       showButton: true,
       logId: res.data?.log_id,
+      fontes: res.data?.fontes,
     });
 
     setPrompt("");
diff --git a/auli-frontend/src/shared/markdown.tsx b/auli-frontend/src/shared/markdown.tsx
index d36a834..df9504c 100644
--- a/auli-frontend/src/shared/markdown.tsx
+++ b/auli-frontend/src/shared/markdown.tsx
@@ -53,7 +53,7 @@ const tableComponents: Components = {
 /** LLM/portal links sometimes arrive without a scheme (e.g. `www.legislacao…` as a markdown-link
  *  target). A scheme-less href is resolved as relative and hangs when clicked, so force any
  *  external-looking URL to an absolute `http://` one. In-app (`/…`) and anchor (`#…`) links pass through. */
-function toAbsoluteHref(href?: string): string | undefined {
+export function toAbsoluteHref(href?: string): string | undefined {
   if (!href) return href;
   if (/^[a-z][a-z0-9+.-]*:/i.test(href)) return href; // already has a scheme (http:, https:, mailto:, tel:)
   if (href.startsWith("/") || href.startsWith("#")) return href;
diff --git a/auli-frontend/src/types/chat.ts b/auli-frontend/src/types/chat.ts
index 2531bbb..d3dd510 100644
--- a/auli-frontend/src/types/chat.ts
+++ b/auli-frontend/src/types/chat.ts
@@ -3,6 +3,17 @@ import type { Dispatch, SetStateAction } from "react";
 /** Who authored a chat message. */
 export type MessageSender = "user" | "server";
 
+/** Um documento recuperado para a resposta, como a lista de fontes o mostra (D-SF-10). O `rotulo`
+ *  é o mesmo da citação no texto (`"Serviço 2"`, `"FAQ 4"`), para o analista bater um com o outro. */
+export interface Fonte {
+  rotulo: string;
+  titulo: string;
+  link: string;
+  /** Veredito da triagem (D-SF-9). Ausente = o documento foi à resposta sem veredito (sem triagem,
+   *  triagem que falhou, ou documento que o modelo omitiu). */
+  triagem?: "aplica" | "condicional" | "descarta";
+}
+
 /** A single message rendered in the chat transcript. */
 export interface Message {
   id: string;
@@ -13,6 +24,8 @@ export interface Message {
    *  Ausente na saudação, nas mensagens do usuário e quando o log falhou — e é essa ausência que
    *  esconde o ícone: não existe registro para abrir. */
   logId?: string;
+  /** Lista de fontes recuperadas (D-SF-10) — só em respostas de Serviços + FAQs. */
+  fontes?: Fonte[];
 }
 
 /** Setter returned by `usePrompt`, matching React's `useState<string>` setter. */
diff --git a/auli-server/crates/auli-cli/src/api/dto.rs b/auli-server/crates/auli-cli/src/api/dto.rs
index 675e49e..a5ec07f 100644
--- a/auli-server/crates/auli-cli/src/api/dto.rs
+++ b/auli-server/crates/auli-cli/src/api/dto.rs
@@ -40,6 +40,10 @@ pub struct Answer {
     /// sair sem a chave em vez de com `null`.
     #[serde(skip_serializing_if = "Option::is_none")]
     pub log_id: Option<String>,
+    /// A lista de fontes recuperadas (D-SF-10) — só no tipo serviços+FAQs. Vazia ⇒ a chave não
+    /// sai, e o cliente antigo, que não a conhece, não vê diferença nenhuma.
+    #[serde(default, skip_serializing_if = "Vec::is_empty")]
+    pub fontes: Vec<crate::rag::Fonte>,
 }
 
 /// Corpo do POST /v1/retrieve. `kind` usa o vocabulário único de `corpus::from_kind`.
@@ -77,7 +81,8 @@ pub struct RetrieveResponse {
 
 #[cfg(test)]
 mod tests {
-    use super::{Question, RetrieveHit, RetrieveRequest, RetrieveResponse};
+    use super::{Answer, Question, RetrieveHit, RetrieveRequest, RetrieveResponse};
+    use crate::rag::Fonte;
 
     #[test]
     fn retrieve_request_sem_kind_e_top_k_desserializa_com_none() {
@@ -166,4 +171,38 @@ mod tests {
         assert_eq!(q.query_type, None);
         assert_eq!(q.entity, None);
     }
+
+    /// O contrato de fio da D-SF-10: sem fontes, a chave não sai (cliente antigo e os tipos 2–4
+    /// não veem diferença); sem veredito, a chave `triagem` também não.
+    #[test]
+    fn fontes_e_triagem_so_aparecem_quando_existem() {
+        let mut a = Answer {
+            question: "q".into(),
+            answer: "r".into(),
+            log_id: None,
+            fontes: vec![],
+        };
+        let json = serde_json::to_value(&a).unwrap();
+        assert!(json.get("fontes").is_none(), "{json}");
+
+        a.fontes = vec![
+            Fonte {
+                rotulo: "Serviço 1".into(),
+                titulo: "Emitir guia".into(),
+                link: "https://s/1".into(),
+                triagem: None,
+            },
+            Fonte {
+                rotulo: "FAQ 1".into(),
+                titulo: "Como pagar?".into(),
+                link: "https://f/1".into(),
+                triagem: Some("descarta".into()),
+            },
+        ];
+        let json = serde_json::to_value(&a).unwrap();
+        let f = json["fontes"].as_array().unwrap();
+        assert_eq!(f[0]["rotulo"], "Serviço 1");
+        assert!(f[0].get("triagem").is_none(), "{json}");
+        assert_eq!(f[1]["triagem"], "descarta");
+    }
 }
diff --git a/auli-server/crates/auli-cli/src/api/handlers/question.rs b/auli-server/crates/auli-cli/src/api/handlers/question.rs
index a515ac7..7c11e60 100644
--- a/auli-server/crates/auli-cli/src/api/handlers/question.rs
+++ b/auli-server/crates/auli-cli/src/api/handlers/question.rs
@@ -5,7 +5,7 @@ use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
 use tracing::debug;
 
 use crate::api::dto::{Answer, Question};
-use crate::rag::{QueryType, exec_all_question};
+use crate::rag::{QueryType, RespostaChat, exec_all_question};
 use crate::state::AppState;
 
 pub async fn question_handler(
@@ -22,7 +22,7 @@ pub async fn question_handler(
     // onde o `mapping` fica no escopo da requisição. O handler devolve a pergunta ORIGINAL ao front.
     // `log_id` só existe no caminho feliz: quando `exec_all_question` devolve `Err`, a mensagem de
     // erro VIRA a resposta (contrato antigo desta rota) e não há registro a oferecer.
-    let (answer, log_id) = exec_all_question(
+    let resposta = exec_all_question(
         state.engine.clone(),
         state.anonimizador.clone(),
         question.clone(),
@@ -30,7 +30,7 @@ pub async fn question_handler(
         query_type,
     )
     .await
-    .unwrap_or_else(|e| (e.to_string(), None));
+    .unwrap_or_else(|e| RespostaChat::so_texto(e.to_string()));
 
     debug!("Consulta concluída em {} ms", started.elapsed().as_millis());
 
@@ -38,8 +38,9 @@ pub async fn question_handler(
         StatusCode::OK,
         Json(Answer {
             question,
-            answer,
-            log_id: log_id.map(|id| id.to_string()),
+            answer: resposta.answer,
+            log_id: resposta.log_id.map(|id| id.to_string()),
+            fontes: resposta.fontes,
         }),
     )
 }
diff --git a/auli-server/crates/auli-cli/src/rag.rs b/auli-server/crates/auli-cli/src/rag.rs
index d3d0887..fe9e33e 100644
--- a/auli-server/crates/auli-cli/src/rag.rs
+++ b/auli-server/crates/auli-cli/src/rag.rs
@@ -13,6 +13,7 @@ use auli_anon::{Anonimizador, TEXTO_FALLBACK_ERRO};
 use auli_contract::{DocumentoPack, Kind, bloco, conteudo_indisponivel};
 use auli_core::corpus::{self, FAQS, SERVICES};
 use auli_retrieval::{Engine, Hit};
+use serde::{Deserialize, Serialize};
 use tracing::{debug, error, info, trace, warn};
 use uuid::Uuid;
 
@@ -378,13 +379,79 @@ pub(crate) fn blocos(hits: &[Hit], docs_root: &Path, entity_id: &str) -> Vec<Str
         .collect()
 }
 
+/// Um documento recuperado, como a **lista de fontes** da resposta o mostra (D-SF-10).
+///
+/// Existe para que o analista veja SEMPRE o que a busca trouxe, independentemente do que o texto do
+/// LLM decidiu citar: o núcleo do Auli é achar a informação, e o texto gerado é complemento. O
+/// `rotulo` é exatamente o da citação (`"Serviço 2"`, `"FAQ 4"` — D-SF-7), para que uma bata com a
+/// outra.
+#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
+pub struct Fonte {
+    pub rotulo: String,
+    pub titulo: String,
+    pub link: String,
+    /// Veredito da triagem (`aplica` / `condicional` / `descarta` — D-SF-9). Ausente quando não
+    /// houve triagem, quando ela falhou, ou quando o modelo omitiu este documento: nos três casos o
+    /// documento foi à resposta sem filtro, e não há veredito a mostrar.
+    #[serde(default, skip_serializing_if = "Option::is_none")]
+    pub triagem: Option<String>,
+}
+
+/// O que o chat devolve: o texto, a identidade do log e a lista de fontes.
+#[derive(Debug, Default)]
+pub struct RespostaChat {
+    pub answer: String,
+    pub log_id: Option<Uuid>,
+    /// Vazia fora do tipo serviços+FAQs — a lista é só dele por decisão (D-SF-10).
+    pub fontes: Vec<Fonte>,
+}
+
+impl RespostaChat {
+    /// Os atalhos que saem antes de qualquer gravação: só o texto, sem log e sem fontes.
+    pub fn so_texto(answer: String) -> Self {
+        RespostaChat {
+            answer,
+            ..Default::default()
+        }
+    }
+}
+
+/// As fontes de serviços+FAQs, na ordem e com os rótulos do contexto ([`rotulos_servicos_faqs`]).
+///
+/// Payload que não desserializa (não deveria passar pelo boot) ainda gera a linha, com título de
+/// aviso e sem link: a lista tem que ter um item por documento, ou a numeração deixa de bater com a
+/// citação.
+pub(crate) fn fontes_servicos_faqs(svc_payloads: &[&str], faq_payloads: &[&str]) -> Vec<Fonte> {
+    let rotulos = rotulos_servicos_faqs(svc_payloads.len(), faq_payloads.len());
+    rotulos
+        .into_iter()
+        .zip(svc_payloads.iter().chain(faq_payloads.iter()))
+        .map(
+            |(rotulo, payload)| match serde_json::from_str::<DocumentoPack>(payload) {
+                Ok(p) => Fonte {
+                    rotulo,
+                    titulo: p.titulo,
+                    link: p.link,
+                    triagem: None,
+                },
+                Err(_) => Fonte {
+                    rotulo,
+                    titulo: "[documento ilegível]".to_string(),
+                    link: String::new(),
+                    triagem: None,
+                },
+            },
+        )
+        .collect()
+}
+
 pub async fn exec_all_question(
     engine: Arc<Engine>,
     anonimizador: Arc<Anonimizador>,
     question: String,
     entity: Option<String>,
     query_type: QueryType,
-) -> Result<(String, Option<Uuid>)> {
+) -> Result<RespostaChat> {
     debug!("Executando consulta: {}", question);
 
     let t_total = Instant::now();
@@ -415,7 +482,7 @@ pub async fn exec_all_question(
             warn!("{}", e);
             // Sem `log_id`: este atalho sai antes de qualquer gravação. O ícone do log não aparece
             // porque de fato não há registro — mesma regra dos outros retornos precoces.
-            return Ok((e, None));
+            return Ok(RespostaChat::so_texto(e));
         }
     };
     info!("Entidade: {} ({})", cfg.id, cfg.name);
@@ -440,6 +507,8 @@ pub async fn exec_all_question(
     // Os blocos de serviços e de FAQs em separado, guardados para a TRIAGEM (D-SF-9): ela precisa
     // remontar o contexto sem os descartados, mantendo a numeração original. `None` nos outros tipos.
     let mut blocos_sf: Option<(Vec<String>, Vec<String>)> = None;
+    // A lista de fontes da resposta (D-SF-10): só serviços+faqs a preenchem.
+    let mut fontes: Vec<Fonte> = Vec::new();
     let rag = match query_type {
         QueryType::ServicosFaqs => {
             // Retrieve this entity's servicos + faqs concurrently, both through the engine.
@@ -476,6 +545,16 @@ pub async fn exec_all_question(
             let faq_docs = blocos(&faq_hits, engine.docs_root(), &cfg.id);
             let rag = montar_rag_servicos_faqs(&svc_docs, &faq_docs);
             blocos_sf = Some((svc_docs, faq_docs));
+            fontes = fontes_servicos_faqs(
+                &svc_hits
+                    .iter()
+                    .map(|h| h.payload.as_str())
+                    .collect::<Vec<_>>(),
+                &faq_hits
+                    .iter()
+                    .map(|h| h.payload.as_str())
+                    .collect::<Vec<_>>(),
+            );
             rag
         }
         QueryType::ColecaoUnica(kind) => {
@@ -497,13 +576,10 @@ pub async fn exec_all_question(
             // contexto vazio (que é convite a alucinação).
             if hits.is_empty() {
                 // Idem: sai antes do `log_question`, logo sem `log_id`.
-                return Ok((
-                    format!(
-                        "A consulta de {} ainda não está disponível para esta entidade.",
-                        kind.titulo()
-                    ),
-                    None,
-                ));
+                return Ok(RespostaChat::so_texto(format!(
+                    "A consulta de {} ainda não está disponível para esta entidade.",
+                    kind.titulo()
+                )));
             }
 
             // Expansão por grafo: pareceres que citam os MESMOS dispositivos dos recuperados — sinal
@@ -598,14 +674,19 @@ pub async fn exec_all_question(
         let (texto, resultado) =
             match llm::triagem(&compor_system_prompt(prompt_triagem, &rag), entrada_llm).await {
                 Ok(txt) => {
-                    let r = triagem::descartados(&txt, &rotulos);
+                    let r = triagem::vereditos(&txt, &rotulos);
                     (txt, r)
                 }
                 Err(e) => (String::new(), Err(format!("a chamada falhou: {e}"))),
             };
         let ms = t_tri.elapsed().as_millis();
         registro_triagem = Some(match resultado {
-            Ok(fora) => {
+            Ok(vereditos) => {
+                // O veredito vai para a lista de fontes, pelo mesmo rótulo do contexto.
+                for f in &mut fontes {
+                    f.triagem = vereditos.get(&f.rotulo).map(|v| v.as_str().to_string());
+                }
+                let fora = triagem::descartados(&vereditos);
                 info!(
                     "Triagem: {} de {} documentos descartados",
                     fora.len(),
@@ -698,7 +779,11 @@ pub async fn exec_all_question(
     .inspect_err(|e| warn!("falha ao gravar o log da consulta: {e}"))
     .ok();
 
-    Ok((answer, log_id))
+    Ok(RespostaChat {
+        answer,
+        log_id,
+        fontes,
+    })
 }
 
 /// Uma consulta a registrar no log de auditoria — tudo que entra no arquivo, menos o carimbo de
@@ -917,9 +1002,9 @@ mod tests {
     use super::{
         FAQ_BAND, FAQ_FLOOR, LEG_BAND, LEG_FLOOR, MARCADOR_CONTEXTO, PAR_BAND, PAR_FLOOR,
         QueryType, RegistroConsulta, SVC_BAND, SVC_FLOOR, TemposConsulta, Uuid, aderencia, bandas,
-        bloco_documento, compor_system_prompt, envolver, format_log_record, log_question,
-        mes_do_log, montar_aderencia, montar_rag_colecao_unica, montar_rag_servicos_faqs,
-        montar_rag_servicos_faqs_sem, rotulos_servicos_faqs,
+        bloco_documento, compor_system_prompt, envolver, fontes_servicos_faqs, format_log_record,
+        log_question, mes_do_log, montar_aderencia, montar_rag_colecao_unica,
+        montar_rag_servicos_faqs, montar_rag_servicos_faqs_sem, rotulos_servicos_faqs,
     };
     use auli_contract::{DocumentoPack, Kind, mddoc};
     use std::path::Path;
@@ -1272,6 +1357,33 @@ mod tests {
         assert!(rec.contains("descartados: 1 de 2 (FAQ 1)"), "{rec}");
     }
 
+    #[test]
+    fn fontes_tem_um_item_por_documento_com_o_rotulo_da_citacao() {
+        let pack = |titulo: &str, link: &str| {
+            serde_json::to_string(&DocumentoPack {
+                kind: Kind::Servicos,
+                trilha: String::new(),
+                titulo: titulo.into(),
+                ementa: String::new(),
+                resumo: String::new(),
+                link: link.into(),
+                doc_path: "docs/servicos/x.md".into(),
+            })
+            .unwrap()
+        };
+        let s1 = pack("Emitir guia", "https://s/1");
+        let f1 = pack("Como pagar?", "https://f/1");
+        let fontes = fontes_servicos_faqs(&[s1.as_str(), "{quebrado"], &[f1.as_str()]);
+        let rotulos: Vec<_> = fontes.iter().map(|f| f.rotulo.as_str()).collect();
+        assert_eq!(rotulos, ["Serviço 1", "Serviço 2", "FAQ 1"]);
+        assert_eq!(fontes[0].titulo, "Emitir guia");
+        assert_eq!(fontes[0].link, "https://s/1");
+        // Payload ilegível não some da lista: a numeração tem que continuar batendo.
+        assert_eq!(fontes[1].link, "");
+        assert_eq!(fontes[2].titulo, "Como pagar?");
+        assert!(fontes.iter().all(|f| f.triagem.is_none()));
+    }
+
     #[test]
     fn rotulos_seguem_a_numeracao_por_colecao() {
         assert_eq!(
diff --git a/auli-server/crates/auli-cli/src/triagem.rs b/auli-server/crates/auli-cli/src/triagem.rs
index 07fd326..b47b713 100644
--- a/auli-server/crates/auli-cli/src/triagem.rs
+++ b/auli-server/crates/auli-cli/src/triagem.rs
@@ -15,7 +15,7 @@
 //! Este módulo é PURO (sem rede, sem I/O): interpreta o texto que o modelo devolveu. A chamada vive
 //! no `rag.rs`, junto da outra.
 
-use std::collections::BTreeSet;
+use std::collections::{BTreeMap, BTreeSet};
 
 use serde::Deserialize;
 
@@ -33,6 +33,17 @@ pub(crate) enum Veredito {
     Descarta,
 }
 
+impl Veredito {
+    /// A grafia de fio — a mesma do JSON que o modelo devolve e da lista de fontes (D-SF-10).
+    pub(crate) fn as_str(self) -> &'static str {
+        match self {
+            Veredito::Aplica => "aplica",
+            Veredito::Condicional => "condicional",
+            Veredito::Descarta => "descarta",
+        }
+    }
+}
+
 #[derive(Debug, Deserialize)]
 struct Item {
     documento: String,
@@ -49,25 +60,37 @@ struct Resposta {
     documentos: Vec<Item>,
 }
 
-/// Interpreta a resposta da triagem e devolve os rótulos **descartados** (`"Serviço 3"`, `"FAQ 7"`).
+/// Interpreta a resposta da triagem: o veredito de cada documento, pelo rótulo canônico
+/// (`"Serviço 3"`, `"FAQ 7"`). Documento que o modelo omitiu simplesmente não está no mapa.
 ///
 /// `rotulos` são os que existem no contexto enviado. Um rótulo desconhecido na resposta é `Err`:
 /// o modelo está falando de um documento que não viu, e nada do que ele disse sobre os outros
 /// merece confiança.
-pub(crate) fn descartados(resposta: &str, rotulos: &[String]) -> Result<BTreeSet<String>, String> {
+pub(crate) fn vereditos(
+    resposta: &str,
+    rotulos: &[String],
+) -> Result<BTreeMap<String, Veredito>, String> {
     let json = extrair_objeto(resposta).ok_or("a resposta não contém um objeto JSON")?;
     let r: Resposta = serde_json::from_str(json).map_err(|e| format!("JSON inválido: {e}"))?;
-    let mut fora = BTreeSet::new();
+    let mut mapa = BTreeMap::new();
     for item in r.documentos {
         let rotulo = normalizar(&item.documento);
         if !rotulos.contains(&rotulo) {
             return Err(format!("rótulo fora do contexto: '{}'", item.documento));
         }
-        if item.veredito == Veredito::Descarta {
-            fora.insert(rotulo);
-        }
+        mapa.insert(rotulo, item.veredito);
     }
-    Ok(fora)
+    Ok(mapa)
+}
+
+/// Os rótulos **descartados** — os únicos que saem do contexto da segunda chamada. Omitido e
+/// `condicional` ficam (fail-open por documento).
+pub(crate) fn descartados(vereditos: &BTreeMap<String, Veredito>) -> BTreeSet<String> {
+    vereditos
+        .iter()
+        .filter(|(_, v)| **v == Veredito::Descarta)
+        .map(|(r, _)| r.clone())
+        .collect()
 }
 
 /// O trecho entre a primeira `{` e a última `}`. Cobre a cerca markdown (```` ```json ````) e o
@@ -102,6 +125,11 @@ fn normalizar(s: &str) -> String {
 mod tests {
     use super::*;
 
+    /// O caminho do `rag.rs`: vereditos e, deles, os descartados.
+    fn fora(r: &str) -> Result<BTreeSet<String>, String> {
+        vereditos(r, &rotulos()).map(|m| descartados(&m))
+    }
+
     fn rotulos() -> Vec<String> {
         ["Serviço 1", "Serviço 2", "FAQ 1", "FAQ 2"]
             .iter()
@@ -116,7 +144,7 @@ mod tests {
             {"documento": "Serviço 2", "veredito": "condicional", "motivo": "regime"},
             {"documento": "FAQ 1", "veredito": "descarta", "motivo": "outro tema"}
         ]}"#;
-        let fora = descartados(r, &rotulos()).unwrap();
+        let fora = fora(r).unwrap();
         // A FAQ 2 foi omitida pelo modelo: fica (fail-open por documento).
         assert_eq!(fora, BTreeSet::from(["FAQ 1".to_string()]));
     }
@@ -124,14 +152,14 @@ mod tests {
     #[test]
     fn cerca_markdown_e_texto_em_volta_sao_tolerados() {
         let r = "Segue:\n```json\n{\"documentos\": [{\"documento\": \"FAQ 2\", \"veredito\": \"descarta\"}]}\n```";
-        let fora = descartados(r, &rotulos()).unwrap();
+        let fora = fora(r).unwrap();
         assert_eq!(fora, BTreeSet::from(["FAQ 2".to_string()]));
     }
 
     #[test]
     fn rotulo_com_colchete_e_caixa_diferente_e_normalizado() {
         let r = r#"{"documentos": [{"documento": "[servico 2]", "veredito": "descarta"}]}"#;
-        let fora = descartados(r, &rotulos()).unwrap();
+        let fora = fora(r).unwrap();
         assert_eq!(fora, BTreeSet::from(["Serviço 2".to_string()]));
     }
 
@@ -141,13 +169,27 @@ mod tests {
             {"documento": "FAQ 1", "veredito": "descarta"},
             {"documento": "Serviço 9", "veredito": "aplica"}
         ]}"#;
-        assert!(descartados(r, &rotulos()).is_err());
+        assert!(fora(r).is_err());
     }
 
     #[test]
     fn veredito_desconhecido_e_resposta_sem_json_sao_erro() {
         let r = r#"{"documentos": [{"documento": "FAQ 1", "veredito": "talvez"}]}"#;
-        assert!(descartados(r, &rotulos()).is_err());
-        assert!(descartados("não sei", &rotulos()).is_err());
+        assert!(fora(r).is_err());
+        assert!(fora("não sei").is_err());
+    }
+
+    #[test]
+    fn vereditos_trazem_os_tres_valores_e_omitido_fica_de_fora_do_mapa() {
+        let r = r#"{"documentos": [
+            {"documento": "Serviço 1", "veredito": "aplica"},
+            {"documento": "Serviço 2", "veredito": "condicional"},
+            {"documento": "FAQ 1", "veredito": "descarta"}
+        ]}"#;
+        let m = vereditos(r, &rotulos()).unwrap();
+        assert_eq!(m.get("Serviço 1").map(|v| v.as_str()), Some("aplica"));
+        assert_eq!(m.get("Serviço 2").map(|v| v.as_str()), Some("condicional"));
+        assert_eq!(m.get("FAQ 1").map(|v| v.as_str()), Some("descarta"));
+        assert!(!m.contains_key("FAQ 2"));
     }
 }
diff --git a/docs/auli_code.md b/docs/auli_code.md
index 90ac5bb..6bee459 100644
--- a/docs/auli_code.md
+++ b/docs/auli_code.md
@@ -241,6 +241,7 @@ pedido.
 | **D-SF-7** | O `rs.txt` exige **citação por afirmação** no formato do marcador do contexto — `[Serviço 2]`, `[FAQ 4]` —, e afirmação sem documento para citar não se escreve | Amarra cada frase à fonte, o que dificulta a fusão entre documentos e torna a conferência mecânica (o rótulo citado existe no contexto? a frase está nele?). O tipo vai junto do número porque a numeração é **por coleção** (D-MARC-3): "[doc 3]" seria ambíguo entre o Serviço 3 e a FAQ 3. A lista de fontes da resposta usa os mesmos rótulos, para o analista bater uma com a outra. |
 | **D-SF-8** | Marcador `{{CONTEXTO}}` no prompt de sistema: o que vem antes dele fica antes dos documentos, o que vem depois vai **depois** do contexto e do `'''` (`rag::compor_system_prompt`). O `rs.txt` usa o espaço para repetir a regra central | Com 15 documentos entre as regras e a pergunta, a regra que mais importa ficava longe do ponto em que o modelo redige. A decisão de O QUE repetir é do prompt (catálogo), não do código. Prompt **sem** marcador sai byte a byte como antes — é o caso dos outros 26 estados, e há teste que trava isso. Divide na primeira ocorrência; o marcador nunca chega ao modelo. |
 | **D-SF-9** | **Triagem em duas chamadas** (`triagem.rs` + `rag.rs`), opt-in por entidade pelo campo `prompt_triagem` do registry (só o `rs`, com `config/prompts/rs-triagem.txt`). A 1ª chamada recebe a pergunta e os documentos e devolve JSON com um veredito por rótulo — `aplica`, `condicional` ou `descarta`; a 2ª redige vendo o contexto **sem os descartados**, com a **numeração original** preservada (`montar_rag_servicos_faqs_sem`). Tudo descartado ⇒ resposta fixa, sem 2ª chamada. Triagem que falha (rede, teto de tempo, JSON inválido, rótulo inexistente) ⇒ segue com o contexto **integral** (fail-open); documento omitido no JSON **fica**. Teto de **20 s para o passo inteiro** (`tokio::time::timeout` por fora, porque o `auli-llm` repete até 3 vezes em timeout); o frontend sobe de 35 s para **70 s** | É a forma explícita da D-SF-1: o que não trata da situação perguntada nem deveria estar na mesa quando o texto é escrito. Escolhida em vez da classificação dentro do raciocínio de uma chamada só, **ciente de que dobra a latência**. `condicional` fica no contexto porque é justamente o irmão que a resposta deve apresentar como alternativa — e, na dúvida, o prompt manda escolher `condicional`, não `descarta`. A numeração original é o que mantém citação (D-SF-7), ADERÊNCIA do log e lista de fontes falando dos mesmos rótulos. Fail-open porque a triagem é filtro de qualidade, não guarda: perdê-la volta à resposta de uma chamada só, nunca derruba a consulta. O log ganha a seção **TRIAGEM** (tempo, descartados e a saída crua do modelo, incluindo o `motivo` de cada veredito), e o `CONTEXTO RAG` continua sendo o **recuperado inteiro** — o que a triagem viu —, o que preserva o sentido do `parity-replay.py`. O MCP não tem triagem: entrega o contexto integral e o assistente externo faz a dele. |
+| **D-SF-10** | **Lista fixa de fontes** na resposta de Serviços + FAQs — e só nela. O `POST /v1/question` ganha o campo `fontes` (`rotulo`, `titulo`, `link`, `triagem`), montado por `rag::fontes_servicos_faqs` a partir dos payloads, e o chat a mostra num `<details>` **recolhido** (`Fontes.tsx`): "Fontes consultadas: 15 (3 descartadas na triagem)" | O analista vê sempre o que a busca trouxe, independentemente do que o texto do LLM decidiu citar — o núcleo do Auli é achar a informação, e o texto é complemento. O `rotulo` é exatamente o da citação (D-SF-7), na ordem do contexto. Um item por documento, mesmo com payload ilegível (título de aviso, sem link), senão a numeração deixa de bater. O veredito vem da triagem pelo mesmo rótulo; ausente quando não houve triagem, quando ela falhou, ou quando o modelo omitiu o documento. Descartada aparece esmaecida e riscada, não some: o descarte errado tem que ser visível. Recolhida porque, aberta, empurraria 15 linhas abaixo de cada resposta no celular. Só neste tipo **por decisão**: os demais seguem sem lista. Campo vazio não serializa — cliente antigo e tipos 2–4 não veem diferença (há teste do contrato de fio). `exec_all_question` passa a devolver `RespostaChat`. |
 
 ### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)
 
@@ -662,7 +663,10 @@ A versão exibida e um `__BUILD_ID__` para cache-busting são injetados em build
   `entityId: entity.id` ao backend.
 - [pages/chat/utils/callServerAPI.ts](auli-frontend/src/pages/chat/utils/callServerAPI.ts):
   `POST` via axios do corpo `{ question, entity? }`, com **timeout de 70 s** via
-  `AbortController`; mensagens de erro/timeout em pt-BR. Lê `res.data.answer`.
+  `AbortController`; mensagens de erro/timeout em pt-BR. Lê `res.data.answer`, `log_id` e
+  `fontes` (D-SF-10).
+- [pages/chat/Fontes.tsx](auli-frontend/src/pages/chat/Fontes.tsx): a lista fixa de fontes de
+  Serviços + FAQs, recolhida num `<details>`, abaixo da resposta e acima do aviso (§3.4.1, D-SF-10).
 - [pages/chat/utils/prompt.ts](auli-frontend/src/pages/chat/utils/prompt.ts): validação do
   prompt (mínimo de 10 caracteres) — fonte única usada pelo guard de envio, botão e contador.
 - [pages/chat/utils/useMessages.ts](auli-frontend/src/pages/chat/utils/useMessages.ts):
diff --git a/docs/auli_features.md b/docs/auli_features.md
index b1482eb..ce7d635 100644
--- a/docs/auli_features.md
+++ b/docs/auli_features.md
@@ -83,6 +83,8 @@ Os conteúdos oficiais são previamente **coletados do portal** da secretaria (s
   trocada pelo cabeçalho.
 - **Chat conversacional** com balões diferenciados para usuário e assistente, mensagem de
   "pensando", **botão de copiar** mensagens e **timeout amigável** (70 s) quando a API demora.
+  Nas consultas de Serviços + FAQs, a resposta traz a **lista das fontes consultadas** —
+  recolhida, com os mesmos rótulos da citação no texto (`[Serviço 2]`) e o veredito da triagem.
 - **Navegação por abas** (sem recarga de página), em três grupos: **Acervo** (Serviços, FAQs,
   Pareceres, Acórdãos TARF, Notas, Conteúdos), **Integrações** (Conectar sua IA, Downloads) e o
   Chat, mais o Sobre no rodapé — com o estado de cada aba preservado ao alternar. Aba de coleção que
-- 
2.43.0

~~~~~
