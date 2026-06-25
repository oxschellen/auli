# Tarefa: mover o cache do modelo (`models/`) para a raiz do projeto

> **Para o Claude Code.** Runbook para executar na máquina local. Trabalhe incremental e
> **verifique a árvore real antes de cada edição** (números de linha são dicas e podem ter mudado).
> Se a realidade divergir do descrito, **pare e reporte**.

## Contexto

`models/` é o cache (gitignored) dos pesos do BGE-M3 (ONNX), baixados do Hugging Face no 1º uso —
não é código versionado. Hoje ele acaba em `auli-engine/models/` porque os lançadores usam o caminho
**relativo** `./models` a partir de CWDs diferentes:

- `start_server.sh` entra em `auli-engine/` e usa o default `./models` → `auli-engine/models`.
- `scripts/build-packs.sh` força `EMBED_CACHE_DIR=$ROOT/auli-engine/models`.
- `.env`/`.env.example` trazem `EMBED_CACHE_DIR=./models`.

Isso é exatamente a inconsistência de "duas fontes" anotada no `auli_pendencias.md`.

**Objetivo:** o cache passa a viver em **`<raiz_do_repo>/models/`** (ex.: `~/Desktop/auli_new/models`),
de forma **CWD-independente**, eliminando a fonte dupla.

**Decisão de design (siga-a):** usar **caminho absoluto `$ROOT/models`** definido pelos scripts de
lançamento como autoridade, e **parar de definir `EMBED_CACHE_DIR` no `.env`** (relativo é frágil
entre lançadores). O default do código (`./models`) pode ficar — só vale para execuções manuais.

## Onde trabalhar

- Repositório: `~/Desktop/auli_new`. Comece em `main` limpo; crie a branch `chore/models-to-root`.

## Nota de coordenação com a outra tarefa

A tarefa de remoção do `auli-server` também edita `start_server.sh` e `scripts/build-packs.sh`
(linha do `CARGO_TARGET_DIR` / `BIN`). **Não conflitam** se rodadas em sequência. Esta tarefa mexe
**apenas** nas linhas de `EMBED_CACHE_DIR`/`models`; **não toque** em `CARGO_TARGET_DIR` aqui.

## Passos

### 1. Branch
Crie e entre em `chore/models-to-root` a partir de `main`.

### 2. Mover os pesos já baixados (evitar re-download)
- Verifique o que existe: procure pastas `models` em até 2 níveis
  (`auli-engine/models`, `auli-server/models`, e uma eventual `models` já na raiz) e seus tamanhos.
- Se existir `auli-engine/models/` com os pesos, **mova-a para a raiz**: o conteúdo deve passar a
  ficar em `<raiz>/models/`. Se já houver uma `models/` na raiz, faça merge (não duplique).
- A `auli-server/models/` é cache morto do monólito; **não** mova — pode apagar (ou deixe que a
  tarefa de remoção do `auli-server` cuide dela).

### 3. Tornar os lançadores absolutos (autoridade = `$ROOT/models`)
Em cada script, defina `EMBED_CACHE_DIR` como `$ROOT/models` (absoluto), preservando a possibilidade
de override externo via `${EMBED_CACHE_DIR:-...}`:

- **`start_server.sh`**: hoje **não** exporta `EMBED_CACHE_DIR` (depende de `.env`/default). Adicione
  um `export EMBED_CACHE_DIR="${EMBED_CACHE_DIR:-$ROOT/models}"` junto aos outros `export` (antes do
  `cd "$WS"`). Como o script já calcula `ROOT` = raiz do repo, isso aponta para `<raiz>/models`
  independentemente do CWD. (O `dotenv` do binário não sobrescreve variável já definida no ambiente,
  então este export prevalece sobre o `.env`.)
- **`scripts/build-packs.sh`**: troque o default de `EMBED_CACHE_DIR` de `$ROOT/auli-engine/models`
  para `$ROOT/models`.
