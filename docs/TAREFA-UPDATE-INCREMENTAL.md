# TAREFA — `update` incremental: identidade estável, cache de vetor e remoções com portão humano

**Origem:** `docs/ESTUDO-identidade-e-update-incremental.md` + `docs/REGISTRO-restart-incremental.md`
(medições de 10/08/2026). Este documento é a especificação fechada; onde ele divergir do ESTUDO, ele
manda — as decisões abaixo foram tomadas depois dele.

**Objetivo:** o `auli update` deixa de re-embeddar o acervo inteiro. Documentos cuja chave de
embedding não mudou têm o **vetor reaproveitado** do pack anterior. Hoje o TARF re-embeda 22,5 mil
acórdãos para incorporar os poucos do dia.

**Escopo:** `crates/vector-store`, `crates/auli-core/manifest.rs`, `crates/auli-cli/src/update.rs`,
~~e uma ferramenta de migração descartável em `tools/`~~ — a migração foi cancelada (ver Fase 2).
**Nada no servidor muda de comportamento** (além de validar um campo novo do manifesto). O frontend
não é tocado.

---

## Decisões (D-INC-*)

**D-INC-1 — Identidade do registro é o `doc_path`.** Os `id-1..id-N` posicionais morrem. `doc_path`
já é único por construção nas quatro coleções (slug+hash8 em serviços/faqs, slug do número na
jurisprudência), é derivado e nunca digitado. Com identidade posicional, upsert incremental é
impossível: inserir um documento no meio da ordem alfabética desloca a identidade de todos os
seguintes.

**D-INC-2 — `key_hash` ao lado do vetor.** Hash do `text_to_embed` gravado por registro. Igual ⇒
reaproveita o vetor; diferente ou ausente ⇒ re-embeda. Campo **opcional** (`serde(default)` +
`skip_serializing_if`): pack antigo desserializa com `None` e é tratado como "tudo alterado", o que
degrada para o comportamento de hoje sem quebrar.

**D-INC-3 — Só o VETOR é cache; o payload é SEMPRE reconstruído da árvore.** Há campos no payload que
não entram na key (o `link` da jurisprudência é o caso concreto). Se o portal reorganizar URLs sem
mudar conteúdo, o `key_hash` bate — e o payload não pode ficar com o link morto. Reaproveitar o
payload seria uma classe inteira de staleness silenciosa.

**D-INC-4 — Ordenação canônica por `id` na escrita.** Sem isso, rebuild total (ordem alfabética da
árvore) e incremental (ordem de aplicação) produzem arquivos diferentes com conteúdo logicamente
idêntico, e o teste de equivalência do §7.1 do ESTUDO não pode exigir igualdade byte a byte. Como
`id = doc_path` e o `doc_path` É o nome do arquivo, ordenar por `id` **preserva exatamente a ordem de
hoje**.

**D-INC-5 — `apply(upserts, removes)` numa operação, com `tmp` + `rename`.** Hoje são duas escritas
(`reset` + `upsert`) do arquivo inteiro. Uma operação, um read-modify-write, escrita atômica. Com
update mensal, crash no meio do `fs::write` era teórico; com escrita frequente, não é. (O boot
recusaria o pack corrompido pelo hash — falha certa, mas melhor não produzir.)

**D-INC-6 — Índice `HashMap<id, usize>` no lugar do `iter_mut().find`.** O find atual é O(n) por
registro: no rebuild do TARF são ~250 M comparações de string.

**D-INC-7 — `pack_format: u32` no manifesto, separado do `strategy_version`.** A fronteira:
**estratégia** = "o que entra no vetor mudou" ⇒ reembed obrigatório; **formato** = "como o arquivo é
escrito mudou" ⇒ migração, **nunca** reembed. `serde(default = 1)`: manifesto sem o campo **é** o
formato 1 (`id-N`, sem `key_hash`); o novo carimba **2**.

**D-INC-8 — Política por FAMÍLIA de coleção, não por flag.** A política acompanha a doutrina de
árvore que já existe no repo:

| família | árvore | pack |
|---|---|---|
| jurisprudência (`pareceres`, `tarf`) | append-only | **incremental** (diff + cache de vetor) |
| mutáveis (`servicos`, `faqs`) | delete + rebuild | **reset + total, sempre** |

Serviços e faqs são rescrapeados, reconstruídos e re-embeddados **integralmente, sempre**, gerando
pack novo completo: são poucos registros, mudam muito no portal, e detectar com segurança o que foi
retirado de lá é complexo. **Formato único nas quatro coleções, duas políticas.** O lugar da política
é um método em `Kind`, ao lado do `exige_resumo()` — assim entidade nova herda a política certa pela
coleção, sem flag operacional para errar.

