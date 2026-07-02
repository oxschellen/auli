# TAREFA — Snapshot de coleta (fase 1): tipos no contrato + subcomando `process`

## Contexto

Hoje o `auli-collections` mistura três responsabilidades: **coleta** (rede, headless
Chrome no RS, API Next.js no SC), **normalização** (dedup por link, corpo da descrição,
materialização de `text_to_embed`) e **geração de artefatos** (contratos `Table<P>`,
per-público JSONs, `servicos-index.json`, `portal-*.txt`).

O objetivo do projeto (em duas fases) é separar a coleta do processamento por meio de um
**arquivo único padronizado por entidade** — o *snapshot de coleta*:

```
scraper (rs|sc) ──> <id>-snapshot.json ──> auli-collections process <id>
                                            ├─> <id>-faqs.json / <id>-servicos.json   (Table<P>, p/ engine)
                                            ├─> per-público JSONs + servicos-index.json (p/ frontend)
                                            └─> portal-faqs.txt / portal-servicos.txt   (auditoria)
```

O `auli-cli update` (engine) **não muda**: continua lendo `<id>-faqs.json` e
`<id>-servicos.json` e vetorizando `text_to_embed()`. Toda a refatoração acontece a
montante dele.

**Esta fase 1** cria os tipos do snapshot no `auli-contract` e o pipeline
`snapshot -> process` dentro do próprio `auli-collections` (os fluxos de scrape atuais
passam a *gravar* o snapshot e então processá-lo). A extração dos scrapers para binários
próprios é a fase 2 e está **fora de escopo** aqui.

## Decisões (registrar como comentários de módulo, no estilo do repo)

- **D-S1.** O snapshot é a fronteira scraper→collections; os `Table<P>` continuam sendo a
  fronteira collections→engine. Ambos vivem no `auli-contract` (que segue magro, só serde).
- **D-S2.** O snapshot carrega dado **bruto porém limpo**: texto já normalizado (links no
  formato `anchor "url"`, descrição = corpo sem o header `tipo/classe/titulo`), mas **sem
  campos derivados** — sem `id` sequencial e sem `text_to_embed`. Quem deriva é o `process`.
  (Isto move a materialização da key para o `process`; a D2 original — "o scraper
  materializa" — é substituída: quem materializa agora é o collections, e os coletores
  ficam agnósticos de `STRATEGY_VERSION`.)
- **D-S3.** Um serviço = **um registro**, com `publicos: Vec<String>`. O dedup por link
  deixa de existir no formato (o `link` é a chave natural única do snapshot). O fan-out
  per-público vira responsabilidade do `process`.
- **D-S4.** Equivalência de saída: os artefatos gerados via `process` devem ser
  **idênticos** aos atuais (contratos, prints, index), exceto onde esta tarefa documenta
  diferença intencional (per-público enxuto, ver D-S5).
- **D-S5.** Os per-público JSONs mantêm o shape atual, **incluindo `descricao`**:
  `{ id, tipo, classe, orgao, link, titulo, descricao }`. Única diferença aceita:
  `descricao` passa a ser o **corpo limpo** vindo do snapshot (sem o header
  `tipo/classe/titulo` duplicado que os per-tipo carregam hoje) — não reconstruir o
  header, pois esses campos já existem como colunas próprias e o frontend
  (`auli-frontend/src/pages/servicoslist/utils.ts`) lê apenas `id, classe, titulo, link`.
- **D-S6.** O subcomando `rebuild` é **removido** (substituído por `process`, que é
  offline por natureza). Remover também o código morto que só o `rebuild` usava
  (`faqs::rebuild_contract_from_tree`, `servicos::rebuild_contract_from_raw`, leitura da
  árvore `faqs.json` como fonte).

## Formato do snapshot

Caminho: `../data/<id>/<id>-snapshot.json` (irmão de `raw/`; `raw/` segue sendo só saída
gerada pelo `process` + caches).

```json
{
  "schema_version": 1,
  "entidade": "rs",
  "scraper": { "nome": "auli-collections", "versao": "<CARGO_PKG_VERSION>" },
  "colecoes": {
    "faqs": {
      "coletado_em": "2026-07-01T09:14:00-03:00",
      "items": [
        {
          "pergunta": "Como emitir a guia de ICMS?",
          "resposta": "Acesse o portal...",
          "origin": "Inicial | Perguntas Frequentes | ICMS",
          "url": "https://atendimento.receita.rs.gov.br/..."
        }
      ]
    },
    "servicos": {
      "coletado_em": "2026-07-01T10:02:00-03:00",
      "publicos_ordem": [
        { "nome": "Cidadãos", "slug": "rs-servicos-ao-cidadao" },
        { "nome": "Empresas", "slug": "rs-servicos-a-empresas" }
      ],
      "items": [
        {
          "titulo": "Emitir guia de arrecadação",
          "descricao": "Corpo limpo da descrição...",
          "link": "https://www.fazenda.rs.gov.br/...",
          "orgao": "SEFAZ",
          "classe": "ICMS",
          "publicos": ["Empresas", "Cidadãos"]
        }
      ]
    }
  }
}
```

