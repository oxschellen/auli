# TAREFA — Scrapers próprios (fase 2): schema v2 + extrair a coleta do `auli-collections`

## Contexto

A fase 1 estabeleceu e testou a fronteira scraper→collections: o snapshot
(`../data/<id>/<id>-snapshot.json`, tipos em `auli-contract::snapshot`) é a única
entrada do `auli-collections process`, que deriva contratos `Table<P>`, per-público
JSONs, `servicos-index.json` e os prints `portal-*.txt`. O engine (`auli-cli update`)
segue intocado.

A verificação real da fase 1 (novo × antigo sobre o mesmo cache) confirmou
equivalência byte a byte de todos os agregados RAG/engine, e identificou **uma
limitação conhecida** no lado frontend: o dedup por `link` do schema v1 elimina as
~25 listagens multi-classe do RS (serviço listado sob duas classes aparece só na
primária). Reordenação/renumeração de `id` nos per-público foi aceita como benigna
(D-S5 ajustada). Esta fase corrige a limitação multi-classe **e** extrai os scrapers.

**Ao final**, o `auli-collections` fica somente com o `process`:

```
auli-scraper-rs (bin) ─┐
                       ├─> <id>-snapshot.json (v2) ─> auli-collections <id>  (process)
auli-scraper-sc (bin) ─┘                                    │
                                                     auli update <id>  (inalterado)
```

Motivação prática além da separação de responsabilidades: os scrapers não dependem de
`ort`/`fastembed` nem do vector store — compilam leves, em qualquer ambiente (sem WSL).
E o `auli-scraper-sc` fica livre até do `headless_chrome`, que é exclusividade do RS.

## Decisões

- **D-F2.1.** **Um crate binário por entidade**: `auli-scraper-rs` (headless_chrome +
  ureq; portais de faqs e serviços da SEFAZ-RS) e `auli-scraper-sc` (ureq + regex;
  API Next.js da SEF-SC). Cada scraper conhece **uma** entidade — não lê o
  `registry.toml` (o registro segue sendo assunto do collections/engine). Dependem
  apenas de `auli-contract` e do kit (D-F2.2).
- **D-F2.2.** **Crate de apoio `auli-scraper-kit`** (novo, magro), com o que os dois
  scrapers compartilham: I/O do snapshot (`load/merge/save` + `coletado_em` UTC via
  `time` — o `snapshot.rs` da fase 1), o cache de páginas em disco, o builder do agent
  ureq (user-agent etc.) e o `aggregate_servicos` (atualizado para o schema v2) com
  seus testes. O `auli-contract` segue serde-only — nada de I/O nele.
- **D-F2.3.** **Schema v2 do snapshot — `Ocorrencia`** (corrige a limitação
  multi-classe da fase 1). `ServicoRaw` troca `classe: String` + `publicos:
  Vec<String>` por:

  ```rust
  pub struct ServicoRaw {
      pub titulo: String,
      pub descricao: String,   // corpo limpo
      pub link: String,        // chave natural única
      pub orgao: String,
      /// Onde o serviço aparece no portal, na ordem de descoberta.
      pub ocorrencias: Vec<Ocorrencia>,
  }

  pub struct Ocorrencia {
      pub publico: String,
      pub classe: String,
  }
  ```

  `SNAPSHOT_SCHEMA_VERSION = 2`. O `process` rejeita v1 com erro amigável em PT
  sugerindo re-scrape (snapshot é artefato regenerável do cache — sem migração).
  `FaqRaw` e `ColetaFaqs` não mudam.
- **D-F2.4.** **Derivações sob o v2** (regras do `process`):
  1. *(tipo, classe) primários* de um serviço = a primeira ocorrência encontrada
     iterando os públicos na ordem de `publicos_ordem` (mesma semântica
     first-occurrence da fase 1 — contrato, print e `text_to_embed` saem idênticos).
  2. *Contrato `Table<Servico>`* e *`portal-servicos.txt`*: inalterados — um registro/
     bloco por `link`, com tipo|classe primários.
  3. *Per-público JSONs*: dentro de cada público, **uma entrada por `(link, classe)`**
     — restaura as listagens multi-classe do portal. `id` local reinicia em 1; ordem =
     ordem dos items no snapshot (a divergência de ordem/id segue aceita, D-S5).
- **D-F2.5.** **Convenções de caminho preservadas**: snapshot em `../data/<id>/`,
  caches em `../data/<id>/cache/<colecao>/`. Nenhuma mudança no engine.
- **D-F2.6.** **CLIs**:
  - `auli-scraper-rs [--usecache] faqs|servicos|all` (`all` = as duas coletas em
    sequência, cada uma com merge próprio no snapshot);
  - `auli-scraper-sc [--usecache] servicos` (faqs SC segue fora de escopo);
  - `auli-collections <entity>` passa a executar o `process` diretamente (único
    subcomando restante; aceitar também `<entity> process` como sinônimo para não
    quebrar hábito/scripts).
  Os scrapers **não** chamam o `process` — a cadeia scrape→process vira dois comandos
  explícitos (ou um script em `scripts/`), reforçando a fronteira.
