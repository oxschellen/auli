# Auli — Operação (compilar, subir, cloudflared, logs, publicar o frontend)

Runbook prático para **compilar, gerar os dados e subir o servidor da Auli** (workspace `auli-server`,
modo `server`) com o túnel do **Cloudflare** (cloudflared), e para saber **onde ficam os logs**. Para a descrição
técnica do código, ver [docs/auli_code.md](docs/auli_code.md) (§3 cobre o workspace `auli-server`).

> TL;DR — numa máquina já preparada (build feito, packs gerados):
>
> ```bash
> ./start_server.sh                  # compila (incremental) + sobe server + túnel
> ./start_server.sh --no-build       # restart rápido, sem recompilar
> ./start_server.sh --no-tunnel       # só o servidor local, sem túnel
> ```

---

## 1. O que sobe

**Dois comandos, em dois terminais:**

```bash
./start_server.sh              # terminal 1 — a API (ocupa o terminal; Ctrl+C derruba com o túnel)
scripts/deploy-frontend.sh     # terminal 2 — o site (pergunta antes de começar)
```

São independentes: o site é estático e serve as abas de referência do próprio origin; só o chat fala
com a API. Com a API fora do ar, suba ela primeiro — volta em minutos, enquanto o deploy sobe
~253 MB por `scp`. A ordem de chamada de dentro do deploy está em
[scripts/README.md](../scripts/README.md).

Cada um compila o que precisa, então rodar separado **não** reintroduz o risco de publicar artefato
de código velho — ver a §11 para o acidente que motivou essa guarda.

> Houve um `scripts/publicar.sh` que encadeava os dois num comando só. Ele nasceu quando o
> `build-packs.sh` e o `deploy-frontend.sh` ainda não compilavam, e a compilação única no topo era a
> garantia. Assim que os dois passaram a compilar sozinhos (09/08/2026), o que sobrou dele foi um
> `read` de confirmação — que foi para dentro do `deploy-frontend.sh`, onde vale sempre e não só
> para quem entra pela porta de cima.

### Os binários

O binário único `auli` tem **três** modos:

- **`auli server`** — sobe a API HTTP (axum) em `:3000`, **somente leitura**: carrega os pacotes
  de vetores, valida o manifesto e responde perguntas (RAG). Embeda só a pergunta; nunca escreve.
- **`auli update`** — lê as **árvores `.md`** (`data/<id>/docs/<kind>/*.md`, uma por coleção — a
  fonte desde a G5b; o contrato JSON saiu do caminho), embeda o `text_to_embed` de cada registro e
  escreve os **pacotes** (`<id>-<kind>.json` + `<id>.manifest.json`). É o **único do engine** que
  escreve dados.
- **`auli bundle`** — empacota as árvores `.md` nos `.zip` de download por estado, com README e
  manifesto. Não toca em vetor nenhum.

Ao lado dele há mais um binário e uma família:

- **`auli-collections`** — deriva artefatos das árvores (`indice`, `process`, `sinopse`). É quem
  gera o `<id>-<colecao>-index.json` que as abas de jurisprudência leem.
- **`auli-scraper-<uf>`** (28 crates) — a coleta. Só entram quando se busca conteúdo novo do portal;
  nunca no caminho de publicação.

### As sequências, e o que cada uma custa

| sequência | o que faz | custo |
|---|---|---|
| coleta (`auli-scraper-<uf>`) | portal → `data/<id>/docs/` | horas, depende de rede |
| `build-packs.sh` | árvore → packs + índices | 72 min e ~42 GB no `rs` |
| `deploy-frontend.sh` | `public/` + zips + build + upload | minutos |
| `start_server.sh` | API + túnel | minutos, **fica em primeiro plano** |

O `start_server.sh` não daemoniza: ele segura o terminal para o `trap` derrubar o `cloudflared` no
Ctrl+C. É por isso que ele mora num terminal só dele, e o deploy roda em outro.

Embeddings (fastembed/BGE-M3) e busca vetorial rodam **in-process** — não há Ollama, ChromaDB nem
serviço de embedding para subir à parte.

---

## 2. Pré-requisitos

| Item                     | Detalhe                                                                                                                                                           |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Rust**                 | toolchain estável (`cargo`, `rustc`).                                                                                                                             |
| **cmake + compilador C** | exigidos por `aws-lc-sys` (TLS rustls). **Nesta máquina não há cmake de sistema**: foi instalado via `pip install --user cmake` (fica em `~/.local/bin`). Ver §3. |
| **Rede (1º build/run)**  | `ort` baixa o ONNX Runtime no build; o modelo **BGE-M3** baixa do Hugging Face para `EMBED_CACHE_DIR` no 1º uso.                                                  |
| **cloudflared**          | túnel do Cloudflare que publica `api.auli.com.br` (em `~/.local/bin`). Configure 1× com `./setup-cloudflared.sh`. Opcional ao rodar: `--no-tunnel`.               |
| **`.env`**               | na raiz do repo `auli/` (ver §4).                                                                                                                                 |

---

## 3. Compilar (build)

```bash
cd /home/ubu/Desktop/auli/auli-server     # o workspace Cargo (raiz do repo: /home/ubu/Desktop/auli)

# cmake desta máquina (pip) + compat de policy do cmake 4 — INÓCUO onde já houver cmake de sistema:
export PATH="$HOME/.local/bin:$PATH"
export CMAKE_POLICY_VERSION_MINIMUM=3.5

cargo build --release --workspace      # ou só o engine: cargo build --release --bin auli
```

- Binários em `auli-server/target/release/`: **`auli`** (server/update) e **um scraper por entidade**
  (9): `auli-scraper-{rs,sc,sp,pr,mg,pe,ba,rj,ce}`. **Nenhum usa headless Chrome** — todos são ureq +
  HTML/JSON (o RS migrou para API JSON `tudofacil`; o BA usa `native-tls`/OpenSSL por causa do TLS
  1.2-CBC do portal). A técnica de cada um está em
  [`crates/scrapers/SCRAPERS.md`](auli-server/crates/scrapers/SCRAPERS.md).
- 1º build recompila fastembed/ort/aws-lc (alguns minutos); depois é incremental (segundos). Os
  scrapers **não** dependem de fastembed/ort — compilam leves (invariante do `crates/scrapers/`).
- Numa máquina com cmake de sistema, **nenhuma** das `export` é necessária.
- Testes: `cargo test --workspace`.

> Se faltar cmake: `python3 -m pip install --user cmake` (sem sudo) e garanta `~/.local/bin` no PATH.

---

## 4. Dados necessários para servir

Os dados vivem na pasta **`data/`** na raiz (`AULI_DATA_DIR`, default `../data` a partir de
`auli-server/`); o **catálogo** (`registry.toml` + `prompts/`) vive na **`config/`** irmã, derivada
dela por convenção (`AULI_CONFIG_DIR` sobrepõe). O `auli server` lê os packs por entidade e — desde
o pack v2 (ago/2026) — as **árvores `data/<id>/docs/<kind>/*.md` das QUATRO coleções**, de onde o
conteúdo é lido **na hora da query**, só para os k documentos escolhidos. O pack guarda o payload
leve, nunca o texto.

> ⚠️ **A árvore `docs/` é requisito de serving, não artefato intermediário.** O manifesto carimba um
> `docs_hash` e o boot **recusa subir** se a árvore divergir dele. Ao copiar dados entre máquinas,
> leve `packs/` **e** `docs/` juntos — pack sem árvore não serve.

**O repositório guarda código + config, não dado coletado.** O catálogo versionado é a pasta
**`config/`** (`registry.toml` + `prompts/`), irmã de `data/`; todo o resto de `data/<id>/**` é
gitignored e reconstruído pelo
pipeline. Um clone novo, portanto, **não traz dados** — rode o pipeline para popular.

> ⚠️ **Ao sincronizar a mudança que tirou os dados do git, arquivos locais em `data/<id>/ref/`
> somem.** Os arquivos do RS (`rs-portal-pareceres.txt`, `rs-portal-notas.txt`,
> `rs-conteudo_site_tree.json`) eram versionados até então. O commit que os destrackeia registra
> uma **deleção**; ao trocar de branch ou mergear, o git os trata como "rastreados e removidos" e
> **apaga do working tree** — mesmo que `git rm --cached` os tenha preservado no momento da
> operação. Perda observada na prática nesta migração.
>
> **Recuperação** (o conteúdo continua no histórico — não houve rewrite):
>
> ```bash
> PRE=<commit-do-merge-que-destrackeou>^
> mkdir -p data/rs/ref
> for f in rs-portal-pareceres.txt rs-portal-notas.txt rs-conteudo_site_tree.json; do
>   git show "$PRE:data/rs/ref/$f" > "data/rs/ref/$f"
> done
> ```
>
> Conferir depois: `grep -cE '^// [0-9]+' data/rs/ref/rs-portal-pareceres.txt` (372 blocos) e
> `grep -c '### Descrição Resumida' …` (372 sinopses). **Faça backup de `data/<id>/ref/` antes de
> sincronizar** numa máquina que tenha dados locais — é a única parte de `data/` que já esteve no
> git e, por isso, a única sujeita a esse efeito. `raw/`, `packs/` e o cache nunca foram
> rastreados e não são afetados.

### 4.1 Pacotes de vetores (`data/<id>/packs/`)