Notas sobre o formato:

- `colecoes.faqs` e `colecoes.servicos` são **opcionais** (`Option`, `skip_serializing_if`):
  cada scrape atualiza só a sua coleção, preservando a outra (merge, não overwrite do
  arquivo inteiro).
- `publicos_ordem` define a ordem de exibição das abas (gera o `servicos-index.json`:
  `nome` -> `tipo`, `slug` -> `filename`) e desempata o "público primário" de cada serviço.
- `coletado_em`: RFC 3339. `schema_version` desconhecido -> erro amigável em português.
- Gravar com `serde_json::to_string_pretty` (diffs legíveis no git).

## Tipos novos no `auli-contract` (módulo `snapshot`)

```rust
// auli-contract/src/snapshot.rs (novo módulo, reexportado no lib.rs)

pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub entidade: String,
    pub scraper: ScraperInfo,
    pub colecoes: Colecoes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScraperInfo { pub nome: String, pub versao: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Colecoes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faqs: Option<ColetaFaqs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub servicos: Option<ColetaServicos>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColetaFaqs { pub coletado_em: String, pub items: Vec<FaqRaw> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColetaServicos {
    pub coletado_em: String,
    pub publicos_ordem: Vec<Publico>,
    pub items: Vec<ServicoRaw>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publico { pub nome: String, pub slug: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaqRaw {
    pub pergunta: String,
    pub resposta: String,
    #[serde(default)]
    pub origin: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicoRaw {
    pub titulo: String,
    pub descricao: String,   // corpo limpo (sem header tipo/classe/titulo)
    pub link: String,        // chave natural única
    pub orgao: String,
    pub classe: String,
    pub publicos: Vec<String>,
}
```

Testes no crate: roundtrip JSON do `Snapshot`; coleção ausente desserializa como `None`;
`schema_version` presente no JSON gravado.

## Regras de derivação no `process` (preservar fórmulas atuais)

1. **Faq (contrato).** Para cada `FaqRaw`, na ordem do snapshot:
   `text_to_embed = if origin.is_empty() { pergunta } else { format!("{} {}", origin, pergunta) }`
   (mesma fórmula de `faqs::collect_faqs` hoje). Demais campos copiados 1:1.
2. **Servico (contrato).** Para cada `ServicoRaw`, na ordem do snapshot, com `id`
   sequencial a partir de 1:
   - `tipo` = **público primário** = o primeiro `nome` de `publicos_ordem` que esteja em
     `publicos` (fallback: primeiro item de `publicos`).
   - `text_to_embed` = fórmula atual de `servico_text_to_embed` (breadcrumb
     `tipo | classe`, título, primeiros 300 chars do corpo).
3. **Ordem dos items no snapshot** (responsabilidade da escrita, etapa B): iterar os
   públicos na ordem de `publicos_ordem` e, dentro de cada público, na ordem do portal;
   um serviço entra na posição da sua **primeira ocorrência** (mesma semântica do dedup
   first-occurrence-wins atual). Assim os `id` e a numeração do print ficam idênticos aos
   de hoje.
4. **portal-servicos.txt**: mesmo bloco atual (`// N.` + `## pergunta` breadcrumb+título +
   `## resposta` corpo + `Link:`), agora renderizado direto dos items do snapshot.
5. **portal-faqs.txt**: mesmo bloco atual de `faqs::portal::render_portal_faqs`, agora
   renderizado do flat (verificado: o render atual não usa nada da árvore além de
   origin/url/pergunta/resposta, na mesma ordem do flatten — a saída deve sair idêntica
   byte a byte).
6. **Per-público JSONs** (`raw/<slug>.json`): fan-out — um arquivo por entrada de
   `publicos_ordem`, contendo os serviços cujo `publicos` inclui aquele nome, no shape
   completo da D-S5 (`descricao` = corpo limpo do snapshot; `tipo` = o nome do público
   daquele arquivo), com `id` local reiniciando em 1 por arquivo (comportamento atual
   do RS).
7. **servicos-index.json**: derivado de `publicos_ordem` (`{ tipo: nome, filename: slug }`).

## Etapas