- **D-F2.7.** **Golden test usa o snapshot real como entrada.** Regenerar o snapshot
  golden em v2 (re-scrape com `--usecache` sobre o cache existente); o
  `golden_rs_equivalence` lê o snapshot (via `AULI_GOLDEN_DATA`), roda as derivações
  do `process` e compara com os artefatos golden. O código de síntese da árvore
  `faqs.json`/per-tipo é removido. Golden dos per-público: conferir conteúdo por
  `(link, classe)` — incluindo os ~25 multi-classe restaurados; ordem/id fora do gate.
- **D-F2.8.** Tudo continua no monorepo/workspace — a separação é de crates, não de
  repositórios.

## O que vai para onde

| Hoje (`auli-collections`) | Destino |
| --- | --- |
| `snapshot.rs` (load/merge/save, `coletado_em`) | `auli-scraper-kit` |
| `servicos/cache.rs` (cache de páginas por URL lógica) | `auli-scraper-kit` |
| builder de agent/user-agent (`faqs/fetch.rs` + `sc.rs`, unificar) | `auli-scraper-kit` |
| `aggregate_servicos` + testes (v2: agrega `Ocorrencia`s por `link`) | `auli-scraper-kit` |
| `faqs/` (walk, `fetch`, `html`, árvore `FaqNode`/`faq.rs`, `flatten_faqs_raw`, `FaqSource`) | `auli-scraper-rs` |
| `servicos/extrair_descricoes.rs`, `servicos/utils.rs` (headless Chrome, `get_tipo_servicos`, `descricao_body`) | `auli-scraper-rs` |
| `servicos/sc.rs` (API Next.js, `normalize_links`, públicos) | `auli-scraper-sc` |
| `process.rs` + derivações (contratos, prints, per-público, index) | fica no `auli-collections` |
| `domain/entities.rs` (registry) | fica no `auli-collections` (validação do `process`) |

A árvore `FaqNode` vira detalhe **interno** do `auli-scraper-rs` (só existe em memória
durante o walk); o que sai dele é `Vec<FaqRaw>`.

## Etapas (cada uma verde e commitável, como na fase 1)

### A. Schema v2, ainda no layout atual
No `auli-contract`: `Ocorrencia`, `ServicoRaw` v2, `SNAPSHOT_SCHEMA_VERSION = 2`,
testes atualizados. No collections: `aggregate_servicos` agrega ocorrências (uma por
par público×classe, ordem de descoberta), `process` aplica as derivações da D-F2.4 e
rejeita v1 com erro amigável. Verificar contra o cache: contrato/prints byte a byte
idênticos aos da fase 1; per-público com os multi-classe restaurados.

### B. `auli-scraper-kit`
Criar o crate e **mover** (não copiar) snapshot I/O, cache, agent builder e
`aggregate_servicos`; o `auli-collections` re-aponta as importações. Nenhum
comportamento muda; workspace inteiro segue compilando e testando.

### C. `auli-scraper-rs`
Criar o binário e mover os módulos RS (faqs walk + servicos). O CLI do collections
perde `rs faqs`/`rs servicos` nesta etapa (erro amigável apontando o binário novo).
Docs de módulo migradas/atualizadas. Dependências: `auli-contract`,
`auli-scraper-kit`, `headless_chrome`, `ureq`, parsers já usados.

### D. `auli-scraper-sc`
Idem para o SC (só `servicos`). Sem `headless_chrome` no Cargo.toml — conferir com
`cargo tree -p auli-scraper-sc`.

### E. `auli-collections` process-only + golden novo
- `main.rs` reduzido: resolver entidade via registry e rodar `process`
  (D-F2.6). Remover `errors`/módulos que só serviam à coleta.
- Golden test reescrito conforme D-F2.7; código de síntese removido.
- Atualizar docs de módulo (collections, kit, scrapers) e o workflow de CI
  (`.github/workflows`) para os novos membros do workspace, se ele enumera crates.
- `data/<id>/raw/` continua contendo **apenas** saídas do `process` + o snapshot como
  irmão — conferir que nenhum caminho antigo ficou órfão.

## Critérios de aceitação
- `cargo test --workspace` e `cargo clippy --workspace --all-targets -- -D warnings`
  limpos.
- `cargo tree -p auli-scraper-rs` e `-p auli-scraper-sc` **sem** `ort`/`fastembed`;
  `-p auli-scraper-sc` sem `headless_chrome`.
- Fluxo completo numa entidade com cache/rede:
  `auli-scraper-rs all` → snapshot v2 → `auli-collections rs` → contrato/prints
  idênticos aos da fase 1, per-público com multi-classe restaurado →
  `auli update rs` roda inalterado.
- Golden test (D-F2.7) verde com `AULI_GOLDEN_DATA` apontando para dados reais;
  inerte sem a variável.
- Nenhum código de coleta remanescente no `auli-collections`; snapshot v1 rejeitado
  com mensagem clara.

## Fora de escopo
- Scraper de faqs do SC (o snapshot já o comporta; entra numa tarefa própria).
- Novas entidades, mudanças no engine ou na estratégia de embedding.
- Separação em repositórios distintos (D-F2.8).