> **Regenerar as 27 — três coisas aprendidas na operação de 12/08/2026** (relatório de execução
> removido; a operação está no git):
>
> - **é tudo-ou-nada.** O `packs::load_all` percorre **todas** as entidades do registry e falha duro na
>   primeira divergente. Com 26 regeneradas e 1 faltando, **nenhum** binário sobe: o novo tropeça na
>   que ficou para trás, o antigo nas 26 que já avançaram. A janela sem servidor reiniciável abre no
>   primeiro pack e só fecha no vigésimo sétimo;
> - **"o servidor antigo tolera pack novo" é falso.** O `validate_manifest` exige **igualdade** do trio
>   nos dois sentidos. O que existe é outra coisa: o servidor carrega os packs **uma vez, no boot**, e
>   nunca mais os relê — então regenerar com ele no ar é seguro **enquanto ele não reiniciar**;
> - **deixe o `build-packs.sh` recompilar em cada entidade.** O `cargo build` é no-op de menos de um
>   segundo quando está em dia, e é a guarda que o próprio script chama de caso mais traiçoeiro do
>   repositório: binário velho grava vetores de fórmula antiga que **passam** na validação de boot.
>
> Ordem recomendada: crescente por tamanho, com `sp` e `rs` no fim — se algo estiver errado, aparece
> em segundos e não depois de 70 minutos. **Backup dos packs antes**, que é a única parte irreversível.

Pipeline em **três passos** (a coleta virou binários próprios na fase 2; tudo roda de `auli-server/`):

1. **Raspar** (rede, **sem headless**) → grava um snapshot por coleção `data/<id>/<id>-<kind>-snapshot.json` (v3):
   `auli-scraper-<id> servicos` para cada uma das **27 entidades** (rs/sc/sp/pr/mg/pe/ba/rj/ce/ms/mt/go/pi/am/pa/es/ro/to/ma/ap/ac/df/rn/pb/al/se/rr); o RS
   também aceita `faqs`/`all`. `--usecache` reusa o cache de páginas (offline, sem rede).
   > **Dependência de runtime do `go`, `df` e `se`:** os scrapers `auli-scraper-go`, `auli-scraper-df` e
   > `auli-scraper-se` chamam o binário **`curl`** (no PATH) para os GETs de catálogo — GO e DF ficam
   > atrás de um WAF que bloqueia o fingerprint TLS (JA3) do `ureq`, e o SharePoint do SE encerra a
   > conexão de um jeito que o `ureq` rejeita (`unexpected end of file`) mas o curl tolera
   > (ver `go_waf.md`/pendências §11). Garantir `curl` instalado no **desktop de coleta E no host
   > do túnel**, se a coleta rodar lá. No modo `--usecache` o curl não é chamado (lê do cache).
2. **Derivar** (offline) → o contrato `<id>-faqs.json`/`<id>-servicos.json` + prints + index +
   per-público (e a árvore `faqs-tree.json` p/ a UI, no RS) em `data/<id>/raw/`:
   `./target/release/auli-collections <id>`.
3. **Vetorizar** → `scripts/build-packs.sh <id>` (roda o `auli update`, que lê as árvores `docs/`).