**D-INC-9 — Remoções NUNCA automáticas na jurisprudência.** Órfão (id no pack sem `.md` na árvore)
**não é removido**: vira linha num log de remoções pendentes, regenerado do diff a cada rodada
(documento que reaparecer sai do log sozinho). Mesma doutrina do `tarf-anomalias.txt`: a decisão é
humana. Isso **dissolve** a discussão de teto percentual — nada é removido sem aprovação, então não
há X% a calibrar, e o caso degenerado "árvore vazia" vira um log que ninguém aprovaria (além de já
estar guardado: `preparar` devolve `None` para diretório ausente **ou vazio**, e coleção pulada nunca
chega ao diff).

**D-INC-10 — Aprovação por SUBCOMANDO, não por flag.** Uma flag `--aplicar-remocoes` acabaria dentro
do `update-*.sh` "para não rodar duas vezes", e o portão humano desapareceria sem ninguém decidir
removê-lo. Subcomando exige invocação deliberada. Regra de aplicação: aplica exatamente
**`aprovados ∩ órfãos_atuais`**, e relata três classes:

1. **aplicados**;
2. **órfãos atuais não aprovados** — permanecem no log, para a próxima revisão;
3. **aprovados que deixaram de ser órfãos** — o documento reapareceu na árvore; descartados, sem
   remoção.

Não recusa a rodada porque o diff mudou em algo irrelevante (documento novo entrou entre a revisão e
a aplicação); recusa apenas remover o que não foi lido. É **idempotente**. E habilita **aprovação
seletiva por linha**: o humano apaga do log as linhas que não quer autorizar; elas voltam no log da
próxima rodada, porque não aprovar hoje é "não agora", não uma decisão permanente.

**D-INC-11 — O subcomando recarimba o manifesto.** Ele escreve o pack, logo tem de atualizar
`CollectionEntry` (count/bytes/hash) e o `docs_hash` pela **mesma função** que o `update` usa. Sem
isso o boot recusa corretamente e o operador não descobre por quê. Extrair o carimbo do `run_update`
para os dois caminhos compartilharem.

**D-INC-12 — Fase 0 do ESTUDO cancelada como portão.** Ela existia para decidir *se* o desenho valia
("taxa acima de 5% muda o desenho"). As decisões posteriores esvaziaram a pergunta: serviços/faqs
saíram do incremental por doutrina (D-INC-8) e a jurisprudência é append-only com re-edição medida em
0,30%. A medição fica como **subproduto gratuito**: o primeiro update incremental real imprime quantos
vetores reaproveitou.

**D-INC-13 — Migração sem reembed, com guarda de `docs_hash`.** O `doc_path` já está dentro do payload
de cada registro, então migrar é **reescrever o campo `id`** a partir do que já está no arquivo e
carimbar o `key_hash`. Mas o `key_hash` só é honesto se a árvore estiver em sincronia com o pack —
senão carimbamos o hash da árvore nova ao lado do vetor do conteúdo velho, e o cache **nasce
mentindo**. Logo: **validar o `docs_hash` antes de migrar e recusar se divergir.** Guard-rail de uma
linha; se esquecido, o defeito é silencioso e de nascença.

**D-INC-14 — `upsert` recebe `Vec<Record<P>>`, não um quarto slice paralelo (divergência decidida por
Carlos, 10/08/2026).** A versão anterior desta TAREFA dizia apenas que o `Record` ganha `key_hash` e
que a store "só o carrega", o que na prática levaria a passar `key_hashes` como quarto slice ao lado de
`ids`/`embeddings`/`payloads`. Isso **agravaria** o problema que a guarda `ArityMismatch` existe para
cobrir (um `zip` silencioso truncando para o menor). Passando `Vec<Record<P>>`, o descasamento fica
**inexpressável pelo tipo** em vez de detectável em tempo de execução — e é a assinatura para onde a
Fase 3 vai de qualquer modo (`apply(upserts, removes)`). Consequências, todas dentro do escopo da
Fase 1:

- **A guarda muda de lugar, não desaparece.** O `zip` sai da store e vai para o `update.rs`, que monta
  os records a partir de `items` e do retorno do `embed_dense`. Ali o descasamento é genuinamente
  possível (embedder devolvendo contagem diferente da de textos), e é ali que a conferência de
  comprimento passa a viver. Na store, ele deixa de existir.
- **Órfãos da própria mudança** (CLAUDE.md §3): a variante `Error::ArityMismatch` fica morta e os
  testes que provavam "batch rejeitado não persiste" **pela via da aridade** perdem o objeto. Remover
  os dois.
- **`DimensionMismatch` FICA.** Ela é sobre largura do vetor, não sobre contagem de itens, e nada no
  tipo a torna impossível — continua sendo a guarda que transforma degradação silenciosa em erro de
  escrita.

