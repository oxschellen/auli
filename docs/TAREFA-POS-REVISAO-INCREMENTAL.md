# TAREFA — correções da revisão da TAREFA-UPDATE-INCREMENTAL

**Contexto:** as Fases 1, 3, 4 e 5 estão concluídas e em produção (commits `0dd34c3` #139,
`c691000` #140, `ee7dc67` #141), com os 27 packs regenerados em formato 2 e 48.408 vetores conferidos
um a um contra o backup, zero divergências. Esta TAREFA é a revisão do que foi entregue.

**O trabalho está bom** — a D-INC-14 saiu melhor implementada do que especificada, e o defeito do log
que o `remover` regenerava, encontrado pelo teste de ponta a ponta, é do tipo que só aparece quando
alguém escreve o teste certo. Nada aqui é reescrita: são **um defeito**, **uma lacuna estrutural**,
**uma dívida de memória** e **quatro pequenas**.

**Ordem obrigatória:** o item 1 primeiro, sozinho, com portão. Ele é o único que corrige
comportamento errado hoje. Os demais podem vir juntos.

---

## 1 — DEFEITO: `auli remover` apaga a proteção do `docs_hash`

Em `crates/auli-cli/src/remocoes.rs`, no fim do `run_remover`:

```rust
// O `docs_hash` cobre a árvore, que não mudou aqui; recalculá-lo é barato e evita que este
// caminho seja o único que deixa o manifesto pela metade.
manifesto.docs_hash = manifest::hash_docs_tree(&docs_dir)?;
```

A premissa do comentário é verdadeira sobre o que o `remover` faz e **falsa sobre o mundo**: a árvore
pode ter mudado desde o último `auli update`. O que acontece então:

1. `auli update` roda → pack em sincronia, `docs_hash = H1`;
2. o scraper traz um acórdão novo → a árvore passa a hashear `H2`, e o pack **não tem** esse
   documento. O boot **recusaria** (`validate_docs_hash`, `H1 ≠ H2`) — a proteção funcionando;
3. alguém roda `auli remover` (havia log aprovado) → as remoções são aplicadas e o manifesto passa a
   carimbar **`H2`**;
4. o boot agora **passa**, e o documento novo está ausente do índice **em silêncio**.

É exatamente a classe de staleness que o `docs_hash` existe para impedir. E o próprio arquivo já
argumenta contra isso, duas dúzias de linhas acima, no doc comment do `ids_na_arvore`: *"decidir
remoções a partir de uma árvore que o pack ainda não viu é decidir sobre o diff errado"*. O recálculo
do `docs_hash` contradiz esse raciocínio.

**Correção:** recusar antes de escrever nada, e nunca reescrever o campo.

- No início do `run_remover`, depois de ler o manifesto e **antes** de qualquer escrita: se
  `manifesto.docs_hash` é `Some(h)` e `hash_docs_tree(&docs_dir)? != Some(h)`, **aborte** com
  mensagem dizendo que a árvore está à frente do pack e que o remédio é `auli update` primeiro.
  Reaproveite `manifest::validate_docs_hash` se a mensagem dele servir — ela já diz o essencial e
  evita uma segunda formulação do mesmo erro.
- **Remova a linha que reescreve `manifesto.docs_hash`.** O `remover` não toca a árvore; preservar o
  valor lido é estritamente correto.

Isto tem um ganho de correção além do `docs_hash`: garante que os órfãos que o humano revisou são os
órfãos do diff que o pack conhece.

**Testes:** (a) com a árvore à frente do manifesto e um log aprovado, o `remover` recusa, **não
escreve pack nem manifesto**, e o log continua no lugar; (b) no caminho normal, o `docs_hash` do
manifesto depois do `remover` é **idêntico** ao de antes (não recalculado).

**Pare aqui e reporte.**

---

## 2 — LACUNA: a equivalência "incremental == do zero" não é testável

`ingest_items` recebe `&Embedder`, tipo concreto. Consequência: **o teste que a TAREFA original
chamava de "o que decide tudo" (§7.1: incremental e reescrita total produzem o mesmo arquivo byte a
byte) não pode existir**, porque exigiria carregar 2 GB de modelo num teste unitário. Foi por isso que
a verificação mais forte de vocês teve de ser operacional — e ela prova outra coisa (que o formato não
mexeu nos vetores), não a equivalência dos dois caminhos.

Hoje a propriedade vale **por construção**: `apply` ordena por `id`, o conjunto de records é o mesmo,
e o vetor do cache é bit-idêntico ao que seria recalculado. "Por construção" é precisamente o que um
teste defende contra regressão futura.

**Correção:** introduza a costura mínima — um trait de um método sobre o que o `ingest_items`
realmente usa do embedder (`embed_dense` e `conta_tokens`), implementado pelo `Embedder` real, com um
fake determinístico nos testes. Nada de abstração especulativa: **só os métodos que o caminho de
ingestão chama**, e o trait vive onde o `ingest_items` está, não em `auli-core`.

**O teste que passa a existir, e é o objetivo deste item:** uma árvore de jurisprudência, um
`ingest_items` de reescrita total, e uma sequência incremental que chega ao mesmo estado lógico —
arquivos **byte a byte idênticos**. Cenários: nenhuma mudança; um documento novo; um documento com
resumo novo; os dois juntos.

**Cuidado com o caso de borda, que é o mais valioso do item:** os arquivos são idênticos **somente
quando não há órfãos**. Com órfão, a reescrita total o apaga e o incremental o preserva por doutrina
(D-INC-9) — e isso é comportamento correto, não divergência. O teste tem de afirmar as duas coisas:
igualdade sem órfão, e divergência **esperada e nomeada** com órfão.

---

## 3 — MEMÓRIA: o pack antigo é desserializado até três vezes por coleção

No caminho incremental de uma coleção, o pack inteiro é lido três vezes: `vetores_reaproveitaveis`,
`orfaos_do_pack` e o `apply`. As duas primeiras **não precisam do payload** — e no TARF o payload é o
`stored_repr` com a fundamentação inteira do acórdão, até 27 mil caracteres por registro. É
exatamente o que fez o RS picar em **13,1 GB** na regeneração (contra 3,1 GB do SP, com número
parecido de documentos).

**Correção:** em `vetores_reaproveitaveis`, leia o pack com um payload que o serde descarta —
`P = serde_json::value::IgnoredAny` — em vez de `String`. A função só usa `(id, key_hash, embedding)`;
o payload é peso morto. Deixe `orfaos_do_pack` como está: ele **precisa** de `titulo` e `link` do
payload.

Não persiga mais que isso aqui: é uma linha de tipo, mede-se no pico de RSS do `rs`, e o resto da
dívida de memória do RS é assunto de outra TAREFA.

---

## 4 — Comentários que ficaram descrevendo o desenho morto

Num repo que trata comentário como contrato, isto é dívida real, não cosmética. Todos referem-se ao
esquema `id-N`, que morreu na Fase 1:

| onde | o que diz | problema |
|---|---|---|
| `update.rs`, doc de `arquivos_md` | "os `id-N` do pack derivam dela" | o id é o `doc_path`; a ordem estável segue importando, mas por outro motivo (a ordem canônica de escrita, D-INC-4) |
| `vector-store/lib.rs`, doc do módulo | "[`Writer`] does `reset`/`upsert`" | é `reset`/`apply` |
| `vector-store/write.rs`, doc de `reset` | "leaves no orphan `id-(N+1)..` records" | não existe mais `id-(N+1)`; o motivo real do `reset` hoje é a política total do D-INC-8 |
| `vector-store/write.rs`, teste | `upsert_rejects_mismatched_dimension` | o método é `apply` |

Ao corrigir o primeiro, diga o motivo **atual** da ordem estável em vez de apenas apagar a menção —
um comentário que perdeu a justificativa convida alguém a remover a ordenação depois.

---

## 5 — Duas notas menores, na mesma leva

**`escrever_log_remocoes` usa `fs::write` direto**, sem o `tmp` + `rename` que a D-INC-5 trouxe para o
pack. O log não é lido no boot, então o risco é baixo — mas é um arquivo que um humano vai editar, e
um truncamento parcial seria confuso justamente no artefato que autoriza apagar documento. Use a
mesma disciplina.

**Documente a atomicidade que não existe entre pack e manifesto no `remover`.** Morrer entre a escrita
do pack e a do manifesto deixa o boot recusando por hash de pack, e o log já foi consumido: a
recuperação é `auli update`, que reconstrói e mantém as remoções (os órfãos não estão na árvore de
qualquer forma). Recuperável e limpo — mas quem topar com o erro não vai adivinhar isso. Um parágrafo
no doc comment do `run_remover`, não código.

---

## O que NÃO fazer

- Não mexa no critério de desempate do `dedup_por_slug` do scraper. A investigação de 10/08 concluiu
  que o empate de `dataDocumento` é sinal de **duplicata ou erro de metadado**, não de revisão, e que
  trocar o critério resolveria um problema que não se manifestou **e mascararia o sinal**. Isso é
  outra TAREFA, com outro desenho (verificação "número do corpo × título da listagem" como anomalia).
- Não "melhore" o `ingest_items` de passagem. O item 2 autoriza **uma** mudança estrutural: a costura
  do embedder. Nada além.
- Não toque nos packs de produção. Nenhum item desta TAREFA muda o formato nem o conteúdo do pack, e
  isso é verificável: depois dela, um `auli update` de uma entidade tem de produzir um pack **byte a
  byte idêntico** ao que está lá. Se não produzir, algo saiu do escopo — pare e diga.

## O que reportar

Na ordem: (a) o item 1, com os dois testes, antes de tudo; (b) se o teste de equivalência do item 2
passou, e se o caso com órfão divergiu como esperado; (c) o pico de RSS do `rs` antes e depois do item
3; (d) qualquer suposição desta TAREFA que não bateu com o código — pare e diga, não contorne.