> **Mexeu em UMA árvore só? Rode só ela** (a partir de 20/08/2026, D-DH-1..8). O caso típico é uma
> entrega de legislação: `scripts/build-packs.sh rs legislacao` vetoriza os pares daquela coleção
> em segundos e reescreve um pack pequeno, em vez de re-embedar serviços + FAQs + legislação e
> reescrever centenas de MB porque uma pasta mudou. Aceita mais de uma:
> `scripts/build-packs.sh rs faqs legislacao`.
>
> **O que a rodada parcial NÃO afrouxa.** O manifesto carimba um hash **por subdiretório** de
> `docs/`, e o boot confere o mapa **inteiro** contra o disco — não só o que rodou. Mexer em duas
> árvores e recarimbar uma é recusa NOMEADA no próximo restart ("docs/faqs/ mudou: manifesto tem
> X, disco tem Y"), com o remédio na mensagem. Nunca silêncio.
>
> **Leia o aviso do fim da rodada.** Quando alguma árvore fora do filtro diverge do carimbo
> preservado, o `update` imprime `⚠️ Árvores fora do filtro divergentes` e lista quais. É aviso,
> não erro — o fluxo legítimo de "mexi em duas" é rodar dois `--kind` em sequência —, mas ignorá-lo
> significa um servidor que não sobe no próximo restart.
>
> **Três recusas antes de carregar o modelo**, todas erro alto: sem manifesto anterior (não há o
> que preservar), manifesto sem o mapa (anterior à migração — rode o completo uma vez), ou
> identidade de embedding divergente (a parcial carimbaria a identidade nova sobre packs do espaço
> velho, e o boot os abençoaria). Falham em milissegundos, de propósito.

```bash
cd auli-server
# RS (FAQs + serviços):
./target/release/auli-scraper-rs all && ./target/release/auli-collections rs && (cd .. && scripts/build-packs.sh rs)
# Demais (só serviços) — a partir de auli-server/:
for id in sc sp pr mg pe ba rj ce; do
  ./target/release/auli-scraper-$id servicos && ./target/release/auli-collections $id && (cd .. && scripts/build-packs.sh $id)
done
```

Produz, por entidade, `data/<id>/packs/<id>-<kind>.json` + `<id>.manifest.json`
(`strategy_version: 2` desde o pack v2 de ago/2026).

**Contagens (08/08/2026): 29.732 documentos** — 4.205 serviços nas 27 entidades, 19.780 pareceres
(sp 15.605, pr 2.060, sc 1.743, rs 372), 1.947 FAQs (só rs) e 3.800 acórdãos do TARF (rs, coleta em
andamento). Números viram mentira sozinhos; para a verdade do momento:

```bash
python3 -c "
import json,glob
t={}
for m in sorted(glob.glob('data/*/packs/*.manifest.json')):
    d=json.load(open(m))
    for c in d['collections']: t[c['kind']]=t.get(c['kind'],0)+c['count']
print(t, sum(t.values()))"
```

`notas` é autorada (sem scraper) e ainda **não** tem fonte struct no contrato — fica
**ausente** até ser modelada; o server tolera packs ausentes (sobe com a coleção vazia). `pareceres`
também é autorado, mas já tem um passo de ingestão do `.txt` de referência (ver nota abaixo). **Só
precisa rodar de novo quando o conteúdo ou a estratégia de embedding mudar.**

> **O que é embedado numa FAQ** (mudou na TAREFA-FAQ-PR, jul/2026): breadcrumb (`origin`) + `P:`
> pergunta + `R:` **resposta**, um por linha, linha de campo vazio omitida. Antes era só breadcrumb +
> pergunta — cego para a resposta, que é onde mora o vocabulário pelo qual o contribuinte pergunta. O
> que o pack **guarda** não mudou (o bloco `## pergunta` / `## resposta`), então o contexto do RAG é o
> mesmo. Na mesma mudança o `max_length` do encoder subiu de **512 para 8192** (o teto do BGE-M3):
> com 512 a resposta seria cortada em silêncio. Texto acima do teto é truncado **com aviso** no
> `auli update`, nomeando os `.md` afetados — no acervo do RS nenhum item chega lá.

> **Re-raspar os serviços de uma entidade**: **`scripts/tools/atualizar-servicos.sh <id>...`**, que faz o
> ciclo inteiro (backup → scrape → process → diff de `raw/` → build-packs) e para na primeira
> entidade que falhar.
>
> O `process` só deriva do **snapshot**, então atualizar exige **re-raspar** — não é derivação
> offline. Depois do `process`, `build-packs.sh <id>` é **obrigatório**: mexer em `docs/` muda o
> `docs_hash` e o boot recusa até os packs baterem. Entidade sem a árvore é simplesmente pulada pelo
> `update` (o fallback para o JSON legado saiu em 02/08/2026, com o `ap`).
>
> Estado (ago/2026): **as 27 entidades têm árvore**. O script nasceu como
> `migrar-arvore-servicos.sh`, para a campanha da TAREFA-SERVICOS-MD; ela terminou, o ciclo ficou.
>
> Vale comparar os artefatos de `raw/` com os de antes: se mudarem, o **portal** mudou (e não a
> migração). Nas oito primeiras entidades migradas apareceram serviço novo (MG, CE), serviço
> removido (PE) e mudança de disponibilidade embutida na descrição (PR — o texto do portal carrega
> "Serviço indisponível no momento" do instante da coleta, então o Auli repete esse status até a
> próxima).
>
> ⚠️ **MG gera churn de nome de arquivo.** O ServiceNow do MG reemite o `sys_id` da URL sem que o
> serviço mude; como o `link` é a identidade (o hash do nome do `.md` vem dele), cada reemissão
> renomeia o documento. Não quebra nada (a árvore é delete + rebuild), mas o MG "troca" documentos a
> cada coleta em que isso acontecer — e links salvos por usuários apodrecem rápido lá.

> **Pareceres / Consultas — pipeline próprio.** Os pareceres (RS) e consultas tributárias (SC/SP/PR)
> têm scraper dedicado **e** um passo de **sinopse por LLM** entre o derive e a vetorização. O fluxo é
> mais longo que o de serviços e tem regras próprias de cota, cache e idempotência: **ver §4.5**.

> Regenerar **sem re-raspar** (ex.: após bump de `STRATEGY_VERSION`): com o snapshot já em disco,
> `auli-collections <id>` re-deriva os contratos offline e `build-packs.sh` regera os packs
> (substitui o antigo subcomando `rebuild`, removido).

> ⚠️ **O cache é sempre lido primeiro — mesmo sem `--usecache`.** Uma página já cacheada **não** é
> re-buscada num novo scrape; `--usecache` apenas transforma um cache-miss em erro (modo offline).
> Para forçar o refetch (novos serviços do portal, conteúdo alterado), **apague o cache antes de
> raspar**: `rm -rf data/<id>/raw/cache/`. No SC isso inclui a listagem paginada e o `buildId`, então
> serviços novos do portal só aparecem depois de limpar o cache.
>
> **Namespace por tipo.** O cache fica em `data/<id>/raw/cache/<kind>/`, um diretório por tipo de
> conteúdo — `servicos/`, `faqs/`, `pareceres/`. Cada caller do kit declara o seu:
> `auli_scraper_kit::cache::{read,write,read_or_bail}(data_dir, kind, url, …)`. Assim uma entidade que
> raspa mais de um tipo (o RS raspa os três) não mistura nem colide. Para limpar só um tipo:
> `rm -rf data/<id>/raw/cache/pareceres/`.

### 4.2 Entidades (`config/registry.toml`)

A lista de entidades e o caminho do prompt de cada uma vêm do **registro único**
`config/registry.toml` (não há mais symlink `./entities`). O system prompt é lido de
`config/prompts/<id>.txt`. Adicionar um estado = uma entrada `[[entities]]` no registry + os dados em
`data/<id>/`.

### 4.3 Modelo (`<raiz>/models`)

Cache do BGE-M3. Os lançadores definem `EMBED_CACHE_DIR=<raiz>/models` (caminho **absoluto**,
CWD-independente — fonte única). Baixa do Hugging Face no 1º uso; depois é reaproveitado (sem rede).

> **⚠️ Execução manual: exporte `EMBED_CACHE_DIR` absoluto.** O default do código é `./models`,
> **relativo ao CWD**. Um `cargo run` avulso disparado de dentro de `auli-server/` (ou de
> `auli-server/crates/`) não acha o cache, re-baixa os 560 MB e os deixa num `models/` órfão ali —
> em silêncio, sem erro. Em 2026-07-25 a faxina encontrou **duas** cópias assim (1,1 GB), criadas em
> 05/07 e 20/07. O mesmo vale para `AULI_DATA_DIR` (default `./data`), que cria árvores `data/`
> vazias fora do lugar. Se for rodar um binário na mão, use os lançadores ou exporte os dois
> caminhos absolutos.

### 4.4 `.env` (raiz do repo `auli/`)

Carregado via `dotenv` a partir do CWD para cima — por isso um `.env` na raiz do repo (`auli/`) serve
para o server rodando em `auli-server/`. Variáveis:

| Variável                                        | Obrigatória?                                                 | Uso                                             |
| ----------------------------------------------- | ------------------------------------------------------------ | ----------------------------------------------- |
| `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` | sim                                                          | LLM externo (Groq-compat) que redige a resposta |
| `EMBED_CACHE_DIR`                               | não (lançadores: `<raiz>/models`; def. do código `./models`) | cache do modelo                                 |
| `EMBED_THREADS`                                 | não (def. `16`)                                              | threads do ONNX Runtime                         |
| `AULI_LOG_DIR`                                  | não (lançadores: `<raiz>/logs`; def. do código `./logs`)     | dir dos logs de Q&A do RAG (§7)                 |

> Faltando uma variável **obrigatória**, o server dá `panic` no boot com mensagem clara.
>
> O server **não tem auth nem banco**: só precisa das variáveis de LLM + embedding acima. Não há
> `DATABASE_URL`, chaves `JWT_*` nem Postgres no boot.

---

### 4.5 Jurisprudência — pipeline completo (scrape → [sinopse] → vetorizar)

Duas coleções, **pareceres** e **tarf**, com o mesmo formato de `.md` e o mesmo caminho de
vetorização desde ago/2026.

**Pareceres / consultas formais** — 4 entidades: **rs** (Pareceres), **sc** (Consultas COPAT),
**sp** (Respostas a Consultas), **pr** (Consultas SEFA). São **3 passos**: o scraper emite a árvore,
a sinopse a preenche, o build vetoriza.

```text
auli-scraper-<id> pareceres      → docs/pareceres/<slug>.md   (rede; um .md por consulta INÉDITA,
                                                               nasce pendente = sem `## resumo`)
auli-collections <id> sinopse    → edita os .md               (LLM; preenche os pendentes)
scripts/build-packs.sh <id>      → packs/<id>-pareceres.json     (embedding; lê a árvore)
                                 → raw/<id>-pareceres-index.json (índice leve da tab; idem)
```

**Acórdãos do TARF** — só **rs**. São **2 passos**: não há sinopse por LLM.

```text
auli-scraper-rs tarf             → docs/tarf/<slug>.md        (rede; um .md por acórdão INÉDITO,
                                                               já nasce COM `## resumo`)
scripts/build-packs.sh rs        → packs/rs-tarf.json + raw/rs-tarf-index.json
```

O `## resumo` do acórdão é a **fundamentação que o próprio documento traz no cabeçalho**, extraída
na coleta; sem fundamentação, o fallback é a ementa. Por isso a coleção **não depende de passo LLM**
— e por isso o `auli-collections <id> sinopse` **não serve para ela**: aquele passo varre só
`docs/pareceres/`. Um acórdão sem `## resumo` é defeito de parser, não pendência a preencher, e o
`auli update` recusa a vetorização dizendo exatamente isso.

**Um acervo, dois consumidores.** O pack serve o RAG (vetores + conteúdo lido tarde); o índice serve
a tab do frontend (numero/assunto/resumo/link, **sem corpo**, copiado para `public/` pelo
`build-frontend-public.sh`). O `build-packs.sh` deriva os dois da mesma leitura da árvore, de
propósito: ele já é inevitável depois de qualquer mexida nela (o `docs_hash` muda e o boot recusa até
ele rodar), então servidor e frontend não têm como divergir de estado. Ele deriva o índice de **toda**
coleção de jurisprudência presente — `pareceres` e `tarf`.

`auli-collections <id> indice [pareceres|tarf]` existe como subcomando avulso — derivação pura,
re-rodar nunca faz mal — mas no fluxo normal não precisa ser chamado à mão.

> **Estado da coleta do TARF (08/08/2026): 3.800 de ~22.522.** Pausada de propósito, para não mover a
> árvore enquanto o código evoluía. Retomável (`./target/debug/auli-scraper-rs tarf`, ~5 h a 1 req/s;
> o cache dos corpos já baixados é reaproveitado). Ao fechar: `build-packs.sh rs`, remover a nota de
> "coleta em andamento" do `bundle.rs` e da `TarfList.tsx`, e publicar. Registro da fonte e das
> exceções conhecidas: `docs/REGISTRO-scrapers.md#tarf`.

> **A árvore `.md` É a fonte** (G5b). Não há mais `.txt` intermediário, promoção manual, JSON de
> contrato nem passo de materialização — o `auli update` lê `docs/<kind>/*.md` direto, nas quatro
> coleções.

**Incremental de graça:** o scraper grava **só o que é inédito** — arquivo que já existe é pulado,
sem sequer ser aberto. Re-raspar é barato e **nunca** destrói uma sinopse já gerada. O corolário é
que **correção de conteúdo não chega sozinha**: se o portal corrigir o corpo de uma consulta já
coletada, é preciso apagar o `.md` (decisão humana, porque isso descarta a sinopse) e recoletar.

> ⚠️ **Um scrape mexe direto na árvore de serving.** Como só acrescenta, uma coleta ruim adiciona
> lixo mas não destrói acervo. Mas o `docs_hash` muda na hora — o boot passa a recusar até o próximo
> `build-packs.sh`. Não reinicie o servidor entre o scrape e o build.

**Bootstrap a partir de `.txt` legado:** `auli-collections <id> pareceres` reconstrói a árvore a
partir de um `ref/<id>-portal-pareceres.txt` antigo (mesma regra: só o inédito). Serve para acervos
coletados antes da G5; no fluxo normal não é usado.

#### Por que existe o passo `sinopse`

A **ementa oficial** do parecer é curta e em juridiquês denso — péssima chave de busca. O `sinopse`
faz **uma** passada de LLM por documento (o corpo é imutável, então é one-shot) gerando uma
**descrição em linguagem natural + palavras-chave**, que passam a ser o texto vetorizado. Resultado
medido: perguntas em linguagem natural passam a recuperar as consultas certas (ver §4.5.4).

#### 4.5.1 Passo 1 — Raspar (emite a árvore)

```bash
cd auli-server
./target/release/auli-scraper-<id> pareceres        # rede; sem --usecache = dados frescos
```

- Grava **um `.md` por consulta inédita** em `data/<id>/docs/pareceres/<slug>.md` (frontmatter
  `numero`/`assunto`/`link` + `## corpo`), **sem sinopse** — o documento nasce pendente.
- **Existe ⇒ pula**, sem abrir o arquivo. É o incremental, e é o que protege sinopses já geradas.
- Relatório ao fim: `N novo(s), M já existente(s) (pulados)`.
- **Colisão de slug é erro**: dois `numero` distintos que gerariam o mesmo arquivo abortam a coleta
  nomeando os dois. Sem isso o segundo sumiria em silêncio (o produtor o veria como "já existe").
- Cada scraper tem uma **guarda de truncamento** (ex.: RS aborta com < 250 consultas) para não
  gravar a partir de uma coleta parcial.
- Cache em `data/<id>/raw/cache/pareceres/` (namespace por tipo; ver o aviso de cache em §4.1).
- O RS exige **`curl`** no PATH (paginação por postback ASP.NET atrás de WAF) e usa UA
  institucional com cortesia de 500 ms entre requisições.

#### 4.5.2 Passo 2 — Sinopse (LLM)

```bash
./target/release/auli-collections <id> sinopse [flags]
```

| Flag               | Efeito                                                                    |
| ------------------ | ------------------------------------------------------------------------- |
| `--dry-run`        | conta pendentes e estima tokens de entrada; **não escreve nada**          |
| `--limit N`        | processa no máximo N documentos nesta rodada (gerados + falhas) — batches |
| `--force <numero>` | re-gera a sinopse de um documento específico (ignora a existente)         |
| `--fake`           | dev-only: preenche resumo sintético, sem tocar rede                       |

**Fonte e destino são os `.md`**: pendente = arquivo **sem a seção `## resumo`**. O passo lê cada
documento, gera o que falta e **regrava o próprio `.md`** (frontmatter `resumo_*` + seção),
atomicamente. Exige a árvore já no disco — quem a cria é o scraper (passo 1); sem ela, erro claro.

- **Env**: `SINOPSE_API_URL` / `SINOPSE_API_KEY` / `SINOPSE_API_MODEL`, com **fallback** para os
  `LLM_*`. Isso permite apontar o lote para um **projeto/quota dedicado**, sem competir com o chat do
  RAG.
- **Prompt**: `config/prompts/sinopse.txt` (versão gravada em `ResumoInfo.prompt_versao`;
  `SINOPSE_PROMPT_VERSION = 1`). Saída validada: as duas seções
  `### Descrição Resumida do Assunto` + `### Palavras Chave do Tema`, na ordem, descrição ≤ 2000
  chars e ≥ 3 palavras-chave. Falha de validação → **1 re-tentativa**; persistindo, conta como falha
  e o lote segue.
- **Entrada truncada** em `CORPO_MAX_CHARS = 24_000` chars (v1 sem chunking; avisa no log).
- **Idempotente**: documento que já tem `## resumo` é **pulado** (zero chamadas). Re-rodar é seguro
  e barato; é assim que se retoma um lote interrompido — a retomada é implícita, sem estado externo.
- **Grava documento a documento** (proteção contra queda): escrita atômica (`.tmp` + rename) no
  próprio `.md`. Uma queda perde no máximo o documento em voo.
- **Memória constante**: processa um a um (lê → gera → grava), nunca carrega a árvore inteira — SP
  tem 15,6 mil arquivos.
- **Relatório final** com invariante de guarda:
  `reaproveitados + gerados + falhas + pendentes-restantes == total`.

##### Cota / rate limit (o ponto operacional mais importante)

O provedor (Groq-compat) impõe **RPD** (requisições/dia) por projeto. Duas defesas, em camadas:

1. **Proactive-stop (primário, zero rejeição).** A cada resposta o cliente lê
   `x-ratelimit-remaining-requests`. Quando o headroom cai a `≤ RPD_MARGEM_PARADA (5)`, o lote **para
   antes** de mandar a requisição que seria rejeitada, e loga o `x-ratelimit-reset-requests` (quando a
   cota volta). A margem não é 0 de propósito: reserva para o chat do RAG, que divide a mesma quota.
2. **Early-abort (rede de segurança).** Se o header não vier e um `429 rate_limit_exceeded` chegar, o
   lote aborta no **primeiro** — em vez de disparar centenas de requisições condenadas.

> ⚠️ **Requisição rejeitada também consome RPD.** Antes dessas defesas, um lote que batia no teto no
> meio seguia disparando e queimava a cota do dia inteiro em 429s — o dia rendia pouquíssimas
> sinopses. Hoje isso não acontece: pior caso é **1** rejeição-probe.

Nos dois casos o documento que parou e os restantes ficam em `pendentes-restantes` (**não** contam
como falha) — basta **re-rodar** depois do reset (ou com o teto elevado) que o merge reaproveita tudo.

Para inspecionar a cota sem rodar o lote:

```bash
set -a; . .env; set +a
curl -sS -D - -o /dev/null -X POST "$LLM_API_URL" \
  -H "Authorization: Bearer $LLM_API_KEY" -H "Content-Type: application/json" \
  -d "{\"model\":\"$LLM_API_MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"max_completion_tokens\":1}" \
  | grep -i x-ratelimit
```

##### Acervos grandes: batches de 1000

Para entidades com milhares de consultas (SP tem **15.605**), rode em batches e audite no caminho:

```bash
./target/release/auli-collections sp sinopse --limit 1000   # repita até pendentes-restantes = 0
```

Cada rodada reaproveita as prontas e ataca as próximas 1000; o proactive-stop encerra antes se a cota
acabar. Dá para automatizar com um driver que repete `--limit 1000`, detecta o proactive-stop no log,
dorme até o reset e retoma — com teto de iterações e abort se um batch não progredir (evita loop
infinito).

#### 4.5.3 Passo 3 — Vetorizar

```bash
scripts/build-packs.sh <id>
```

- **A fonte é a árvore** (G5b): o `auli update` lê `docs/pareceres/*.md` em ordem de nome de arquivo
  (estável, para os `id-N` do pack não dançarem entre rodadas) e monta o registro dali — frontmatter,
  `## resumo` e `## corpo`. Não há JSON de pareceres no caminho.
- **Guarda de ingestão:** **recusa** vetorizar se **qualquer** documento estiver sem `## resumo`,
  listando os números. A árvore em si segue válida — pendência é estado legal dela; só a vetorização
  é barrada.
- **O que é embedado** (`text_to_embed`): `numero` + `assunto` + `resumo` (a sinopse), unidos por
  quebra de linha, vazios pulados. O **corpo integral NÃO é embedado**.
- **O que o pack guarda** (mudou na G3): não é mais o bloco pronto com o corpo, e sim um **payload
  leve** — `numero`/`assunto`/`resumo`/`link`/`doc_path`. O corpo é lido da árvore **na query**, só
  para os documentos selecionados. Se um `.md` sumir, o servidor **degrada** (serve a sinopse com o
  aviso `[corpo indisponível — ver link]` e loga `ERROR`) em vez de derrubar a consulta.
- **`STRATEGY_VERSION`** (hoje `1`, numeração **reiniciada** na TAREFA-FAQ-PR — a série anterior foi
  até `4` e virou nota histórica no doc-comment da constante) é carimbado no manifesto e validado no
  boot: pacote com versão diferente ⇒ o server **recusa subir**. Regenerar sinopses **em massa**
  (mudança de `SINOPSE_PROMPT_VERSION` ou troca do modelo + re-geração) muda os textos embedados ⇒ **bump
  obrigatório**. Sinopses novas convivendo com antigas (append-only) **não** exigem bump.

  > ⚠️ **O bump é global e o boot é fatal para o servidor inteiro.** `packs::load_all` valida
  > **todas** as entidades; a primeira com versão antiga derruba o boot. Ao bumpar, re-rode
  > `build-packs.sh` em **todas** as entidades antes de subir o servidor — não dá para migrar uma de
  > cada vez com o serviço no ar.

#### 4.5.4 Validar o retrieval (sem LLM)

```bash
EMBED_CACHE_DIR=$PWD/models \
  cargo run --release -p auli-cli --example retrieval_test -- data/<id>/packs/<id>-pareceres.json
```

Carrega o embedder + o pack e imprime o **top-5 por proximidade** de um conjunto de perguntas em
linguagem natural (score = distância; **menor = mais próximo**). É o jeito rápido de confirmar que a
key nova está funcionando, sem gastar quota de LLM.

#### 4.5.5 Estado atual

| Entidade | Consultas | Sinopses | Observação                                       |
| -------- | --------- | -------- | ------------------------------------------------ |
| **rs**   | 372       | 372      | —                                                |
| **sc**   | 1.743     | 1.743    | —                                                |
| **pr**   | 2.060     | 2.060    | —                                                |
| **sp**   | 15.605    | 15.605   | fechado em ~7 h de lote; use `--limit` (§4.5.2)   |

Todas as 4 completas. As sinopses vivem em `data/<id>/docs/pareceres/*.md` — **é essa árvore que
precisa de backup**, não o `.txt`.

#### 4.5.6 Trabalhar com um lote em curso — e o que acontece se a máquina cair

Um lote de sinopse roda por **horas** (o SP são ~15 mil documentos). Não é preciso ficar parado: dá
para desenvolver e até rodar o pipeline de **outras** entidades em paralelo, desde que se respeite o
que o lote está usando.

##### O que o lote está usando

| Recurso                                                 | Em uso pelo lote?                        | Consequência                                                    |
| ------------------------------------------------------- | ---------------------------------------- | --------------------------------------------------------------- |
| `target/release/auli-collections`                       | **sim** — o driver o invoca a cada batch | não pode ser substituído: o próximo batch rodaria outro binário |
| `target/release/auli`                                   | só se houver `auli server` no ar         | livre para rebuild/execução quando o server está parado         |
| `data/<id>/raw/<id>-pareceres.json` da entidade em lote | **sim**, reescrito a cada documento      | não ler nem regenerar essa entidade                             |
| `data/<outra>/**`                                       | não                                      | **disjunto** — outras entidades são seguras                     |

##### Regras para buildar sem derrubar o lote

- **Escopar o build**: `cargo build --release -p <pacote>` produz **apenas** o binário daquele
  pacote. As libs compartilhadas (`auli-contract`, `auli-core`) viram `.rlib` novos, mas um
  executável já linkado é estático — o processo em execução e o arquivo no disco **não mudam**.
- **Nunca** `cargo build --release` do workspace inteiro enquanto um lote roda: isso relinka
  **todos** os binários, inclusive o que está em uso.
- Testes/clippy em **perfil debug** (`cargo test -p <pacote>`) nunca encostam em `target/release/`.
- Alternativa de risco zero, ao custo de um rebuild do zero (fastembed/ort/aws-lc, alguns minutos):
  `CARGO_TARGET_DIR=/tmp/target-paralelo cargo build --release …`.

##### Regra das entidades

Os dados são **disjuntos por entidade** (`data/<id>/`). Rodar `auli update --entity rs` ou
`scripts/build-packs.sh rs` **não toca em nada** do SP. A regra é simples: **mexa em qualquer
entidade exceto a que está em lote**.

##### Se a máquina cair no meio de um lote de sinopse

**Nada a reconciliar — o passo foi desenhado para isso.**

- O `sinopse` grava **atomicamente por documento** (`.tmp` + rename): tudo que já foi gerado está no
  disco. Perde-se, no máximo, o documento em voo.
- O driver de batches **não tem estado próprio**: ele deriva "quantos faltam" do próprio JSON. Não há
  lock, arquivo de progresso nem transação pendente para limpar.
- **Recuperação:** relançar o driver (ou rodar `auli-collections <id> sinopse --limit N` à mão). O
  merge por `numero` reaproveita todos os prontos e ataca só os pendentes.

Para ver onde parou:

```bash
python3 -c "import json;d=json.load(open('data/<id>/raw/<id>-pareceres.json'));i=next(v for v in d.values() if isinstance(v,list));c=sum(1 for r in i if (r.get('resumo') or '').strip());print(f'{c}/{len(i)} prontas, {len(i)-c} pendentes')"
```

##### Se cair durante `auli update` / `build-packs.sh`

O pack pode ficar truncado — mas isso **não vira serviço quebrado**: a validação do manifesto no boot
(identidade de embedding + hash de integridade) faz o server **recusar subir** em vez de responder a
partir de um pacote corrompido. Recuperação: rodar `scripts/build-packs.sh <id>` de novo (idempotente).

##### Se cair com trabalho de código não commitado

O de sempre: só sobrevive o que está commitado (e, de preferência, pushado). Com um lote longo em
paralelo, a tentação é acumular mudanças por horas — commite cedo e com frequência.

---

## 5. Subir o servidor + túnel Cloudflare

O jeito recomendado é o script [start_server.sh](start_server.sh) (na raiz do repo `auli/`):

```bash
./start_server.sh                       # build incremental + server + túnel
./start_server.sh --no-build            # pula o cargo build (restart rápido) + túnel
./start_server.sh --no-tunnel            # só o servidor local, sem túnel
./start_server.sh --no-build --no-tunnel # restart local puro
```

O que ele faz: exporta o env de cmake desta máquina, reusa `auli-server/target`, entra em `auli-server/`,
exporta `AULI_DATA_DIR=../data`, derruba uma instância anterior na porta, compila (a menos de
`--no-build`), sobe o **túnel cloudflared em background** e o **server em foreground**. **Ctrl+C**
encerra os dois (um `trap` derruba o cloudflared junto). O túnel precisa ter sido configurado 1× com
`./setup-cloudflared.sh` (ver §10); sem isso, sobe só o servidor local.

**Escuta só em loopback** (`BIND=127.0.0.1`, default desde 09/08/2026). O cloudflared roda nesta
mesma máquina e seu ingress aponta para `http://localhost:3000`, então nada precisa alcançar a porta
pela rede. Com o `0.0.0.0` de antes, qualquer máquina da rede local falava com a API **contornando o
Cloudflare** — e o rate limiting junto, que mora lá (§10.2). Para expor de propósito — testar de um
celular, reverse proxy em outra máquina — `BIND=0.0.0.0 ./start_server.sh`.

### Liberar o terminal

O server é foreground por desenho: é o que permite ao `trap` derrubar o cloudflared no Ctrl+C. Para
recuperar o prompt:

```bash
nohup ./start_server.sh > /tmp/auli-server.log 2>&1 &
tail -f /tmp/auli-server.log     # acompanhar o boot; Ctrl+C sai do tail, não do servidor
```

Para derrubar depois, **`kill` no PID** (o que o `&` imprimiu, ou `pgrep -f start_server.sh`) — não
`kill -9`: o `trap` só roda com sinal capturável, e sem ele o cloudflared fica órfão segurando o
túnel. Se acontecer, `pkill cloudflared`.

Variáveis de ambiente para sobrescrever:

```bash
PORT=8080 ./start_server.sh                       # outra porta
AULI_DATA_DIR=/dados ./start_server.sh            # outra raiz de data/ (o catálogo vira /config)
TUNNEL_NAME=outro-tunel ./start_server.sh         # outro túnel cloudflared
./start_server.sh --no-tunnel                     # só o servidor local, sem túnel
```

**Boot saudável** se parecer com (5 entidades ativas):

```text
🏛️  Entidades carregadas: [mg, pr, rs, sc, sp]
🔎 Manifesto de 'rs' validado contra a identidade local.
📦 rs-servicos — 586 registros
📦 rs-faqs — 1937 registros
🔎 Manifesto de 'sp' validado contra a identidade local.
📦 sp-servicos — 537 registros
🔎 Manifesto de 'pr' validado contra a identidade local.
📦 pr-servicos — 141 registros
🔎 Manifesto de 'sc' validado contra a identidade local.
📦 sc-servicos — 208 registros
🔎 Manifesto de 'mg' validado contra a identidade local.
📦 mg-servicos — 148 registros
✅ Server started successfully at 127.0.0.1:3000
```

### Sem o script (comando direto)

```bash
cd /home/ubu/Desktop/auli/auli-server
AULI_DATA_DIR=../data target/release/auli server --packs-dir ../data
```

> **Nunca use `sudo`.** A porta 3000 não exige root, e sob `sudo` o server procura `.env`/cache no
> HOME do root e não acha os dados.

---

## 6. Smoke tests

Com o server no ar, em outro terminal:

```bash
curl -s localhost:3000/v1/health
curl -s -X POST localhost:3000/v1/question -H 'Content-Type: application/json' \
  -d '{"entity":"rs","question":"Como obtenho certidão negativa de débitos?"}'
# entidade desconhecida -> erro amigável, sem panic, HTTP 200:
curl -s -X POST localhost:3000/v1/question -H 'Content-Type: application/json' \
  -d '{"entity":"zz","question":"x"}'

# Retrieval PURO (sem LLM): devolve os documentos com score, sem redigir resposta.
curl -s localhost:3000/v1/retrieve -H 'Content-Type: application/json' \
  -d '{"question":"crédito de ICMS na aquisição de energia elétrica","entity":"sc","kind":"pareceres","top_k":5}' \
  | python3 -m json.tool
# kind inválido -> 400; entidade inválida -> 404; entidade sem acervo -> 200 com arrays vazios.
# kinds válidos: servicos | faqs | pareceres | tarf | notas.

# As QUATRO coleções do rs, e a checagem que importa depois de mexer na árvore ou nos packs:
# nenhum documento pode voltar degradado ("indisponível") — isso significaria doc_path que não resolve.
for k in servicos faqs pareceres tarf; do
  printf '%-10s ' "$k"
  curl -s localhost:3000/v1/retrieve -H 'Content-Type: application/json' \
    -d "{\"entity\":\"rs\",\"kind\":\"$k\",\"question\":\"parcelamento de débito\",\"top_k\":3}" \
    | python3 -c "import sys,json; d=json.load(sys.stdin); i=d['pareceres'] or d['hits']; \
      print(len(i),'hits · degradados:',sum('indisponível' in json.dumps(h,ensure_ascii=False) for h in i))"
done

# MCP (protocolo completo: initialize -> initialized -> tools/list -> tools/call):
./scripts/tools/mcp-smoke.sh http://localhost:3000/mcp
```

### 6.1 Paridade do contexto RAG (após mexer na recuperação)

Depois de qualquer mudança no caminho de recuperação — o motor, os knobs `*_FLOOR`/`*_BAND`, a
montagem do contexto — vale conferir que **os documentos recuperados e a ordem deles** não
mudaram. Nenhum teste unitário cobre isso: os de `montar_rag_*` pinam o formato e os de
`bloco_parecer` pinam a leitura do corpo, mas o conjunto/ordem só aparece em consulta real.

O `scripts/tools/parity-replay.py` reenvia as perguntas dos logs de auditoria já existentes e compara o
`CONTEXTO RAG` que o próprio servidor grava:

```bash
# 1. servidor local com os logs num diretório À PARTE (não misture com ./logs, que é o baseline)
AULI_DATA_DIR=./data EMBED_CACHE_DIR=./models AULI_LOG_DIR=/tmp/parity-logs \
  ./auli-server/target/release/auli server --port 3111 --bind 127.0.0.1

# 2. compara (o -u mostra progresso ao vivo; sem ele o Python bufferiza e parece travado)
python3 -u scripts/tools/parity-replay.py logs /tmp/parity-logs http://localhost:3111
```

O 2º argumento **tem que ser o mesmo `AULI_LOG_DIR`** do servidor — é lá que o script procura o
log recém-gravado. Exit 0 = tudo idêntico; 1 = divergência, com o primeiro diff impresso.

> **Custo:** uma chamada de LLM por log (o log de auditoria só é gravado depois que o modelo
> responde). Rode contra um servidor **local**, nunca o de produção. A resposta do modelo é
> descartada — ela varia com a temperatura; o contexto, não.

Usado no G2 (extração do motor `auli-retrieval`) sobre **40 consultas reais** — 16 `pareceres` +
24 `servicos+faqs`, entidades rs/sp/sc, contextos de 2,8 KB a 100 KB: **40 idênticos, 0 divergentes**.

---

## 7. Logs — onde ficam

Há **três** destinos distintos:

| Tipo                    | Onde                                                                   | Conteúdo                                                                                                                                                                                                                |
| ----------------------- | ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Q&A do RAG** ⚠️ PII    | `logs/<AAAA-MM-DD_HH-MM-SS>_<uuid>.txt` na raiz do repo (um arquivo por pergunta) | Cinco seções: `PERGUNTA (ORIGINAL)` — **texto cru, com PII** —, `PERGUNTA (ANONIMIZADA)`, `RESPOSTA` (**restaurada**, PII de volta), `ADERÊNCIA` (ver §7.2) e `CONTEXTO RAG`. Ver **§7.0**. Gravado por [rag.rs](auli-server/crates/auli-cli/src/rag.rs) em `$AULI_LOG_DIR` (default `./logs` do CWD). O `start_server.sh` exporta `AULI_LOG_DIR=<raiz>/logs` (absoluto), então caem na **raiz do repo**, não em `auli-server/`. |
| **Chamadas MCP** ⚠️ PII  | o **mesmo** `logs/…` acima                                              | Mesmo formato, **sem a seção `RESPOSTA`** (não há LLM neste caminho) e com `tipo: mcp:<ferramenta>` no cabeçalho. Só as três ferramentas que recebem pergunta; `listar_entidades` não gera registro. O `obter_parecer` também não traz `ADERÊNCIA` — acha pelo número, não por vetor. Ver §7.0, §7.2 e §12. |
| **cloudflared**         | `/tmp/auli-cloudflared.log`                                            | saída do túnel Cloudflare (redirecionada pelo `start_server.sh`).                                                                                                                                                       |
| **Console (`tracing`)** | **stdout/stderr** do terminal (não vai a arquivo)                      | boot, scores, `info/debug/warn`. Controlado por `RUST_LOG`.                                                                                                                                                             |

Exemplos:

```bash
ls -lt logs/ | head                             # últimas consultas (raiz do repo)
tail -f /tmp/auli-cloudflared.log              # acompanhar o túnel
RUST_LOG=auli_cli=debug ./start_server.sh   # ver arrays de score + prompt RAG completo no console
```

> Para gravar também o console em arquivo: `./start_server.sh --no-tunnel 2>&1 | tee logs/console.log`.

### 7.0 ⚠️ `logs/` contém PII — trate como dado pessoal

O log de Q&A é **deliberadamente íntegro**: a seção `PERGUNTA (ORIGINAL)` guarda o texto cru do
usuário e a `RESPOSTA` guarda a resposta já **restaurada** (os placeholders `[CNPJ_1]` trocados de
volta pelos valores reais). Isso é escolha, não descuido — sem o par original/anonimizada lado a
lado não há como auditar o que o `auli-anon` deixou passar. A doutrina está em `format_log_record`
([rag.rs](auli-server/crates/auli-cli/src/rag.rs)); ela **revisa** o §4 do plano do `auli-anon`, que
mandava persistir só o texto anonimizado.

O que o `auli-anon` protege é outra fronteira, e segue valendo: o **LLM externo** só vê placeholders
e o **stdout em nível `info`** só mostra a pergunta anonimizada. O disco não.

Controles compensatórios:

| Controle | Estado |
| --- | --- |
| Permissão | `logs/` **0700**, arquivos **0600** — reaplicado a cada gravação pelo `log_question`, então um diretório herdado com modo frouxo se conserta sozinho. |
| Versionamento | `**/logs/` está no `.gitignore` — nunca vai ao GitHub. Confira com `git check-ignore -v logs/`. |
| Retenção | **manual** (ver abaixo). Não há expurgo automático. ⚠️ **Desde 2026-08-02 o MCP também grava**, e o regime de volume é outro: o chat é ~1 arquivo por interação humana, enquanto um assistente faz **várias** chamadas por conversa. Reavaliar a periodicidade do expurgo — a doutrina de retenção não mudou, o volume sim. |
| Cópia/compartilhamento | Quem lê o diretório lê PII. Não colar trecho de log em issue, PR ou chat sem revisar. |
| Leitura pela rede | `GET /v1/log/{uuid}` (desde 09/08/2026) serve **um** registro por vez a quem tem o UUID dele — ver 7.0.1. Não existe rota que liste, conte ou enumere. |

#### 7.0.1 O acesso pela rota, e por que ele não afrouxa o 7.0

O ícone do chat abre o log **daquela** resposta. A porta é uma *capability*: o UUID v7 nasce no
`log_question`, volta no campo `log_id` da resposta e, portanto, só chega a quem fez a pergunta.
Sem ele não há caminho — não há listagem, e adivinhar esbarra em 74 bits de aleatoriedade.

A premissa do 7.0 continua de pé porque **o que mudou foi o alcance, não o conteúdo**: antes o log
só era legível por quem tinha o filesystem da máquina; agora também por quem possui o UUID de uma
consulta. Cada um lê apenas o que ele mesmo digitou, mais mecânica sobre acervo público (contexto
RAG, distâncias, tempos). O registro sai **como está**, incluindo o par original/anonimizada — ver o
mascaramento funcionando é prova de privacidade para o autor, não vazamento.

Três consequências operacionais:

- **os arquivos são servidos da máquina do Desktop Server**, um por vez e sob demanda; nada é
  copiado para o VPS, e o deploy do estático não muda;
- **logs anteriores a 09/08/2026 não têm UUID no nome e são inalcançáveis pela rota** — sem
  migração retroativa, por decisão (D-LOG-1);
- o **expurgo** abaixo segue valendo tal como está: um log podado responde `404` com a mesma
  mensagem de um que nunca existiu, de propósito.

Retenção — o expurgo é seu, e não roda sozinho:

```bash
find logs -name '*.txt' -mtime +90 -print          # o que sairia (confira ANTES)
find logs -name '*.txt' -mtime +90 -delete         # expurga o que passou de 90 dias
```

Para medir a exposição sem imprimir nada (útil antes de compartilhar a pasta):

```bash
grep -lE '[0-9]{3}\.[0-9]{3}\.[0-9]{3}-[0-9]{2}' logs/*.txt | wc -l   # arquivos com CPF
grep -lE '\[(CPF|CNPJ)_[0-9]+\]' logs/*.txt | wc -l                   # arquivos onde o anon atuou
```

### 7.1 Latência por fase (linha `TEMPOS`)

Cada Q&A do chat grava, no cabeçalho do log, os quatro tempos da consulta — `embed` (vetorizar a
pergunta, CPU), `retrieve+montagem` (varredura + leitura dos corpos + montagem do contexto), `llm`
(a chamada externa) e `total`:

```bash
grep -h "^TEMPOS" logs/*.txt        # a série temporal, uma linha por consulta
```

Exemplo real (pareceres, RS): `embed: 18 ms · retrieve+montagem: 0 ms · llm: 3487 ms · total: 3508
ms` — a chamada externa domina (~99%), como esperado. **As fases não somam o total**: `total`
cobre também a cola não cronometrada (anonimização, resolução de entidade, montagem do prompt,
restauração). O `embed_ms` medido aqui, em produção, é o número que decide o `--device cuda` do
server (ver TAREFA-GPU): 18 ms de CPU sobre 3,5 s de resposta torna o ganho de GPU imperceptível.

A mesma medição em campo estruturado sai no console (`tracing`): `info` "tempos da consulta" com
`embed_ms`/`retrieve_ms`/`llm_ms`/`total_ms` separados, para filtrar/agregar no journal. As faces
`/v1/retrieve` e `/mcp` logam um `ms` único (só o embed+scan; não chamam LLM).

### 7.2 Aderência por documento (seção `ADERÊNCIA`)

Toda consulta que passa por busca vetorial grava, logo antes do `CONTEXTO RAG`, **o quão perto da
pergunta ficou cada documento escolhido** — na mesma ordem e com o mesmo número do bloco RAG:

```text
----- ADERÊNCIA (proximidade da pergunta) ----------------------
servico 1 · aderência 0.656 · distância 0.344
...
faq 1 · aderência 0.744 · distância 0.256
```

São **duas leituras do mesmo número**. `distância` é a distância cosseno crua que o
`vector-store` calcula: **0 = idêntico**, e quanto menor, mais perto. `aderência` é `1 − distância`,
para ler na direção intuitiva (maior = mais aderente). As duas aparecem porque servem a coisas
diferentes:

- a **aderência** responde "este documento tinha o que ver com a pergunta?";
- a **distância** é a unidade dos knobs `SVC_BAND`/`FAQ_BAND`/`PAR_BAND` em
  [rag.rs](auli-server/crates/auli-cli/src/rag.rs) — a banda se calibra lendo esta coluna, sem
  converter nada de cabeça (ver o comentário dos knobs, que já mandava "rodar perguntas reais e
  ler os scores").

Duas ressalvas de leitura:

- **Não é probabilidade nem porcentagem de acerto.** É proximidade no espaço do BGE-M3. Em corpus
  real os valores costumam ficar entre 0,55 e 0,75 — pouca dispersão, então o que informa é a
  **queda relativa** dentro da lista, não o valor absoluto.
- **A busca sempre devolve os mais próximos que existirem.** Uma lista inteira em ~0,55 sem nada
  se destacando é o sinal típico de "não há documento sobre isso" — e é exatamente esse caso que a
  seção torna visível no log.

Pareceres trazidos pela **expansão por grafo** (co-citação de dispositivos) aparecem como
`parecer relacionado N · por co-citação de dispositivos (sem busca vetorial)`: não vieram do vetor
e não têm distância — inventar um número ali seria mentir no registro.

A seção vive **só no log**. O bloco que vai ao prompt do LLM não muda um byte: é a mesma string
`rag` nos dois lugares, e os testes `montar_rag_*_pina_o_formato` a travam byte a byte.

```bash
grep -h "aderência" logs/*.txt | sort -t' ' -k4 -n | head   # os piores casos do acervo
```

---

## 8. Parar / reiniciar

- **Parar:** `Ctrl+C` no terminal do server (encerra server + cloudflared pelo `trap`).
- **Matar à força (porta presa):** `pkill -f "release/auli server"`.
- **Reiniciar rápido (sem recompilar):** `./start_server.sh --no-build`.

---

## 9. Troubleshooting

| Sintoma                                                                     | Causa / correção                                                                                                                                                                                                                    |
| --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Não foi possível ler o registro de entidades` / `Entidades carregadas: []` | `AULI_DATA_DIR` não aponta para a pasta `data/`, ou a `config/` irmã não existe. Rode via `start_server.sh` (exporta `../data`); para catálogo fora do layout, use `AULI_CONFIG_DIR`.                                                                                                              |
| `📦 Pacotes carregados de …` (e nada carregado)                             | `--packs-dir`/`AULI_DATA_DIR` aponta para a raiz errada (sem `<id>/packs/`). Sem `--packs-dir`, o server usa `AULI_DATA_DIR` (default `./data`); o `start_server.sh` exporta `../data`.                                             |
| Rebaixando `model_quantized.onnx` toda vez                                  | Cache não encontrado. Os lançadores já apontam `EMBED_CACHE_DIR=<raiz>/models` (absoluto), então o CWD não importa — confira se os pesos estão em `<raiz>/models`. Em execução manual, exporte `EMBED_CACHE_DIR` para esse caminho. |
| Erro de cmake / `aws-lc-sys` no build                                       | Sem cmake no PATH ou cmake 4 reclamando de policy. `export PATH="$HOME/.local/bin:$PATH"` e `export CMAKE_POLICY_VERSION_MINIMUM=3.5` (o `start_server.sh` já faz).                                                                 |
| `Variável de ambiente obrigatória ausente: ...`                             | Falta variável de LLM no `.env` (`LLM_API_URL`/`LLM_API_KEY`/`LLM_API_MODEL`). Ver §4.4.                                                                                                                                            |
| `Manifest incompatível ...` no boot                                         | Pacotes gerados com modelo/dim/`strategy_version` diferente do binário. Re-gere com `auli update`.                                                                                                                                  |
| `Permission denied` ao rodar o script                                       | Faltou `chmod +x start_server.sh`, ou usou `sudo` (não use).                                                                                                                                                                        |
| cloudflared com "connection refused"/"context deadline" no início           | Normal: o túnel tenta conectar enquanto o server ainda carrega o modelo; conecta quando o boot termina.                                                                                                                             |
| `failed to create tunnel ... already exists`                                | O túnel `auli-api` já existe. O `setup-cloudflared.sh` é idempotente; rode de novo (ele detecta e reaproveita).                                                                                                                     |
| `An A, AAAA, or CNAME record with that host already exists`                 | O CNAME do ngrok ainda está em `api.auli.com.br`. O script usa `--overwrite-dns`; se persistir, apague o registro no painel e rode de novo.                                                                                         |
| HTTP 1015 / "rate limited" no cliente                                       | A regra de rate limiting do Cloudflare disparou (§10.2). Ajuste o limite ou o período.                                                                                                                                              |

---

## 10. Túnel Cloudflare + rate limiting

O `api.auli.com.br` é publicado por um **Cloudflare Tunnel** (`cloudflared`) — substitui o ngrok.
O `cloudflared` roda nesta máquina e disca **para fora**, então **o Cloudflare é o único caminho de
entrada** (não há URL pública de origem para burlar) e as regras de rate limiting do Cloudflare são
de fato aplicadas. O `CF-Connecting-IP` chega ao limitador interno do app (1 req/s, burst 2 por IP).

### 10.1 Configuração do túnel (uma vez)

```bash
./setup-cloudflared.sh        # login (abre navegador) + cria o túnel 'auli-api' + roteia o DNS
```

O script: autoriza a conta (`~/.cloudflared/cert.pem`), cria o túnel, escreve
`~/.cloudflared/config.yml` (ingress `api.auli.com.br` -> `http://localhost:3000`) e aponta o
**CNAME proxied** `api.auli.com.br` -> `<uuid>.cfargotunnel.com` (substituindo o CNAME do ngrok).
Depois disso, **`./start_server.sh` sobe o túnel automaticamente** (`cloudflared tunnel run auli-api`).

No painel Cloudflare, **reative a zona `auli.com.br`** (estava em *pause*) e confira que
`api.auli.com.br` está **proxied** (nuvem laranja).

### 10.2 Regra de rate limiting (no painel Cloudflare)

**Security -> WAF -> Rate limiting rules -> Create rule**:

| Campo                          | Valor                                                                                                                               |
| ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| **If incoming requests match** | `Hostname` equals `api.auli.com.br` **AND** `URI Path` equals `/v1/question`                                                        |
| **(opcional)**                 | **AND** `Request Method` equals `POST`                                                                                              |
| **Rate**                       | ex.: **60** requests per **1 minute** (ajuste à realidade de um órgão; lembre que NAT faz um escritório inteiro sair por **um IP**) |
| **Counting characteristic**    | `IP` (com proxy, é o IP real do cliente)                                                                                            |
| **Then**                       | **Block** por **10s**-**60s**, resposta **429** (corpo custom opcional em pt-BR)                                                    |

Defesa em camadas: a regra do Cloudflare é a barreira de borda; o limitador interno do app
(`/v1/question`, por IP) é a segunda linha. Não há mais URL de ngrok pública para contornar.

### 10.3 Verificação

```bash
tail -f /tmp/auli-cloudflared.log                 # túnel: deve logar "Registered tunnel connection"
curl -s https://api.auli.com.br/v1/health         # 200 via Cloudflare
# dispare > limite no /v1/question e espere 429 (vinda do Cloudflare; veja em Security -> Events)
```

---

## 11. Publicar o frontend

O frontend é **estático e independente do servidor da API**: build Vite servido por Apache no VPS,
com Cloudflare na frente. Não fala com nada em build time — as abas de conteúdo leem arquivos do
próprio origin (`/<id>/<id>-<arquivo>`), e só o chat chama a API.

O dado percorre três estágios até a tela — e os zips de download entram pelo segundo, porque o Vite
copia `public/` inteiro para o `dist/`; eles não têm caminho de publicação próprio (é também por
isso que o `dist/` pesa ~253 MB):

```text
data/<id>/{raw,ref}/  →  auli-frontend/public/<id>/  →  auli-frontend/dist/  →  /var/www/html/
      (fonte)          build-frontend-public.sh       npm run build         deploy-frontend.sh
                            + `auli bundle` ⇢ public/downloads/*.zip
```

**Um comando faz os três:**

```bash
scripts/deploy-frontend.sh --dry-run     # roda a parte local inteira; imprime os comandos remotos
scripts/deploy-frontend.sh               # publica
```

A API sobe à parte, pelo `./start_server.sh` (§1 e §5). A ordem de chamada de dentro deste script —
quem chama quem, e onde a operação para — está em [scripts/README.md](../scripts/README.md).

Destino e afins saem de variáveis (`DEPLOY_HOST`, `DEPLOY_PORT`, `WEBROOT`, `SMOKE_HOST`,
`SMOKE_ORIGIN`, `SMOKE_BASE`); os padrões apontam para o VPS de produção. Ver o cabeçalho do script.

> ### A quarta origem: o binário — fechada no passo 1/8
>
> O diagrama acima mostra três estágios e esconde uma **quarta origem**, que não é arquivo do
> repositório e sim um binário compilado:
>
> ```text
> auli-server/crates/auli-cli/src/bundle.rs  →  target/release/auli  →  public/downloads/*.zip
>                                            cargo build --release    deploy-frontend.sh (passo 3/8)
> ```
>
> **Por que isso é uma armadilha.** Até 09/08/2026 o script nunca compilava: o passo dos zips
> invocava o `auli bundle` do `target/release/` como estivesse no disco. Naquele dia os zips saíram
> com a frase antiga (`"Coleta em andamento"` no README do `tarf/`) cinco horas depois de ela ter
> sido removida do código — o binário era anterior ao commit. E saiu em silêncio: o script só
> reclamava quando o binário estava *ausente*, nunca quando estava *velho*. Nenhum teste pega, e não
> é lacuna de cobertura: a suíte roda contra o código-fonte, e o artefato publicado vem do binário.
> **Commit em Rust não muda binário nenhum — e o que sobe é o binário.**
>
> **Hoje o `deploy-frontend.sh` compila sozinho**, no passo 1/8, antes de gerar qualquer artefato.
> Não há mais o que lembrar antes de publicar.
>
> **Quem decide se precisa recompilar é o cargo**, e não uma comparação de datas. Ele resolve por
> fingerprint do grafo inteiro, o que cobre também mudança de dependência no `Cargo.lock` — um bump
> de `ort` altera os vetores sem tocar num `.rs` (§34.2 das pendências), e mtime não veria. Quando já
> está em dia, é um no-op de menos de um segundo.
>
> **A escotilha:** definir `AULI_BIN` pula a compilação. Existe para quem aponta para um binário
> pré-compilado de propósito — e aí manter o binário atual volta a ser responsabilidade de quem
> definiu a variável.
>
> **Fechado em todos os scripts (09/08/2026, mais tarde no mesmo dia).** A guarda começou aqui e se
> espalhou: hoje **todo script que usa `auli`, `auli-collections` ou um `auli-scraper-<id>` ou
> compila antes, ou recebe o caminho pronto de quem compilou**.
>
> O `build-packs.sh` era o mais traiçoeiro dos que faltavam, e por um motivo específico: com binário
> velho ele gravava packs de estratégia antiga com um `docs_hash` que **passa** na validação de boot,
> porque o hash é da árvore e não do código. Vetores errados, nenhum erro em lugar nenhum. O
> `atualizar-servicos.sh` tinha um furo diferente e pior: `cargo build … | tail -1` descartava o
> status, e um build QUEBRADO deixava o ciclo seguir com o binário anterior.
>
> **Conferir o que de fato saiu** continua barato, e é o único jeito de ter certeza:
>
> ```bash
> unzip -p auli-frontend/public/downloads/rs.zip tarf/README.md | head -5
> ```

**O smoke test mede a ORIGEM, não a borda.** Ele conecta direto no IP da VPS (`SMOKE_ORIGIN`,
derivado do `DEPLOY_HOST`) mandando o SNI de `SMOKE_HOST` (`auli.com.br`), via `curl --resolve`. Os
dois motivos:

- **`auli.com.br` está atrás de Cloudflare** (resolve para `2606:4700:…`, não para o IP da VPS).
  Verificar pelo domínio mediria o cache do CDN junto com o servidor — logo depois de um deploy isso
  mente nas duas direções: pode aprovar o que não subiu, ou reprovar o que subiu.
- **O certificado do servidor cobre só `auli.com.br`.** Bater no hostname da VPS por HTTPS falha com
  `curl (60) no alternative certificate subject name matches`. Era o que o `SMOKE_INSECURE=1`
  contornava, ao custo de deixar o gate cego para qualquer problema de TLS. Com o `--resolve`, dá
  para ter conexão direta E cadeia validada.

`SMOKE_INSECURE=1` continua como escape (cert expirado/ausente) e **mantém** o fixamento na origem —
ele só desliga a verificação. Se `SMOKE_ORIGIN` não resolver, o script avisa e cai no DNS público em
vez de abortar: o `getent` tem `|| true` justamente porque, com `pipefail`, uma falha ali mataria o
script **depois do upload e antes do rollback automático**.

### 11.1 A guarda de entidade vazia

`build-frontend-public.sh` faz `rm -rf` + `mkdir` por entidade. Entidade que está no
`config/registry.toml` mas **não tem `data/<id>/` nesta máquina** produz um `public/<id>/` **vazio** —
e o único sinal é um `(0 arquivos)` no meio do log.

Isso não é hipotético: **Roraima está assim em produção**. Ela aparece no seletor de estados
(`src/shared/entities.ts` declara `rr` com `collections: ["servicos"]`) e a aba de Serviços falha —
o Apache devolve o `index.html` do SPA para o JSON que não existe, e o `jsonFetcher` trata HTML como
erro de carga.

O deploy **aborta** nesse caso. As duas saídas:

- recuperar/coletar o `data/<id>/` e rodar de novo; ou
- tirar a entidade do `config/registry.toml` e rodar `node scripts/tools/gen-frontend-entities.mjs`
  (o `entities.ts` é **gerado** do registry — não edite à mão).

`--allow-vazias` publica mesmo assim, ciente do que quebra.

### 11.2 Troca atômica e rollback

O upload vai para `<webroot>.novo` e a publicação é um `mv`. Motivo: esvaziar o webroot durante um
upload de **~253 MB** (eram ~50 MB antes de o TARF fechar; os zips vão dentro do `dist/`) não só
derruba o site como deixa um `index.html` em cache apontando para um bundle `assets/*.js` que acabou
de ser apagado — app branca até o cache expirar.

A versão anterior fica em `<webroot>.antigo`. Rollback:

```bash
ssh -p 22 root@<host> "rm -rf /var/www/html.novo && mv /var/www/html /var/www/html.novo && mv /var/www/html.antigo /var/www/html"
```

> ⚠️ **Dotfiles do webroot não vêm do `dist/`.** O `.htaccess` (roteamento do SPA: qualquer caminho
> inexistente cai no `index.html`) é configuração do servidor. A sequência manual antiga o preservava
> por acidente — `rm -rf /var/www/html/*` não casa dotfile. O script copia esses arquivos para o
> staging **de propósito**, antes da troca. Se mexer nessa parte, confira com
> `ssh … 'ls -la /var/www/html/'` que nada de configuração ficou para trás.

### 11.3 Smoke test

Roda sozinho no fim; **se falhar, reverte**. Verifica `/`, os **três** índices JSON do RS (serviços,
pareceres e TARF) e — o caso mais traiçoeiro — que o bundle `/assets/*.js` referenciado pelo
`index.html` recém-publicado existe de fato. "O site responde 200" não pega dessincronia entre
`index.html` e `assets/`.

> O do TARF entrou em 09/08/2026 e é o que mais justifica a checagem: com 22.476 acórdãos ele tem
> 35 MB, contra 0,6 MB do de pareceres — se um upload truncar, é o candidato número um, e sem essa
> linha o smoke passaria verde com a aba quebrada em produção.

Manualmente, o teste que distingue arquivo servido de fallback do SPA é o **content-type**:

```bash
curl -sk -o /dev/null -w '%{http_code} %{content_type}\n' https://<host>/rs/rs-pareceres-index.json
# 200 application/json  → ok
# 200 text/html         → o arquivo NÃO existe; é o index.html do fallback
```

---

## 12. Conectando clientes ao MCP

O servidor expõe, no **mesmo processo e na mesma porta** do `/v1/question`, um endpoint **MCP**
(Model Context Protocol) em `/mcp` — streamable HTTP, via `rmcp`. Isso permite que a IA de um
auditor consulte o acervo como **ferramenta**, sem passar pelo chat.

Quatro ferramentas: `listar_entidades`, `buscar_pareceres`, `obter_parecer` e
`consultar_servicos_faqs` (detalhes em [docs/auli_code.md](docs/auli_code.md) §3.12).

> **Privacidade (D-MCP-5).** Este caminho **não chama LLM externo**: a pergunta é embedada
> localmente e nunca sai do processo — nenhum texto do usuário atravessa a fronteira do serviço.
>
> **Registro em disco (revisto em 2026-08-02).** Antes, o MCP só emitia metadados no `tracing`, e
> nada em `logs/`. Isso não era doutrina — era lacuna: a D-MCP-5 fala da fronteira do LLM, nunca
> decidiu que a chamada não deveria ser registrada. Hoje **as três ferramentas que recebem
> pergunta** (`buscar_pareceres`, `obter_parecer`, `consultar_servicos_faqs`) gravam pelo MESMO
> caminho do chat: mesmo diretório, mesmas permissões, mesmo formato — com `tipo: mcp:<ferramenta>`
> no cabeçalho e **sem a seção `RESPOSTA`**, que não existe aqui (quem redige é o assistente do
> usuário). `listar_entidades` não gera registro: não há pergunta. O anonimizador passou a rodar
> neste caminho **só para produzir o par original/anonimizada** — o mesmo par que torna a §7.0
> auditável.
>
> A superfície MCP é pública e sem autenticação, e era a única sem trilha. A correção alinha as
> duas faces: **não se registra quem perguntou** (sem nome, IP ou sessão — o IP existe só em
> memória, no rate limiter), registra-se **a pergunta, o horário e o que foi devolvido**.

### 12.1 Teste de protocolo (antes de qualquer cliente)

```bash
./scripts/tools/mcp-smoke.sh http://localhost:3000/mcp
```

Faz `initialize` → `notifications/initialized` → `tools/list` → `tools/call` e imprime as
ferramentas com seus schemas, os resultados reais e o erro de UF sem acervo.

Desde 09/08/2026 ele exercita **as quatro** — antes só três, porque a `consultar_servicos_faqs`
entrou na #115 e nunca chegou ao smoke — e também a busca com `colecao: "tarf"`, que é caminho
distinto no servidor (outra coleção). Sem essas duas, uma quebra em qualquer um dos dois passava com
o smoke inteiro verde.

**Depois do deploy, rode também contra a URL pública** — não só localhost:

```bash
./scripts/tools/mcp-smoke.sh https://api.auli.com.br/mcp
```

> ⚠️ **Por que os dois.** O rmcp valida o header `Host` como guarda de DNS rebinding, e o default
> aceita **só loopback**. O smoke em localhost passa mesmo com a lista mal configurada; atrás do
> tunnel o `Host` chega como `api.auli.com.br` e o servidor responde **403 "Forbidden: Host header
> is not allowed"**, com um `WARN ... rejected request with disallowed Host header` no log. A lista
> é a constante `MCP_ALLOWED_HOSTS` em `auli-cli/src/api/mod.rs` — ao trocar de domínio, edite lá
> (é o mesmo lugar conceitual das origens do CORS).

### 12.2 CLI — Claude Code

```bash
# local (a máquina do servidor):
claude mcp add --transport http auli-local http://localhost:3000/mcp
# produção:
claude mcp add --transport http auli https://api.auli.com.br/mcp
```

Numa sessão, `/mcp` lista o servidor e as ferramentas. O Claude Code fala com **localhost direto** —
é o único cliente Claude que testa sem expor o servidor.

### 12.3 Chat — claude.ai, Desktop, celular

Customize → Connectors → “+” → **Add custom connector**, com a URL `https://api.auli.com.br/mcp`.
Disponível nos planos free/Pro/Max/Team/Enterprise (free = 1 conector; em Team/Enterprise um Owner
precisa adicionar antes em Organization Settings).

> **Atenção à rede.** O Claude conecta ao servidor a partir da **nuvem da Anthropic**, não do
> dispositivo do usuário — vale para claude.ai, Desktop e apps móveis. Logo: (a) **não funciona
> contra `localhost`**; (b) se houver WAF/bot-fight no Cloudflare, o caminho `/mcp` precisa liberar
> os IPs da Anthropic.

### 12.4 Dogfooding no próprio repo

O repo versiona um [`.mcp.json`](.mcp.json) apontando para produção, então toda sessão de Claude
Code dentro do repo enxerga o próprio acervo como ferramenta (`mcp__auli__buscar_pareceres` etc.) —
útil para consultar pareceres durante o desenvolvimento, e um teste de regressão permanente do
endpoint. Na primeira sessão o Claude Code **pede aprovação** do `.mcp.json` de projeto:
comportamento esperado.

### 12.5 Rate limit

O `/mcp` tem limiter próprio: **10 req/s, burst 30** por IP, separado do limiter de
`/v1/question`+`/v1/retrieve` (1 req/s, burst 2). A cota é mais folgada porque o handshake MCP faz
várias requisições em sequência. Excedida, a resposta é **429** em JSON simples (não JSON-RPC) —
correto no nível HTTP; clientes MCP tratam como falha de transporte.