- **`auli-engine/scripts/start_local.sh`**: este script faz `cd` para `auli-engine/`, então `./models`
  resolveria errado. Defina `EMBED_CACHE_DIR` apontando para a **raiz** do repo. Como ele não calcula
  `ROOT`, derive a raiz a partir do diretório do script (a raiz é o pai de `auli-engine/`) e use esse
  caminho absoluto; atualize também o comentário de ajuda que diz `default ./models`.

### 4. Remover a fonte concorrente no `.env`
- **`.env.example`**: remova (ou comente) a linha `EMBED_CACHE_DIR=./models`, com uma nota curta de
  que os scripts de lançamento definem o cache em `<raiz>/models` automaticamente; quem rodar o
  binário **na mão** pode exportar `EMBED_CACHE_DIR` manualmente.
- **`.env`** (arquivo real, gitignored — não está no GitHub): faça a mesma remoção/comentário no
  arquivo local, para o ambiente real bater com o `.example`.

### 5. Documentação
Atualize as menções para refletir a raiz:
- **`README.md`**: a descrição do `EMBED_CACHE_DIR` (default/caminho) e qualquer texto que situe o
  cache em `auli-engine/models`.
- **`auli-engine/README.md`**: os exemplos que usam `EMBED_CACHE_DIR=./models` — esclareça que, pelos
  lançadores, o cache fica em `<raiz>/models`; o `./models` só vale para execução manual de dentro de
  `auli-engine/`.
- **`auli_operations.md`**: as linhas que citam `EMBED_CACHE_DIR=./models → auli-engine/models` e o
  troubleshooting de "CWD errado" — agora o caminho canônico é `<raiz>/models` (absoluto), então o
  problema de CWD deixa de existir pelos scripts.
- **`auli_pendencias.md`**: marque a pendência do "`EMBED_CACHE_DIR` com duas fontes" como
  **resolvida** (fonte única, absoluta na raiz), ou remova o item.

### 6. `.gitignore` (tidy, opcional mas recomendado)
- A raiz já ignora `/models/` e `**/models--*/` — então o cache na raiz já está coberto. ✔
- A entrada `auli-engine/models/` (root `.gitignore`) e `/models` (em `auli-engine/.gitignore`) ficam
  desnecessárias; pode removê-las para evitar confusão. (Deixe `auli-server/models/` para a tarefa de
  remoção do `auli-server`.)

### 7. Testes (opcional)
O `auli-engine/crates/auli-cli/tests/packs_smoke.rs` documenta um invocação com `EMBED_CACHE_DIR`
relativo. Se quiser consistência, ajuste o comentário/exemplo para apontar ao `models/` da raiz
(absoluto). Não é bloqueante.

## Verificação

1. **Onde o modelo é procurado.** Rode `start_server.sh` (ou só inspecione o `export`) e confirme que
   `EMBED_CACHE_DIR` resolve para `<raiz>/models` — não para `auli-engine/models`. O mesmo para
   `build-packs.sh` e `start_local.sh`.
2. **Sem re-download.** Se os pesos foram movidos no passo 2, subir o server **não** deve rebaixar o
   modelo (logs do `fastembed`/`ort` não mostram download). Se rebaixar, o caminho ainda está errado.
3. **Sem fonte concorrente.** `grep -rIn "EMBED_CACHE_DIR" . --exclude-dir=.git` não deve mostrar mais
   o `.env`/`.env.example` definindo `./models`; só os scripts (absoluto) e o default do código.
4. `GET /v1/health` OK após subir.

## Commit, push, merge

Commits sugeridos:
1. `chore: move o cache do modelo para <raiz>/models (caminho absoluto nos lançadores)`
2. `docs: atualiza referências de EMBED_CACHE_DIR e fecha a pendência de fonte dupla`

`git push origin chore/models-to-root`; após verificar, `git switch main`, merge e
`git push origin main`.

## Critérios de sucesso

- O cache do modelo vive em `<raiz>/models/` e é encontrado por **todos** os lançadores,
  independentemente do CWD.
- `EMBED_CACHE_DIR` tem **fonte única** (os scripts, absoluto); o `.env` não compete mais.
- Subir o server não rebaixa o modelo (pesos foram movidos, não duplicados).
- Documentação coerente e a pendência de "duas fontes" fechada.
