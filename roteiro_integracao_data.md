# Roteiro de integração — pasta `data/` no root

Plano faseado para unificar a integração entre `auli-collections`, o workspace `auli` e
`auli-frontend` sob uma única pasta `data/` no nível root, e para eliminar a triplicação das
definições de entidade. **Este documento é só o plano — nenhuma alteração de código foi feita.**

> **Revisão (2026-06-22, contra o código).** O plano foi auditado linha a linha contra os fontes.
> Ajustes incorporados: (1) `raw/` foi dividido em **`ref/` (autorado, versionado)** vs **`raw/`
> (gerado, gitignored)** — pareceres/notas/conteúdos **não têm scraper** e seriam perdidos num clone
> limpo; (2) Fase 0 ganhou a decisão de **fonte canônica por arquivo**; (3) o passo 3 da Fase 1 foi
> corrigido (não é "só caminho"); (4) registrado que **`load_all` tolera packs ausentes** (entidade
> parcial sobe), corrigindo uma preocupação anterior. Detalhes inline abaixo.

Princípios que guiam a ordem das fases:

1. **Menor risco primeiro.** Começamos por plumbing de caminhos (flags e scripts) com uma única
   micro-mudança de lógica isolada, e só depois mexemos no que tem mais superfície de erro
   (registry único, frontend).
2. **Cada fase deixa o sistema funcionando.** Nada de uma fase quebrar a anterior; ao fim de cada
   uma, `auli server` sobe e responde.
3. **Tudo reversível.** Cada fase tem rollback explícito. Trabalhar em branch.
4. **Definição ≠ artefato.** Tratamos *inventário* (quais entidades/coleções existem) e *conteúdo*
   (os arquivos gerados) como problemas separados — eles têm soluções diferentes.

---

## Estado atual (confirmado no código)

### Quem escreve / lê onde

| Componente | Lê de | Escreve em | Onde isso está no código |
| --- | --- | --- | --- |
| `auli-collections` | config da entidade em `src/entities/<id>/` (`entity.json`, `prompt.txt`) | `./data/<id>/` → `portal-<kind>.txt` + `<kind>.json` (+ `cache/`) | `src/domain/entities.rs`, `src/faqs/mod.rs` |
| `auli update` | `--source <dir>` (lê `<dir>/<file>` por kind) | `--out <dir>` → **plano**: `<id>-<kind>.json` + `<id>.manifest.json` | `crates/auli-cli/src/update.rs` |
| `auli server` | `--packs-dir <dir>` → **plano**: `<dir>/<id>-<kind>.json` + `<dir>/<id>.manifest.json`, por entidade registrada | — (read-only) | `crates/auli-cli/src/packs.rs` |
| `auli-frontend` | `public/<id>/` | — | `src/shared/fetchers.ts` |

### Fatos do código que mudam o desenho (confirmados na revisão)