> **Nota de implementação (opção R — B aditiva).** A etapa B foi entregue de forma **aditiva**: os
> fluxos de scrape passam a *também* gravar o `<id>-snapshot.json` (honrando merge por coleção, ordem
> first-occurrence via `publicos_ordem` e SC um-registro-por-serviço), mas **nada é removido** e todos
> os artefatos atuais seguem sendo gerados. Os dois itens que acoplavam B↔C — os fluxos *chamarem* o
> `process` e o SC *deixar* de escrever os per-público — foram **movidos para a etapa C** (senão
> `finish()` quebraria sem o `process` existir). Assim cada passo fica verde e shippável.
>
> **Verificação de scrape real:** esta máquina **não tem cache de páginas** (`data/<id>/cache/`
> vazio), então o protocolo da etapa E — rodar o scrape antes/depois — **roda na máquina do autor**.
> Localmente a derivação é conferível sintetizando o snapshot a partir dos intermediários já
> existentes e diffando a saída do `process` contra os artefatos golden.
>
> **Resultado do gate golden local (etapa C):** o teste `golden_rs_equivalence` (inerte sem
> `AULI_GOLDEN_DATA`) sintetiza as coletas de `data/rs/raw/{faqs.json, per-tipo}` e roda as derivações.
> `rs-faqs.json`, `rs-servicos.json` e `portal-faqs.txt` saem **byte a byte idênticos**;
> `servicos-index.json` bate no conteúdo (o golden em disco só tem um `\n` final que o
> `to_string_pretty` — antigo e novo — não emite). `portal-servicos.txt` não tem golden RS em disco,
> então fica para o scrape real do autor.

### A. `auli-contract`: módulo `snapshot`
Tipos e testes acima. Docs de módulo explicando a nova fronteira (D-S1, D-S2).

### B. `auli-collections`: escrita do snapshot nos fluxos de scrape
- Novo módulo `snapshot.rs` no collections com load/merge/save do arquivo
  (`load_or_default`, atualizar só a coleção raspada, gravar pretty).
- **faqs (rs)**: após `scrape()`, converter a árvore em `Vec<FaqRaw>` (mesma travessia do
  `flatten_faqs`, sem `text_to_embed`) e gravar em `colecoes.faqs`. A árvore `FaqNode`
  continua existindo só em memória durante o scrape.
- **servicos (rs)**: após a raspagem per-tipo, agregar em `Vec<ServicoRaw>` juntando os
  `publicos` por `link` (ordem conforme regra 3) e montar `publicos_ordem` a partir de
  `utils::get_tipo_servicos()` (`tipo` -> `nome`, `filename` -> `slug`); `descricao` já
  entra como corpo (`descricao_body` aplicado na coleta).
- **servicos (sc)**: o backend SC já tem `publicos` como lista na API — mapear direto,
  sem o fan-out per-público que ele faz hoje (isso passa ao `process`).
- Os fluxos `faqs`/`servicos` terminam chamando o `process` (etapa C), de modo que o
  comportamento observável do CLI atual se mantém.

### C. `auli-collections`: subcomando `process`
- `cargo run <entity> process`: lê `<id>-snapshot.json`, valida `schema_version` e
  `entidade`, e gera **todos** os artefatos das regras 1–7 em `data/<id>/raw/`.
  Coleção ausente no snapshot -> pular com aviso (`⏭️`), não é erro.
- Fatorar a geração de artefatos para funções que recebem `&Snapshot` (ou as coletas),
  sem I/O de leitura além do snapshot.

### D. Limpeza (política de código morto do repo)
- Remover `rebuild` do `main.rs`, `rebuild_contract_from_tree`,
  `rebuild_contract_from_raw` e a escrita/leitura dos per-tipo como fonte intermediária
  do contrato (os per-tipo agora são só *saída* do `process`).
- Atualizar os docs de módulo (`main.rs`, `servicos/mod.rs`, `faqs/mod.rs`) para o novo
  fluxo.

### E. Verificação de equivalência (usar `--usecache`)
1. Em um checkout **antes** da mudança: `cargo run --usecache rs faqs` e
   `cargo run --usecache rs servicos`; guardar `rs-faqs.json`, `rs-servicos.json`,
   `portal-faqs.txt`, `portal-servicos.txt`, `servicos-index.json`.
2. Depois da mudança: repetir + `cargo run rs process`.
3. `diff` deve ser vazio para os cinco artefatos acima. Diferença aceita: apenas o campo
   `descricao` dos per-público (corpo limpo, sem o header duplicado — D-S5); todos os
   demais campos (`id/tipo/classe/orgao/link/titulo`) devem bater exatamente.
4. Repetir o possível para `sc servicos` (se houver cache local de SC).

## Critérios de aceitação
- `cargo test` e `cargo clippy -- -D warnings` limpos no workspace.
- `cargo run <e> faqs|servicos` produz snapshot + artefatos; `cargo run <e> process`
  regenera artefatos idênticos sem rede.
- Diffs da etapa E vazios (exceto per-público, documentado).
- Nenhuma referência remanescente a `rebuild` ou à árvore `faqs.json` como fonte.
- `auli-cli update` roda inalterado sobre os contratos gerados.

## Fora de escopo (fase 2)
- Extração dos scrapers para binários/crates próprios (`auli-scraper-rs`, `auli-scraper-sc`).
- Scraper de faqs do SC (portal Next.js — walk/parse ainda não escrito; o snapshot já o
  comporta sem mudança de schema).
- Qualquer mudança no engine, na estratégia de embedding ou em `STRATEGY_VERSION`.
