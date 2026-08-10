# `scripts/`

Tudo o que é preciso para **publicar o site e servir a API**. O que não entra nesse caminho —
coleta, codegen, verificação e as guardas de CI — mora em [`tools/`](tools/).

## Subir o app inteiro: um comando

```bash
scripts/publicar.sh
```

Compila, publica o frontend e sobe a API, nesta ordem. Pergunta antes de qualquer coisa sair da
máquina.

```bash
scripts/publicar.sh --dry-run     # faz o local inteiro; nada é enviado, a API não sobe
scripts/publicar.sh --packs rs    # + re-vetoriza o rs antes (~72 min, pico ~42 GB)
scripts/publicar.sh --no-tunnel   # a API sobe só em localhost, sem o túnel Cloudflare
scripts/publicar.sh --sim         # sem o portão de confirmação (automação)
```

## A ordem de chamada

```text
scripts/publicar.sh
│
├─ 1  cargo build --release -p auli-cli -p auli-collections
│     ↳ compila UMA vez e exporta AULI_BIN + AULI_COLLECTIONS_BIN; daqui para baixo ninguém
│       recompila, e nenhum artefato pode sair de código velho
│
├─ 2  scripts/build-packs.sh <id>          ← só com --packs
│     ↳ auli update            vetoriza data/<id>/docs/ → data/<id>/packs/
│     ↳ auli-collections indice deriva os índices de pareceres e tarf
│
├─ ⏸  PORTÃO — mostra versão do frontend e contagem dos packs, e espera você confirmar
│
├─ 3  scripts/deploy-frontend.sh
│     ├─ 1/8  compila o auli (pula: já veio pronto do passo 1)
│     ├─ 2/8  scripts/build-frontend-public.sh   public/<id>/ ← data/<id>/{raw,ref}/
│     ├─ 3/8  auli bundle                        os 27 .zip em public/downloads/
│     ├─ 4/8  guarda: nenhuma entidade do registry pode estar sem dados
│     ├─ 5/8  npm run build                      dist/  (com public/ dentro)
│     ├─ 6/8  ssh + scp                          sobe para <webroot>.novo — produção intocada
│     ├─ 7/8  ssh                                troca atômica: dois `mv`. AQUI O SITE MUDA
│     └─ 8/8  curl                               smoke; se falhar, reverte sozinho
│
└─ 4  ./start_server.sh --no-build          ← na RAIZ do repo, não aqui
      ↳ auli server na :3000 + túnel do Cloudflare
      ↳ ÚLTIMO por construção: não daemoniza, e o `exec` substitui o publicar.sh
```

Sem `--packs` são 3 passos: o 2 some e os outros renumeram.

## Os quatro scripts

| script | o que faz | chamado por |
| ------ | --------- | ----------- |
| `publicar.sh` | orquestra os quatro passos acima | você |
| `deploy-frontend.sh` | os 8 sub-passos da publicação, com troca atômica e rollback automático | `publicar.sh`, ou você |
| `build-frontend-public.sh` | copia `data/<id>/{raw,ref}/` → `auli-frontend/public/<id>/` | `deploy-frontend.sh`, ou você com `<id>` |
| `build-packs.sh` | vetoriza a árvore `.md` de uma entidade e deriva os índices | `publicar.sh --packs`, `tools/atualizar-servicos.sh`, `tools/update-rs-faqs.sh`, ou você |

## Quando rodar cada um sozinho

**`./start_server.sh`** (na raiz) — só a API, sem publicar nada. É o que se usa para recuperar o
atendimento rápido, ou para desenvolver contra a API local.

**`scripts/deploy-frontend.sh`** — só o site, sem mexer na API. Elas são independentes: o frontend é
estático e serve as abas do próprio origin; só o chat chama a API.

**`scripts/build-packs.sh <id>`** — obrigatório sempre que a árvore `data/<id>/docs/` mudar. O
manifesto carimba um `docs_hash` sobre ela e o servidor **recusa subir** se divergir, então pular
este passo não dá erro sutil: dá boot recusado.

**`scripts/build-frontend-public.sh <id>`** — para ver dado novo no `npm run dev` sem publicar. É o
passo 7 do procedimento de entidade nova ([SCRAPERS.md](../auli-server/crates/scrapers/SCRAPERS.md)).

## Separando servidor e frontend

Publicar os dois de uma vez é o caso comum, mas quando a API está fora do ar vale recuperá-la
primeiro — ela volta em minutos, enquanto o deploy do frontend sobe ~253 MB por `scp`:

```bash
nohup ./start_server.sh > /tmp/auli-server.log 2>&1 &   # 1. a API volta
tail -f /tmp/auli-server.log                            # espere o "Server started successfully"
scripts/deploy-frontend.sh                              # 2. o site, quando quiser
```

Rodados assim, os dois continuam compilando o que precisam — separar não reintroduz o risco de
binário velho.

## Uma regra que vale para todos

**Commit em Rust não muda binário nenhum, e o que sobe é o binário.** Todo script daqui que usa
`auli` ou `auli-collections` ou compila antes, ou recebe o caminho pronto de quem compilou. Isso
não era verdade até 09/08/2026, e o resultado foi publicar zips com um texto removido do código
cinco horas antes — sem erro, sem teste vermelho. Ver
[auli_operations.md §11](../docs/auli_operations.md).

## Onde está o resto

- [`tools/`](tools/) — coleta, codegen, verificação e as guardas de CI, com README próprio.
- [`docs/auli_operations.md`](../docs/auli_operations.md) — o runbook: pré-requisitos, dados
  necessários, logs, túnel, troubleshooting.