- **Packs hoje vivem num diretório único e plano.** `load_all` itera as entidades de
  `crate::entities::ENTITIES` e procura `<packs-dir>/<id>-<kind>.json`
  ([packs.rs:42-48](auli/crates/auli-cli/src/packs.rs#L42-L48)). Não há subpasta por entidade.
  **Decisão tomada: cada entidade terá seu próprio diretório de coleções** (`data/<id>/packs/`), o
  que exige uma pequena alteração de lógica em `packs.rs` e no `--out` do `update` (ver Fase 1).
- **`load_all` NÃO quebra com packs ausentes** (corrige uma preocupação do plano original).
  `vector-store::read_collection_file` devolve coleção **vazia** em `NotFound`
  ([lib.rs:62-67](auli/crates/vector-store/src/lib.rs#L62-L67)); então uma entidade **parcial**
  (ex.: `sc` só com `servicos`) **sobe normal** — `load_all` apenas loga `sc-faqs — 0 registros`.
  *Corolário:* o `hash`/`bytes` por coleção do manifesto **não são reconferidos no load**
  (`validate_manifest` só compara modelo/dim/strategy), então **trocar o layout de diretório dos
  packs é seguro** e o `file` do manifesto pode ficar como está.
- **Conteúdo autorado ≠ gerado.** O frontend serve arquivos que **nenhum scraper produz**:
  `conteudo_site_tree.json` (aba Conteúdos) e `portal-pareceres.txt` / `portal-notas.txt`
  (sem scraper — [auli_code.md](auli_code.md) §5.8). Hoje `auli-collections/data/rs/` está
  **incompleto** (sem `servicos-index.json` nem `conteudo_site_tree.json`), enquanto
  `auli-frontend/public/rs/` os tem. Isso força separar **`ref/` (autorado, versionado)** de
  **`raw/` (gerado, gitignored)** — ver Layout-alvo e Fase 0.
- **As definições de entidade estão em 4 lugares**, não 3:
  1. `auli-collections/src/entities/<id>/entity.json` + `prompt.txt` (config por entidade)
  2. `auli-collections/src/domain/entities.rs` (registro)
  3. `auli-cli/src/entities.rs` (`ENTITIES`)
  4. lista de estados no `auli-frontend`

  Essa é a verdadeira fonte da triplicação descrita em `auli_code.md` §6.

---

## Layout-alvo

Cada entidade (estado) armazena **todas** as suas coleções num diretório separado, inclusive os
packs vetorizados:

```text
data/
  registry.toml          # FONTE ÚNICA: entidades + coleções + labels  (versionado no git)
  prompts/
    <id>.txt             # system prompt por entidade (multilinha → arquivo, não inline no toml)
  <id>/                  # rs/, sc/
    ref/                 # conteúdo AUTORADO (sem scraper)                        (VERSIONADO)
                         #   portal-pareceres.txt, portal-notas.txt, conteudo_site_tree.json
    raw/                 # saída GERADA pelo scraper                              (gitignored)
                         #   faqs.json, portal-faqs.txt, servicos*.json, servicos-index.json,
                         #   portal-servicos.txt
    cache/               # cache do scraper                                       (gitignored)
    packs/               # saída do auli update desta entidade:                   (gitignored)
                         #   <id>-<kind>.json + <id>.manifest.json
```

**Decisões travadas:**

- **Packs por entidade** — `data/<id>/packs/`, não um diretório plano compartilhado. Isso isola
  totalmente os dados de cada estado e casa com o modelo multi-tenant.
- **Nome da coleção mantém o id** — dentro de `data/<id>/packs/` o arquivo continua
  `<id>-<kind>.json` (e o manifesto `<id>.manifest.json`). Há redundância com a pasta, mas em troca
  a mudança em `update.rs`/`packs.rs` fica mínima: troca-se o **diretório-base**, não o padrão de
  nome. O campo `file` do manifesto segue `<id>-<kind>.json`, sem tocar (e nem é reconferido no load
  — ver "Fatos do código").
- **Autorado em `ref/` (versionado), gerado em `raw/` (gitignored).** `pareceres`, `notas` e
  `conteudos` **não têm scraper**: seus arquivos (`portal-pareceres.txt`, `portal-notas.txt`,
  `conteudo_site_tree.json`) são **autorados** e ficam em `data/<id>/ref/` **versionado**. Só
  `faqs`/`servicos` (com scraper) caem em `raw/` gitignored. Sem essa separação, o gitignore da
  Fase 4 apagaria conteúdo de referência num clone limpo.

> O que **fica versionado**: `registry.toml`, `prompts/` e **`<id>/ref/`** (autorado). O que vai
> pro `.gitignore`: `<id>/raw/`, `<id>/cache/`, `<id>/packs/` (gerados/scraped e grandes).
> **Cuidado:** nunca jogar `ref/` no gitignore — pareceres, notas e conteúdos seriam perdidos
> (não há scraper que os regere).

---

## Fase 0 — Preparação e decisões (sem código)

**Objetivo:** travar as decisões e montar a rede de segurança antes de tocar em qualquer arquivo.

**Passos**
1. Criar branch: `git checkout -b feat/data-root-integration`.
2. Capturar um **baseline funcional** para comparar depois:
   - Subir o servidor atual e salvar a saída de boot (entidades carregadas, contagem por coleção).
   - Rodar 3–5 perguntas de referência e salvar as respostas. Esse é o seu "antes".
3. Reconciliar as **divergências da §6** *no papel*: listar o que cada uma das 4 fontes declara
   (entidades, coleções por entidade, labels, prompts) e decidir o conjunto canônico. O
   `registry.toml` vai materializar essa decisão — então ela precisa estar fechada antes da Fase 2.
4. **Fonte canônica POR ARQUIVO** (decisão nova — a migração da Fase 1 depende dela). Há três cópias
   parciais e divergentes; escolher a verdade de cada arquivo antes de mover:
   - `portal-*.txt` que **alimentam os packs**: hoje vêm de `auli-server/entities/<id>/` (é o que o
     `auli update` lê na prática), **não** de `auli-collections/data/<id>/`.
   - **Conteúdo de referência** mais completo está em `auli-frontend/public/<id>/` (tem
     `servicos-index.json` e `conteudo_site_tree.json` que `auli-collections/data/rs/` **não** tem).
   - Decidir onde vive o **autorado** (`pareceres`/`notas`/`conteudos`) → `data/<id>/ref/` versionado.
   - **Atenção (Fase 1):** o `auli update --source <dir>` lê `portal-*.txt` de **todos** os kinds de
     um único diretório, mas após o split os `portal-{pareceres,notas}.txt` ficam em `ref/` e os
     `portal-{faqs,servicos}.txt` em `raw/`. Resolver: (a) `update` passa a resolver a origem por
     kind, ou (b) montar um dir "source" agregando `ref/` + `raw/` (symlink/cópia no script). Travar
     a escolha aqui.

**Verificação:** você tem, escrito, (a) baseline de boot+respostas, (b) a tabela canônica de
entidades/coleções e (c) a fonte canônica por arquivo + a decisão de origem do `update --source`.

**Rollback:** nenhum (nada mudou).

---

## Fase 1 — Unificar os artefatos sob `data/`, com packs por entidade (baixo risco)

**Objetivo:** fazer collections, update e server apontarem todos para o root `data/`, com cada
estado em seu próprio diretório. Isso elimina **duas das três cópias manuais** (collections→backend
deixa de existir; só sobra a do frontend).

**Pré-condições:** Fase 0 concluída.

**Passos**
1. Criar a árvore no root: `data/`, `data/<id>/raw/`, `data/<id>/packs/`, `data/prompts/`.
2. **Mover** (não copiar) o conteúdo existente para o novo lugar, **respeitando a fonte canônica da
   Fase 0** (não mover cego de `auli-collections/data/<id>/`, que está incompleto):
   - gerado pelo scraper (`faqs.json`, `portal-faqs.txt`, `servicos*.json`, `servicos-index.json`,
     `portal-servicos.txt`) → `data/<id>/raw/`
   - autorado (`portal-pareceres.txt`, `portal-notas.txt`, `conteudo_site_tree.json`) → `data/<id>/ref/`
   - packs atuais → `data/<id>/packs/` (mantendo os nomes `<id>-<kind>.json` e `<id>.manifest.json`)
3. **collections:** apontar a saída para `data/<id>/raw/`. **Não é "só trocar a constante":**
   `DATA_DIR="./data"` é **relativo ao CWD do collections** (use `../data` ou caminho absoluto), e o
   código compõe `data_dir = DATA_DIR/<id>` + `data_file = data_dir/<base>`
   ([domain/entities.rs:60,102](auli-collections/src/domain/entities.rs#L102)) — é preciso inserir o
   segmento `/raw` na composição. (Config da entidade continua em `src/entities/<id>/` por enquanto —
   isso é Fase 2.)
4. **server (`packs.rs::load_all`) — micro-mudança de lógica, isolada:** montar o caminho por
   entidade em vez de plano. O que era `<packs-dir>/<id>-<kind>.json` passa a
   `<packs-root>/<id>/packs/<id>-<kind>.json`, e o `manifest_path` aponta pra
   `<packs-root>/<id>/packs/<id>.manifest.json`. O padrão de nome do arquivo **não muda**.
5. **update (`update.rs`):** o `--out` passa a ser `data/<id>/packs` por execução (uma por
   entidade). O `CollectionEntry.file` no manifesto segue `<id>-<kind>.json` — sem alteração.
6. **scripts:** atualizar o launcher **vivo** [start_server.sh](start_server.sh) (e, secundariamente,
   `auli/scripts/start_all.sh`/`start_local.sh`, que não estão no caminho atual):
   - `auli update --entity <id> --source <dir> --out data/<id>/packs --version <v>`, onde `<dir>` é
     a origem decidida na Fase 0 (passo 4) — `ref/`+`raw/` agregados, ou origem por kind se `update`
     for ajustado.
   - `auli server --packs-dir data` (a raiz; `load_all` resolve `<id>/packs/` por dentro)
   - Lembrar: `start_server.sh` agora também sobe o **cloudflared** (não ngrok) — não mexer nessa
     parte ao editar os caminhos de dados.

> A mudança de lógica está concentrada nos passos 4 e 5. Fazer **um commit só** para ela, separado
> do plumbing de caminhos (passos 3 e 6), para manter o diff auditável.

**Verificação**
- `auli update` roda para `rs` e escreve em `data/rs/packs/` com manifesto. (Pode rodar para `sc`
  também, mas o server **só carrega `rs`** nesta fase — `sc` vira entidade do server na Fase 2, com
  o registry; os packs de `sc` ficam prontos mas ociosos por enquanto.)
- Boot do servidor: "Manifesto de 'rs' validado" + contagens por coleção **iguais ao baseline da
  Fase 0**.
- As 3–5 perguntas de referência retornam respostas equivalentes ao baseline.

**Rollback:** os caminhos antigos ainda existem se você *copiou* em vez de mover; reverter os
commits (lógica e caminhos são commits separados) via `git revert`/`checkout`.

**Não tocar:** `registry`/definições de entidade, frontend, lógica de RAG/embedding, padrão de
nomes dos arquivos de pack.

---

## Fase 2 — `registry.toml` único (a fase grande)

**Objetivo:** uma só declaração de "o que existe" lida pelos 4 consumidores, eliminando a
triplicação da §6. **Comportamento de scraping continua no collections** — só o *inventário* vira
dado compartilhado.

**Pré-condições:** Fase 1 estável; conjunto canônico da Fase 0 fechado.

**Passos**
1. Escrever `data/registry.toml` com o conjunto canônico: entidades (id, nome/label, UF), coleções
   ativas por entidade, e o caminho do prompt (`prompts/<id>.txt`). Mover os `prompt.txt` atuais
   para `data/prompts/<id>.txt`.
2. **auli-cli:** substituir o `ENTITIES` hardcoded por um loader que faz parse do `registry.toml`
   no boot (serde + `toml`). `load_all` passa a iterar as entidades do registry.
3. **auli-collections:** substituir o registro em `src/domain/entities.rs` (e o uso de
   `src/entities/<id>/entity.json`) por leitura do mesmo `registry.toml`.
4. **frontend:** consumir o registry em vez da lista hardcoded — via import no build (se o toml for
   convertido para JSON no build) **ou** via um endpoint do backend (ver Fase 3, que casa com isso).
5. Garantir **uma única definição do enum/lista de kinds** também, se hoje estiver duplicada.

**Verificação**
- Remover/renomear temporariamente uma entidade no `registry.toml` e confirmar que **os três**
  componentes refletem a mudança sem editar código.
- Boot e perguntas de referência seguem equivalentes ao baseline.

**Rollback:** manter os módulos `domain`/`ENTITIES` antigos atrás de um fallback até a verificação
passar; só então removê-los, num commit separado.

**Não tocar:** os templates de URL e a lógica de dedup do scraper — são específicos do collections
e ficam onde estão.

---

## Fase 3 — Eliminar a terceira cópia (frontend)

**Objetivo:** o frontend para de depender de uma cópia manual em `public/<id>/`.

**Pré-condições:** Fase 1 (artefatos em `data/<id>/raw/` + `ref/`); idealmente Fase 2.

**Duas opções (escolher uma)**

> **Consequência do split `ref/` + `raw/` (achado da revisão):** o frontend serve conteúdo dos
> **dois** diretórios (`faqs`/`servicos` de `raw/`; `pareceres`/`notas`/`conteudos` de `ref/`). Um
> único symlink **não junta duas pastas**, então a opção A pura não cobre tudo.

- **A — Symlinks (rápido, dev):** **dois** symlinks por entidade não dá (colidem no mesmo
  `public/<id>`). Alternativas: (a-1) symlinkar por arquivo/coleção; (a-2) um passo de build que
  copia `ref/` + `raw/` para `public/<id>/`. **Cuidado WSL/Windows:** symlink versionado no git pode
  quebrar no checkout em filesystem Windows; trate como setup local (`start_local.sh`) ou gere no
  boot do dev server — nunca commitado.
- **B — Endpoint de leitura (limpo, longo prazo):** `GET /v1/<id>/<kind>` no backend servindo de
  `data/<id>/{raw,ref}/` (o backend **mescla** as duas origens — vantagem decisiva sobre o symlink),
  e o frontend passa a fazer fetch da API em vez de ler arquivo estático. Elimina a cópia de vez e
  casa com a Fase 2 passo 4. Custo: uma rota nova + ajuste nos `fetchers.ts`.

**Recomendação (revisada):** como o conteúdo agora vem de `ref/` **e** `raw/`, a opção A vira um
**passo de cópia no build** (a-2), não um symlink simples; **B é o destino** e resolve a mescla
naturalmente. Se quiser destravar já, faça a-2; abra B como item seguinte.

**Verificação:** abas de referência (Serviços, FAQs, …) carregam o mesmo conteúdo de antes, sem
nenhum arquivo em `public/<id>/` mantido à mão.

**Rollback:** restaurar a cópia em `public/<id>/` e reverter `fetchers.ts` (opção B).

---

## Fase 4 — Limpeza, guard-rails e docs

**Objetivo:** remover o que ficou obsoleto e impedir regressão para o estado triplicado.

**Passos**
1. `.gitignore`: adicionar `data/<id>/raw/`, `data/<id>/cache/`, `data/<id>/packs/`; **garantir que
   `data/registry.toml`, `data/prompts/` e `data/<id>/ref/` (autorado) fiquem versionados**. Conferir
   com `git status`/`git check-ignore` que nenhum arquivo de `ref/` foi ignorado por engano.
2. Remover as pastas/códigos mortos: `auli-collections/data/` antigo, `entities/<id>/` do backend
   baseline, `public/<id>/` copiado à mão (conforme a opção da Fase 3), e os módulos `domain`
   duplicados substituídos na Fase 2.
3. Atualizar a documentação:
   - `auli_code.md` §2 — redesenhar o diagrama para o fluxo com `data/<id>/` (e packs por entidade,
     no lugar das rotas de ingestão do baseline).
   - `auli_operations.md` — caminhos do runbook (`--source`, `--out`, `--packs-dir`).
   - `README.md` — a tabela "Repository layout" e a nota de "files copied between directories".
   - `auli_code.md` §6 — marcar as divergências como resolvidas (ou removê-las).
4. *(Opcional)* um guard-rail leve: um alvo de `just`/`make` ou um pequeno check em CI que falha se
   reaparecer uma definição de entidade fora do `registry.toml`.

**Verificação:** clone limpo do repo + Fase 0/baseline reproduzem boot e respostas equivalentes,
sem nenhuma cópia manual no caminho.

---

## Resumo da ordem e do risco

| Fase | Entrega | Mexe em código? | Risco | Cópias manuais restantes |
| --- | --- | --- | --- | --- |
| 0 | Decisões + baseline | não | nenhum | 3 |
| 1 | Artefatos sob `data/<id>/`, packs por entidade | caminhos + micro-lógica em `load_all`/`update` | baixo | 1 (frontend) |
| 2 | `registry.toml` único | sim (4 consumidores) | médio | 1 |
| 3 | Frontend sem cópia | symlink (A) ou rota (B) | baixo (A) / médio (B) | 0 |
| 4 | Limpeza + docs + guard-rail | remoções | baixo | 0 |

**Caminho mínimo para já sentir ganho:** Fase 1 sozinha já mata a cópia collections→backend,
centraliza os artefatos e isola cada estado em seu diretório. Dá pra parar ali e seguir o resto em
outra sessão.