---

## Fases

Ordem escolhida para que cada fase seja verificável sozinha e nenhuma deixe o repo num estado que só
funciona pela metade. **Portão humano entre fases** (padrão TAREFA): não siga para a próxima sem
revisão.

**Estado em 10/08/2026:**

| fase | estado |
| --- | --- |
| 1 — formato do pack | **feita** (PR #139) e em produção: as 27 entidades regeneradas, 48.408 vetores idênticos |
| 2 — migração | **cancelada** pelo próprio Passo 0 (ver abaixo) |
| 3 — caminho incremental | **feita** (PR #140) |
| 4 — remoções com portão humano | **aberta — a única que resta** |
| 5 — invalidação por identidade | **feita** (PR #140, junto da 3) |

As fases 3 e 5 foram entregues **no mesmo pacote**, e isso é decisão: a 5 é a guarda que invalida o
cache, e entregá-la depois deixaria uma janela em que o cache existe sem ela.

### Fase 1 — formato do pack (`doc_path` como id + `key_hash`) — **FEITA**

Ainda **sem** caminho incremental: `reset` + reescrita total, como hoje. Só o formato muda.

- `Record<P>` ganha `key_hash: Option<String>` (`serde(default)`, `skip_serializing_if`). A store
  **não interpreta** o campo — só o carrega; permanece agnóstica (`lib.rs` promete `(id, vector,
  payload)` e a promessa continua válida: o campo é opaco).
- `write_collection_file` passa a gravar em `.tmp` + `rename` (D-INC-5).
- Escrita ordena os records por `id` (D-INC-4).
- `upsert` passa a receber `Vec<Record<P>>` (D-INC-14); `ArityMismatch` e seus testes de aridade saem.
- `update.rs`: monta os `Record` com `id = doc.doc_path()` (os `id-N` morrem) e o `key_hash` do
  `text_to_embed`, conferindo uma única vez que o `embed_dense` devolveu tantos vetores quantos textos.
- `manifest.rs`: `pack_format` (D-INC-7), carimbado como 2.
- Boot: valida `pack_format` com a **mesma igualdade** do triplo de identidade. Depender da tolerância
  do serde a campo desconhecido funcionaria, mas é robustez por acidente, não por contrato.

**Verificação:** o pack gerado tem um `id` por documento igual ao nome do `.md`; a busca do servidor
não muda de resultado (o `scan` nunca leu o `id` — confirmado por grep: `ReadStore` só expõe payloads);
`cargo test` verde.

### ~~Fase 2 — migração (`tools/`)~~ — **CANCELADA em 10/08/2026, e executada de outro jeito**

Ver [TAREFA-FASE-2-MIGRACAO.md](TAREFA-FASE-2-MIGRACAO.md) para o registro completo. Em resumo: o
pré-flight do Passo 0 mediu **27/27 entidades com a árvore em sincronia com o pack**, e com isso a
migração perdeu a razão de existir — ela valia a pena para evitar `auli update` em entidades
divergentes, e não havia nenhuma. O Carlos escolheu o re-embed completo; o migrador nunca foi
escrito.

As 27 foram regeneradas em 128 minutos, e **48.408 vetores saíram idênticos aos anteriores**, um a
um, conferidos contra um backup do formato 1. A verificação que esta fase prometia ("migrado ==
reconstruído, numa entidade") acabou sendo substituída por uma mais forte: "vetor a vetor idêntico
ao anterior, no acervo inteiro".

O que **não** se perdeu com o cancelamento: o `D-INC-13` continua valendo como doutrina — a guarda
de `docs_hash` antes de carimbar `key_hash` é o que a Fase 3 vai precisar quando reaproveitar
vetores. Só não há mais um migrador para aplicá-la.

### Fase 3 — caminho incremental (jurisprudência) — **FEITA**

- `update.rs`: para `Kind` de família jurisprudência, lê o pack atual (`read_collection_file` já é
  pública), monta `id → key_hash`, e parte em três conjuntos:
  **novos/alterados** (embedar), **inalterados** (vetor copiado do pack, payload reconstruído da
  árvore — D-INC-3), **órfãos** (id no pack sem `.md`).
- Serviços e faqs seguem no caminho de hoje (`reset` + total), por D-INC-8.
- `apply(upserts, removes)` na store (D-INC-5, D-INC-6) — nesta fase `removes` chega **sempre vazio**.
- Imprime a taxa de reaproveitamento (D-INC-12).
- `avisar_truncados` passa a tokenizar **só o delta** no caminho incremental (hoje tokeniza a coleção
  inteira a cada update).

**Verificação — o teste que decide tudo (§7.1 do ESTUDO):** rebuild total e sequência incremental
que chega ao mesmo estado lógico produzem arquivos **byte a byte idênticos**. Os f32 são reprodutíveis
graças ao `batch_size=1` da correção v4; a ordenação canônica garante o resto. Cenários mínimos:
nenhuma mudança; um documento novo; um documento com sinopse nova; combinação dos dois.

> **FEITA (PR #140).** Os quatro cenários passaram byte a byte, com o binário real sobre 30 pareceres
> do SC, cada um comparado a uma reescrita total do mesmo estado. No cenário da sinopse, **exatamente
> 1** documento foi re-vetorizado — o `key_hash` ser da KEY e não do arquivo funcionando na prática:
> o `resumo_gerada_em` também mudou naquele `.md` e não invalidou nada.
>
> **O ganho, medido em entidades reais**, com o pack resultante byte-idêntico ao de produção:
>
> | entidade | antes | depois | | reaproveitados |
> | --- | ---: | ---: | ---: | --- |
> | `sc` | 279 s | **9,6 s** | 29× | 1.743 de 1.951 |
> | `rs` | 4.291 s | **297 s** | 14,5× | 22.848 de 25.381 |
>
> **⚠️ Corrige a expectativa da §31 e do ESTUDO.** A projeção era "72 minutos para ~140 vetores", como
> se o incremental levasse a rodada a segundos. O ganho real no RS é **72 min → 5 min**, não segundos,
> porque serviços e faqs (2.533 registros) são re-vetorizados toda rodada por doutrina (D-INC-8) e são
> um piso que o incremental não toca. O que ele elimina é o TARF: 22.476 acórdãos que custavam ~70
> minutos e passam a custar zero quando nada mudou.

### Fase 4 — remoções com portão humano — **A ÚNICA ABERTA**

É a fase onde um erro apaga documento bom. Nada aqui roda sem revisão.

- Diff passa a emitir os órfãos num **log** por coleção (`doc_path` como primeiro campo — é a chave —,
  título e link depois; legível por máquina e por humano).
- Subcomando de aplicação com a regra de interseção e o relatório de três classes (D-INC-10).
- Recarimbo do manifesto pela função compartilhada (D-INC-11).

**Verificação:** aplicar um log aprovado remove exatamente os ids aprovados; rodar duas vezes não faz
mal; documento que reapareceu na árvore não é removido e é relatado.

### Fase 5 — desligar o atalho quando a identidade muda — **FEITA**

`STRATEGY_VERSION` ou `EMBED_MODEL_ID` diferente ⇒ o cache é **inteiramente** invalidado, mesmo com
`key_hash` igual. É a guarda que passa por cima de todas as outras: reaproveitar vetor de outro modelo
é o pior defeito possível deste desenho, porque é silencioso e contamina o acervo todo.

**Verificação:** bump artificial de `STRATEGY_VERSION` força re-embed de 100% dos registros.

> **FEITA (PR #140), no mesmo pacote da Fase 3** — e isso é decisão, não conveniência: entregá-la
> depois deixaria uma janela em que o cache existe sem a guarda que o invalida.
>
> O argumento, que agora vive também no comentário do `run_update`: trocar o `EMBED_MODEL_ID` **não
> altera o `text_to_embed`**, então a key bate, o `key_hash` bate, e o cache serviria vetores de dois
> modelos misturados no mesmo pack — sem que a `DimensionMismatch` pegue nada, porque modelos
> diferentes com 1024 dimensões são o caso comum, não a exceção.
>
> A comparação implementada é do **triplo de identidade inteiro**, não só do modelo: estratégia
> diferente está igualmente errada. Verificado com o `strategy_version` do manifesto bumpado à mão:
> `0 reaproveitados de 32`. Sem manifesto anterior, o cache também nasce desligado.

---

## Notas de contexto que a implementação deve respeitar

- **O `key_hash` não vive dentro do `DocumentoPack`** — vive no `Record` da store, como campo opaco.
  (O ESTUDO cogitava o contrário; a escolha aqui é do `Record`, porque é o vetor que está sendo
  cacheado, e o pareamento com o vetor tem de ser estrutural. A store continua sem interpretar.)
- **No dia 1 o caminho de re-edição do TARF fica dormente**, porque o scraper v1 faz "existe⇒pula"
  ignorando republicações. A correção do scraper (regra `existe E mesmo GUID ⇒ pula`) é **outra
  TAREFA**; quando ela entrar, o `key_hash` pega a mudança sozinho, sem alteração aqui. Nos pareceres
  o mesmo caminho fica dormente por serem imutáveis de fato.
- **Achado menor, fora de escopo, não corrigir aqui:** o `scan` do read-side clona **todos** os
  payloads a cada query antes de truncar para k; daria para ordenar por índice e clonar só os k
  vencedores. Anotado, não pedido.
