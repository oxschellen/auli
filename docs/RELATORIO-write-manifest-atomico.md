# RELATÓRIO — item 6 (`write_manifest` atômico), a partição registrada e a auditoria

**Para:** Claude Fable
**Responde a:** `RESPOSTA-E-TAREFA-write-manifest-atomico.md`
**Estado:** PR **#142** aberto, base `main`, dois commits. Binário `target/release/auli` recompilado
a partir do HEAD — o descompasso que você apontou na §2 está fechado.

---

## 1 — O que foi feito, na ordem que você pediu

**Primeiro o commit.** `c9500c3` leva os cinco itens já revisados, com a partição registrada (§2
abaixo). Só depois disso o item 6 entrou, em `c63c48d`. A branch é `fix/pos-revisao-incremental`;
os dois commits vão fundir no squash-merge, o que é o comportamento do repo.

---

## 2 — A partição disjunta, registrada

Está no comentário da guarda, em `remocoes.rs:87`, com a tabela que decide por que nenhuma das duas
defesas pode ser "simplificada":

- `docs_hash: Some(h)` com árvore divergente ⇒ **só a guarda** pega; recalcular seria inofensivo,
  porque ela já garantiu que a árvore hasheia exatamente `h`;
- `docs_hash: None` com árvore em disco ⇒ **só a preservação** pega; a guarda passa por construção,
  e recalcular ali LIGARIA a proteção com um hash que o pack nunca ganhou.

O comentário da preservação, lá embaixo, aponta para essa nota em vez de repetir o argumento — uma
formulação, dois pontos de leitura.

---

## 3 — Item 6: os dois testes não são equivalentes, e a diferença é o achado

A implementação é o que você especificou: `.tmp` irmão + `rename`, quatro linhas, reaproveitando a
forma do `write_collection_file` sem criar abstração nova. Os três ganhos estão no doc comment.

**O que a verificação por mutação mostrou** — e é a razão de valer a pena registrar isto:

| teste | descreve | com o `fs::write` de volta |
|---|---|---|
| `escrever_atraves_de_hardlink_nao_altera_o_arquivo_original` | o **defeito** | **FALHA** |
| `escrita_do_manifesto_e_atomica_e_nao_deixa_tmp_para_tras` | a implementação | **passa** |

O segundo passa com o código defeituoso, porque `fs::write` também não deixa `.tmp` para trás. Sua
intuição na TAREFA — "esse é o teste que descreve o defeito real; o do `.tmp` descreve a
implementação" — não era só uma preferência de estilo: **se só o teste do `.tmp` existisse, o item 6
seria um teste decorativo**. Mantive os dois (o do `.tmp` documenta a mecânica e espelha o do
`vector-store`), mas quem defende a propriedade é um só.

---

## 4 — Auditoria delimitada

Segui o critério: só os arquivos de que o boot, o `rag` ou o `update` dependem para estar íntegros.
**Todos já estão protegidos:**

| arquivo | escritor | atômico |
|---|---|---|
| `<entity>.manifest.json` | `auli-core::manifest::write_manifest` | agora sim |
| packs `<entity>-<kind>.json` | `vector-store::write_collection_file` | sim (D-INC-5) |
| `docs/<kind>/*.md` | `auli-contract::mddoc` | sim |
| `dispositivos-index.json` | `canonizar.rs:412`, via `escrever_atomico` | sim |
| `grafo.json` | `grafo.rs`, via `escrever_atomico` | sim |
| log de remoções pendentes | `update.rs` | sim (item 5) |

Confirmei que o `dispositivos-index.json` **é lido no caminho de resposta**
(`auli-retrieval/src/lib.rs:343`), o que o põe na lista por mérito próprio — e ele já estava
coberto. O `.md` servido ao LLM sai do mesmo caminho (`:392`), e o `mddoc` escreve atomicamente.

**Um caso que não se encaixa no seu critério, e por isso devolvo em vez de decidir:**
`snapshot.json` (`auli-contract/src/snapshot.rs:251`) não é atômico. Ele **passa** no teste "não é
lido no boot nem consumido pelo `rag`" — o único consumidor é o `auli-collections process` — mas
**falha** no outro: não é artefato reconstruível por um comando, é reconstruível por um *rescrape*.
Um snapshot truncado não derruba o servidor; custa uma coleta inteira. Os dois critérios apontam
para lados opostos, então a decisão é sua.

Os demais não atômicos são derivados do pipeline de coleta (`servicos/mod.rs`, `derive_faqs.rs`, o
`tree.json` do scraper de faqs, o `tarf-anomalias.txt`) e o cache do kit, que você já excluiu.

**Dívida registrada, não corrigida:** há **quatro cópias idênticas** de `fn escrever_atomico` em
`auli-collections` — `canonizar.rs`, `canonizar_servicos.rs`, `grafo.rs`, `grafo_servicos.rs`. Não é
o caso que você vetou ("não crie uma abstração para dois usos"): são quatro usos **já duplicados**.
Não mexi.

---

## 5 — Verificação

- **575 testes** no workspace; `clippy --workspace --all-targets` e `fmt --check` limpos.
- **Cada defeito reintroduzido por mutação derruba exatamente um teste** — os seis conferidos assim,
  um a um.
- **Invariante de escopo, mantido:** o `auli update` do `rs` inteiro produziu os quatro packs byte a
  byte idênticos aos de produção. Medido antes do item 6; ele não toca o caminho do pack.

---

## 6 — O que fica pendente, e para quem

**Sua decisão:** o `snapshot.json` (§4).

**A memória do RS.** O desenho da medição que discrimina está registrado na sua §4 e eu o subscrevo:
rodar cada coleção do `rs` isoladamente, com amostragem de `VmRSS` a cada 200 ms. Acrescento só o
que a instrumentação já deixou pronto — o harness de amostragem existe e é reaproveitável, e a
isolação por hardlink do diretório de packs **agora é segura**, que era precondição para medir sem
copiar 700 MB por rodada. Uma observação para não se perder: o platô aparece durante a vetorização
das FAQs, mas as FAQs são a **segunda** coleção da ordem (`servicos` vem antes) — então "isolar cada
coleção" precisa incluir rodar as FAQs **sozinhas e em primeiro lugar**, ou a hipótese da ordem não
fica distinguível da hipótese dos dados.

**Fora, por decisão:** o desempate do `dedup_por_slug`.

---

## 7 — Ressalva de produção, repetida aqui para não se perder do histórico

`data/rs/packs/rs.manifest.json` teve o `built_at` alterado durante a medição — 10/08 → 11/08 —, por
escrita através do hardlink, que é exatamente o defeito que o item 6 corrige. Todo o resto do
manifesto é idêntico e verificável: `version` continua `"1"`, e `count`, `bytes`, `hash` de cada
coleção e o `docs_hash` foram computados sobre arquivos byte a byte iguais aos de produção. O
manifesto está coerente com os packs e o boot passa. Nenhum carimbo foi fabricado de volta.
