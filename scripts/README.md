# `scripts/`

Tudo o que é preciso para **publicar o site e servir a API**. O que não entra nesse caminho —
coleta, codegen, verificação e as guardas de CI — mora em [`tools/`](tools/).

## Subir o app inteiro

São **dois comandos, em dois terminais**. Eles são independentes: o site é estático e serve as abas
de referência do próprio origin; só o chat fala com a API.

```bash
# terminal 1 — a API (ocupa o terminal; Ctrl+C derruba servidor e túnel juntos)
./start_server.sh

# terminal 2 — o site (pergunta antes de começar)
scripts/deploy-frontend.sh
```

Quando a API está fora do ar, suba ela primeiro: volta em minutos, enquanto o deploy sobe ~253 MB
por `scp`.

Espere estas duas linhas no terminal 1 antes de considerar a API no ar:

```text
📦 rs-tarf — 22476 registros
✅ Server started successfully at 127.0.0.1:3000
```

## Os três scripts

| script | o que faz | chamado por |
| ------ | --------- | ----------- |
| `deploy-frontend.sh` | publica o site: 8 passos, troca atômica, rollback automático | você |
| `build-frontend-public.sh` | copia `data/<id>/{raw,ref}/` → `auli-frontend/public/<id>/` | `deploy-frontend.sh`, ou você com `<id>` |
| `build-packs.sh` | vetoriza a árvore `.md` de uma entidade e deriva os índices | `tools/atualizar-servicos.sh`, `tools/update-rs-faqs.sh`, ou você |

O `./start_server.sh` fica na **raiz** do repositório, não aqui: é o comando mais rodado do projeto,
e a raiz é onde você já está quando abre o terminal.

## O que o `deploy-frontend.sh` faz

```text
scripts/deploy-frontend.sh
│
├─ ⏸  PORTÃO — mostra versão do frontend e contagem dos packs, e espera você confirmar.
│        Antes de qualquer trabalho, para você responder e sair de perto.
│
├─ 1/8  cargo build --release -p auli-cli        (pula se AULI_BIN vier de fora)
├─ 2/8  scripts/build-frontend-public.sh         public/<id>/ ← data/<id>/{raw,ref}/
├─ 3/8  auli bundle                              os 27 .zip em public/downloads/
├─ 4/8  guarda: nenhuma entidade do registry pode estar sem dados
├─ 5/8  npm run build                            dist/  (com public/ dentro)
├─ 6/8  ssh + scp                                sobe para <webroot>.novo — produção intocada
├─ 7/8  ssh                                      troca atômica: dois `mv`. AQUI O SITE MUDA
└─ 8/8  curl                                     smoke; se falhar, reverte sozinho
```

```bash
scripts/deploy-frontend.sh --dry-run       # faz o local inteiro; nada é enviado
scripts/deploy-frontend.sh --sim           # sem a confirmação (automação)
scripts/deploy-frontend.sh --allow-vazias  # publica mesmo com entidade sem dados (raro)
```

## Quando rodar cada um sozinho

**`scripts/build-packs.sh <id>`** — obrigatório sempre que a árvore `data/<id>/docs/` mudar. O
manifesto carimba um `docs_hash` sobre ela e o servidor **recusa subir** se divergir, então pular
este passo não dá erro sutil: dá boot recusado. É caro — no `rs`, ~72 min e pico de ~42 GB de RAM,
porque re-vetoriza a entidade inteira e não só a coleção que mudou.

Depois dele, rode o `deploy-frontend.sh` (que regenera o `public/`) e reinicie a API — os packs só
são lidos no boot.

**`scripts/build-frontend-public.sh <id>`** — para ver dado novo no `npm run dev` sem publicar. É o
passo 7 do procedimento de entidade nova
([SCRAPERS.md](../auli-server/crates/scrapers/SCRAPERS.md)).

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
