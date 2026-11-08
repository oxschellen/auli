# Estudo: atualização do fastembed (5.17.2 → 7.6.1) com revetorização de todo o acervo

Documento de implementação para o Claude Code. Versão 2, revisada em 07/11/2026 — substitui o estudo anterior.
Objetivo: atualizar a dependência `fastembed` no monorepo Auli mantendo Rust, BGE-M3 e o contrato do embedder (dense-only, packs reprodutíveis), e **revetorizar o acervo inteiro** com o encoder novo. Não há exigência de igualdade vetorial entre 5.17.2 e 7.6.1.

## Resultado do piloto e decisão (07/11/2026) — vale sobre o plano abaixo onde ele diverge

**O gate falhou, e a causa não é a esperada.** No piloto do RS, nenhum dos 3.538 vetores comparados ficou acima
de 0,995: o mesmo texto (`key_hash` igual) saiu com cosseno ~0,986 (0,973–0,992) entre a 5.17.2 e a 7.6.1. O
piloto parou e o RS voltou do backup. Isolado num experimento à parte (23 textos): os ids de token são idênticos
nas duas versões, o `PreferCPU` não muda nada, a sessão é a mesma (`Level3`). Sobra o **ONNX Runtime 1.24.2 →
1.31.0 no modelo INT8**: a quantização dinâmica muda de resultado com a versão. A premissa da seção seguinte ("só
drift de ponto flutuante, ≥ 0,999") estava errada.

**Medições** (as quatro coleções do RS sem o TARF, 3.538 documentos, num diretório de laboratório que reproduz a
produção bit a bit; 292 perguntas reais do RS do log de consultas; 2.056 perguntas das FAQs do RS como consulta,
acerto = a própria FAQ em 1º):

| | INT8 5.17.2 | INT8 7.6.1 | FP32 7.6.1 (`BAAI/bge-m3`) |
|---|---|---|---|
| revetorizar os 3.538 | 5 min 38 s | — | 12 min 49 s (2,3×) |
| pico de RSS do update | 2,9 GB | ~3,0 GB | 3,8 GB |
| vetorizar uma pergunta (mediana) | 16 ms | — | 39 ms |
| acerto em 1º / MRR@10 (FAQs do RS) | 90,6% / 0,9400 | 90,8% / 0,9408 | **91,2% / 0,9434** |
| o mesmo texto no ONNX Runtime 1.24 × 1.31 | cosseno ~0,986 | | **cosseno 1,0000000** |
| lote > 1 | vaza padding | | bit a bit igual ao um por um |

As duas INT8 ficam à mesma distância do FP32 (cosseno ~0,975–0,98; o 1º resultado igual ao do FP32 em ~70% das
perguntas): a 7.6.1 não piora a busca, só muda os números.

**Decisão do usuário (07/11): o encoder passa a ser o BGE-M3 FP32** (`EmbeddingModel::BGEM3` do `TextEmbedding`;
vetor denso por pooling CLS, normalizado), **e o `EMBED_MODEL_ID` passa de `bge-m3-q-int8` a `bge-m3-fp32`**. É troca
de modelo, e o id existe para isso: o boot recusa pack do modelo antigo, e o cache de vetores do update se desliga
sozinho. O `Some(1)` e o `FATIA` ficam (no FP32 o `FATIA` limita o `last_hidden_state` que o fastembed guarda até o
pooling).

**O que muda no procedimento abaixo:**

- O gate de 0,995 contra o pack antigo não se aplica: FP32 e INT8 diferem por construção (cosseno ~0,98). A
  validação foi o piloto acima; na migração, cada entidade tem de sair com o log "Cache de vetores DESLIGADO" e
  "0 reaproveitados" em todas as coleções.
- Não é preciso tirar o manifesto para desligar o cache (o id novo já desliga), mas o backup continua: mover
  `data/<id>/packs` para `data/<id>/arquivo/packs-bge-m3-q-int8` antes do `build-packs.sh <id>`.
- **O boot passa a ser tudo-ou-nada:** da hora do merge até a 27ª entidade, o servidor não pode ser reiniciado (o
  binário novo recusa as que faltam; o antigo, as já feitas). A mistura de modelos, por isso, não acontece.
- A checagem final é o próprio boot: as 27 com `embed_model_id = bge-m3-fp32`, num servidor de laboratório antes
  do restart.
- Tempo estimado da revetorização completa (37.941 vetores, pela taxa do piloto): ~2 h 20 min.
- O modelo INT8 (`models/models--gpahal--bge-m3-onnx-int8`, 560 MB) fica até o fim da janela de rollback.

## Decisão: revetorizar tudo, sem versionar o encoder

- **A identidade do manifesto não muda.** `EMBED_MODEL_ID`, `EMBED_DIM`, `STRATEGY_VERSION` e `PACK_FORMAT` ficam como estão, e nenhum campo novo entra no manifesto. A troca do encoder se resolve revetorizando todas as 27 entidades, não carimbando versão.
- **Consequência 1 — nada barra a mistura.** Com a identidade igual, o servidor com o binário novo sobe com packs antigos sem reclamar, e o antigo sobe com packs novos. Quem garante que a troca foi completa é o procedimento (passos 5 e 6) e a checagem final, não o boot.
- **Consequência 2 — a armadilha do cache.** O `auli update` reaproveita o vetor de todo documento cujo `key_hash` bate com o do pack anterior, sempre que a identidade do manifesto anterior é igual à do binário (`cache_valido` e `vetores_reaproveitaveis` em `crates/auli-cli/src/update.rs`). Como a identidade não muda, um `scripts/build-packs.sh <id>` comum reaproveitaria 100% dos vetores e **não revetorizaria nada**. Para forçar a revetorização, o manifesto anterior tem de sair do lugar antes: sem ele o update imprime "🧊 Cache de vetores DESLIGADO" e vetoriza tudo. O passo 5 faz isso movendo o diretório de packs para o backup.
- **Por que a mistura temporária é aceitável.** Pelo fonte (seção seguinte), o modelo e a tokenização do BGE-M3 são os mesmos; o que muda é o ONNX Runtime. O esperado é cosseno ≥ 0,999 entre vetor velho e novo do mesmo texto. Se o piloto confirmar, a janela em que packs velhos convivem com o binário novo é inócua, e a migração pode ser feita entidade por entidade, sem congelar o servidor. Se não confirmar (algum cosseno < 0,995), a janela deixa de ser inócua: parar e investigar antes de seguir.

## O que muda de fato entre 5.17.2 e 7.6.1 (fonte comparado em 07/11/2026)

- **Modelo: o mesmo.** `Bgem3Model::BGEM3Q` continua `gpahal/bge-m3-onnx-int8`, arquivo `model_quantized.onnx`. O `hf-hub` continua 0.5.0, então o cache em `models/` (`EMBED_CACHE_DIR`) serve sem novo download.
- **Tokenização: a mesma.** `load_tokenizer` mantém padding `BatchLongest`, truncamento em `max_length` e os tokens especiais; a codificação continua `encode_batch(inputs, true)`. Os dois fallbacks novos (`pad_token` como objeto, `pad_token_id` via `token_to_id`) não disparam: o config do cache tem `pad_token_id = 1` e `pad_token = "<pad>"`.
- **O que muda:** ONNX Runtime 1.24.2 → 1.31.0 (`ort` `=2.0.0-rc.12` → `=2.0.0-rc.15`) e `tokenizers` 0.22.2 → 0.23.2. Diferença numérica de kernel é a única fonte de drift esperada.
- **API usada pelo `embed.rs`: sem alteração.** `Bgem3Embedding::try_new(Bgem3InitOptions)`; `Bgem3InitOptions::new / with_max_length / with_intra_threads / with_cache_dir`; `embed(&mut self, texts, batch_size: Option<usize>) -> Result<Bgem3EmbeddingOutput>` com `dense`, `sparse`, `colbert`.
- **Quebra (6.0.0):** `fastembed::Error` era `pub type Error = anyhow::Error` e virou enum. Impacto no Auli: passo 3.
- **Novo na 7.x: o controle do dispositivo.** O ONNX Runtime pode escolher sozinho onde roda o modelo (CPU, GPU ou NPU). O `ort` já faz isso hoje, na 5.17.2: toda sessão nasce com a política `MaxEfficiency`, que prefere NPU e, no macOS, sempre registra o CoreML. A 7.x só traz o controle para mudar isso: `with_auto_device(AutoDevicePolicy)`, com `AutoDevicePolicy` reexportado pelo `fastembed`; `PreferCPU` nunca registra execution provider. Nesta máquina (Linux, Ryzen 9 7950X, sem NPU, ONNX Runtime baixado só com CPU), a política já resolve para CPU, e fixar `PreferCPU` não muda nenhum vetor. Fixar mesmo assim (passo 2): num Mac, ou numa máquina com NPU/GPU e um ONNX Runtime que a suporte, o `MaxEfficiency` levaria o modelo para outro dispositivo em silêncio, possivelmente com precisão menor, e os vetores gerados ali sairiam diferentes dos gerados aqui. É o mesmo tipo de divergência que o `Some(1)` e o lockfile único existem para impedir.
- **Features:** `ort-download-binaries-rustls-tls` já habilita o novo feature `onnx`; nada a acrescentar na linha do `Cargo.toml`.
- **MSRV:** `rust-version = "1.88"` no crate; a máquina tem 1.99.1.
- **Changelog 7.2–7.6.1:** o diff do fonte acima cobre o caminho do BGE-M3. O changelog fica como leitura complementar, não como gate.

## Contexto atual (verificado em 07/11/2026)

- Workspace `auli-server/`, crate `crates/auli-core`. Pin em `crates/auli-core/Cargo.toml`:

  ```toml
  fastembed = { version = "5.17.2", default-features = false, features = ["ort-download-binaries-rustls-tls", "hf-hub-rustls-tls"] }
  ```

- `Cargo.lock`: fastembed 5.17.2, ort 2.0.0-rc.12, tokenizers 0.22.2, hf-hub 0.5.0.
- Encoder único em `crates/auli-core/src/embed.rs`, construído pelo update (`crates/auli-cli/src/update.rs`) e pelo servidor (`crates/auli-cli/src/lib.rs`).
- `Bgem3Model::BGEM3Q` (INT8), `EMBED_MAX_TOKENS = 8192`, `batch_size = Some(1)` obrigatório (evita vazamento de padding), `FATIA = 256` (limita a memória do `colbert`/`sparse` que o fastembed acumula), `TETO_TEXT_TO_EMBED = 6_000` caracteres em `auli-contract`.
- Três testes ignorados em `mod testes_ordem` fixam o determinismo bit a bit e a independência da composição do lote.
- `crates/auli-core/src/error.rs` tem a variante `Anyhow(#[from] anyhow::Error)`, que só existe por causa do fastembed: é o único uso de `anyhow` no `auli-core`.
- **Acervo:** 27 entidades, 37.941 vetores — faqs 17.134, pareceres 11.284, tarf 4.712, serviços 4.302, legislação 509. Maiores: SP 10.257 e RS 8.250. Os packs somam 508 MB e ficam fora do git (`/data/*` no `.gitignore`).
- **Packs × árvores:** em 07/11 a contagem de cada pack é igual ao número de `.md` da árvore correspondente, e não há `*-remocoes-pendentes.tsv` em nenhuma entidade. Revetorizar a partir das árvores não perde nem descarta documento.
- **Boot:** valida identidade e `docs_hashes` de cada entidade, e um erro em qualquer uma derruba o boot inteiro (`crates/auli-cli/src/packs.rs`). Como a identidade não muda, a troca do encoder não aciona essa validação.
- **Compilação implícita:** `start_server.sh` e `scripts/build-packs.sh` compilam por padrão, e todos os `scripts/tools/update-*-faqs.sh` chamam o `build-packs.sh`. Depois do merge, qualquer um deles já usa o encoder novo.

## Escopo

1. Subir a dependência para `7.6.1`.
2. Fixar `PreferCPU` no construtor.
3. Trocar a variante de erro do fastembed; nenhuma assinatura pública muda.
4. Manter o contrato de `Embedder::embed_dense` (`Vec<Vec<f32>>`), o `Some(1)` e o `FATIA`.
5. Revetorizar as 27 entidades a partir das árvores, sem reaproveitar nenhum vetor.
6. Não mexer na identidade do manifesto nem no `PACK_FORMAT`; não introduzir hybrid retrieval, Qwen3 nem mudança de modelo.

## Passos de implementação

### 1. Dependência

Em `auli-server/crates/auli-core/Cargo.toml`:

```toml
fastembed = { version = "7.6.1", default-features = false, features = ["ort-download-binaries-rustls-tls", "hf-hub-rustls-tls"] }
```

A partir de `auli-server/`:

```bash
cargo update -p fastembed
cargo tree -i ort          # um único ort, =2.0.0-rc.15
cargo tree -i tokenizers   # 0.23.2
```

Commitar o `Cargo.lock`. O comentário no `Cargo.toml` raiz já explica por que o lockfile único é obrigatório (encoder de documentos e de queries idênticos).

### 2. Construtor do Embedder

```rust
use fastembed::{AutoDevicePolicy, Bgem3Embedding, Bgem3InitOptions, Bgem3Model};

let opts = Bgem3InitOptions::new(Bgem3Model::BGEM3Q)
    .with_max_length(EMBED_MAX_TOKENS)
    .with_intra_threads(threads)
    .with_cache_dir(cache_dir)
    .with_auto_device(AutoDevicePolicy::PreferCPU);
let model = Bgem3Embedding::try_new(opts)?;
```

Documentar no comentário do `embed.rs` que o `PreferCPU` faz parte do contrato de reprodutibilidade: nunca registra execution provider automaticamente, em qualquer SO.

### 3. Erro do fastembed

O `embed.rs` devolve o `Result` do `auli-core`, não `anyhow::Result`. Hoje os dois `?` (em `try_new` e em `model.embed`) convertem via `Anyhow(#[from] anyhow::Error)`; na 7.x o erro é `fastembed::Error`, e os dois deixam de compilar.

- Em `crates/auli-core/src/error.rs`, trocar a variante por:

  ```rust
  #[error(transparent)]
  Embed(#[from] fastembed::Error),
  ```

- Remover `anyhow` do `Cargo.toml` do `auli-core` (confirmar com `grep -rn anyhow crates/auli-core` que não sobrou uso).
- Atualizar o comentário da linha do `try_new` no `embed.rs` (hoje diz "anyhow::Error -> crate::Error via #[from]").
- `embed_dense` sem mudança de lógica: `model.embed(fatia, Some(1))`, fatiamento por `FATIA`, retorno só do dense.

### 4. Testes e guardas

A partir de `auli-server/`, com o cache aquecido da raiz do repo (o padrão dos testes, `../models` relativo ao crate, não é ele):

```bash
export EMBED_CACHE_DIR="$(git rev-parse --show-toplevel)/models"
cargo test -p auli-core testes_ordem -- --ignored
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Os três testes de `testes_ordem` têm de passar na 7.6.1: `embedding_independe_da_ordem_e_da_composicao_do_lote`, `fatiar_a_entrada_nao_muda_um_unico_vetor` e `embedding_e_deterministico_entre_chamadas`. O filtro `testes_ordem` pega os três; o `embedding_` do estudo anterior deixava o segundo de fora.

### 5. Piloto: revetorizar uma entidade e comparar

Piloto: `rs`, que tem as cinco coleções e ~22% do acervo, o que dá uma boa base para estimar tempo e RAM. A partir da raiz do repo, com o código do PR compilando:

```bash
date -u +%FT%T > migracao-T0.txt   # início da migração em UTC (o built_at é UTC); usado na checagem final
ID=rs
ARQ=data/$ID/arquivo/packs-fastembed-5.17.2
mkdir -p data/$ID/arquivo
mv data/$ID/packs "$ARQ"      # backup e, ao mesmo tempo, tira o manifesto: o update não reaproveita nada
/usr/bin/time -v scripts/build-packs.sh $ID 2>&1 | tee build-$ID.log
```

Conferir no log:

- "🧊 Cache de vetores DESLIGADO";
- em cada coleção, "0 reaproveitados";
- `Maximum resident set size` e `Elapsed`. Desde o `FATIA` o pico medido foi 1,64 GB (SC), e ele não cresce mais com o tamanho da coleção; o tempo do RS × ~4,6 estima o acervo inteiro.

Depois, comparar o pack velho com o novo, registro a registro, por `id`. O script abaixo lê `data/<id>/arquivo/packs-fastembed-5.17.2/` e `data/<id>/packs/`. Salvar fora do repo (por exemplo, no scratchpad) e rodar da raiz com `python3 -I comparar_packs.py data rs`; sem ids, compara todas as entidades que já têm o backup.

```python
import glob, json, math, os, statistics, sys

# uso (da raiz do repo): python3 -I comparar_packs.py data [id ...]
# Compara data/<id>/arquivo/packs-fastembed-5.17.2/<id>-<kind>.json (velho) com data/<id>/packs/<id>-<kind>.json (novo).
raiz, ids = sys.argv[1], sys.argv[2:] or ['*']
coss, iguais, faltam, key_mudou, total = [], 0, 0, 0, 0
for i in ids:
    for f in sorted(glob.glob(os.path.join(raiz, i, 'arquivo', 'packs-fastembed-5.17.2', '*-*.json'))):
        ent = f.split(os.sep)[-4]
        g = os.path.join(raiz, ent, 'packs', os.path.basename(f))
        a = {r['id']: r for r in json.load(open(f))['records']}
        b = {r['id']: r for r in json.load(open(g))['records']} if os.path.exists(g) else {}
        for k, ra in a.items():
            total += 1
            rb = b.get(k)
            if rb is None:
                faltam += 1
                continue
            if ra['key_hash'] != rb['key_hash']:
                key_mudou += 1
                continue
            x, y = ra['embedding'], rb['embedding']
            iguais += x == y
            d = sum(p * q for p, q in zip(x, y))
            coss.append(d / (math.sqrt(sum(p * p for p in x)) * math.sqrt(sum(q * q for q in y))))
coss.sort()
print(f'registros {total} | sem par no novo {faltam} | key_hash mudou {key_mudou} | '
      f'comparados {len(coss)} | bit a bit iguais {iguais}')
if coss:
    print(f'cosseno mín {coss[0]:.6f} | p1 {coss[len(coss) // 100]:.6f} | '
          f'mediana {statistics.median(coss):.6f} | máx {coss[-1]:.6f} | '
          f'abaixo de 0,995: {sum(c < 0.995 for c in coss)}')
```

Leitura:

- **"sem par" e "key_hash mudou" = 0.** A árvore não mudou entre o backup e a revetorização. Com o `key_hash` igual, o texto vetorizado é o mesmo, e toda diferença de vetor vem do encoder.
- **Cosseno: esperado ≥ 0,999. Gate: nenhum abaixo de 0,995.** O limiar é apertado de propósito: o vazamento de padding que motivou o `Some(1)` e os `testes_ordem` dava 0,978, e um limiar como "> 0,98" o deixaria passar. Abaixo do gate: parar, restaurar a entidade (`rm -rf data/$ID/packs && mv "$ARQ" data/$ID/packs`) e investigar (tokenizer, `max_length`, truncamento, execution provider) antes de seguir.
- **"bit a bit iguais" é informativo**, não prova de reaproveitamento: a prova de que nada foi reaproveitado é o "0 reaproveitados" do log.
- Registrar no PR: mín, p1, mediana e máx do cosseno, o tempo e o pico de RAM do piloto.

### 6. Revetorizar as demais entidades

Só depois do piloto aprovado. Uma entidade por vez (o update já usa 16 threads; em paralelo, só disputariam CPU), na raiz do repo:

```bash
set -o pipefail   # sem isto, o `| tee` esconde a falha do build-packs
for ID in $(ls -d data/*/docs | cut -d/ -f2); do
  [ "$ID" = rs ] && continue
  ARQ=data/$ID/arquivo/packs-fastembed-5.17.2
  [ -e "$ARQ" ] && { echo "⏭  $ID já tem backup — conferir à mão"; continue; }
  mkdir -p data/$ID/arquivo && mv data/$ID/packs "$ARQ" \
    && scripts/build-packs.sh $ID 2>&1 | tee build-$ID.log || { echo "❌ $ID"; break; }
done
python3 -I comparar_packs.py data
```

- `ls -d data/*/docs` lista as 27 entidades (o `data/eval/` não tem `docs/`).
- **Não reiniciar o servidor no meio de uma entidade:** sem manifesto e sem packs, ela sobe vazia, com aviso e sem erro.
- Atualizações de rotina de um estado (`update-*-faqs.sh`) durante a janela não estragam nada. Feitas antes da revetorização do estado, reaproveitam o vetor velho, mas o laço vai revetorizar aquele estado depois. Feitas depois, reaproveitam o vetor novo. Nesse caso, "sem par" e "key_hash mudou" na comparação deixam de ser zero, pelo motivo esperado.

**Checagem final** — nenhuma entidade pode ter ficado com vetor velho, e sem versão no manifesto o boot não pega isso. Cada uma das 27 tem de ter o backup e um manifesto escrito depois do início da migração (o `migracao-T0.txt` do piloto):

```bash
T0=$(cat migracao-T0.txt)
for ID in $(ls -d data/*/docs | cut -d/ -f2); do
  B=$(python3 -I -c "import json,sys; print(json.load(open(sys.argv[1]))['built_at'])" data/$ID/packs/$ID.manifest.json)
  [ -d data/$ID/arquivo/packs-fastembed-5.17.2 ] && [[ "$B" > "$T0" ]] || echo "❌ $ID: built_at $B"
done
```

### 7. Implantação

1. Merge do PR (código, `Cargo.lock`).
2. Passos 5 e 6, com o servidor atual no ar: ele carrega os packs na memória no boot e não depende dos arquivos até o próximo restart.
3. Smoke antes e depois: com o servidor antigo ainda no ar, gravar o top-5 de 5–10 perguntas reais (`data/eval/` ou o log de auditoria) via `POST /v1/retrieve`; depois do restart, repetir e comparar. Pequenas trocas de ordem são esperadas; resultado vazio ou irrelevante, não.
4. Restart do servidor (`start_server.sh`, que recompila com o encoder novo) depois das 27.
5. Frontend e downloads não mudam: os índices das abas derivam das árvores, e o `auli bundle` empacota só os `.md`.

**Rollback**, enquanto os backups existirem: por entidade, `rm -rf data/$ID/packs && mv data/$ID/arquivo/packs-fastembed-5.17.2 data/$ID/packs`; `git revert` do commit; restart. Se a árvore de um estado mudou depois do backup, o `docs_hashes` do manifesto restaurado diverge e o boot recusa; nesse caso, rodar `scripts/build-packs.sh <id>` com o binário antigo, que reaproveita os vetores pelo `key_hash`.

Os backups (`data/*/arquivo/packs-fastembed-5.17.2`, ~508 MB no total) ficam até a próxima revetorização completa.

## Riscos e mitigações

| Risco | Mitigação |
|-------|-----------|
| O update reaproveita os vetores velhos em silêncio (identidade igual) | Mover o diretório de packs antes do `build-packs.sh`; conferir "Cache DESLIGADO" e "0 reaproveitados" no log |
| Entidade esquecida com vetor velho — sem versão, o boot não acusa | Checagem final das 27 (backup + `built_at` > `T0`) |
| Encoder novo quebrado (tokenizer, truncamento) | Comparação velho × novo de todos os registros no piloto, gate 0,995, antes de seguir |
| Regressão de padding/batch | `Some(1)` + `FATIA` + os três testes de `testes_ordem` |
| Execution provider escolhido automaticamente | `PreferCPU` explícito |
| `fastembed::Error` virou enum | Variante `Embed(#[from] fastembed::Error)`; assinatura pública inalterada |
| Restart com entidade sem packs | Não reiniciar no meio de uma entidade |
| Tempo/RAM da revetorização | Piloto no RS; RAM ~1,6–2 GB por rodada desde o `FATIA` |

## Fora de escopo

- Controle de versão do embedding: bump de `STRATEGY_VERSION` ou de `EMBED_MODEL_ID`, campos de versão do encoder no manifesto.
- Trocar de modelo (Qwen3, EmbeddingGemma etc.) ou ativar `qwen3`.
- Hybrid dense+sparse. Nota: o 7.0.0 trouxe `SparseTextEmbedding::try_new_from_user_defined` e `UserDefinedSparseModel::with_idf_file`, que é o que o `ESTUDO-busca-hibrida.md` chamou de "teste de meia hora". Fica para depois, mas este upgrade abre a porta.
- Alterar `FATIA`, `batch_size`, `max_length`, `TETO_TEXT_TO_EMBED` ou `PACK_FORMAT`.

## Critério de pronto

- [ ] `fastembed` 7.6.1 no `Cargo.toml` e no `Cargo.lock`; `cargo tree -i ort` mostra um único ort, rc.15
- [ ] `PreferCPU` no construtor, com comentário
- [ ] Variante `Embed(#[from] fastembed::Error)`; `anyhow` fora do `auli-core`; nenhuma assinatura pública mudou
- [ ] Os três testes de `testes_ordem` passam na 7.6.1; fmt, clippy e `cargo test --workspace` limpos
- [ ] Piloto RS: log com "0 reaproveitados"; comparação com cosseno mínimo ≥ 0,995; mín/p1/mediana/máx, tempo e RAM no PR
- [ ] As 27 entidades revetorizadas; checagem final sem ❌
- [ ] Smoke de retrieval ok; servidor reiniciado com o binário novo
- [ ] Backups em `data/*/arquivo/packs-fastembed-5.17.2` até a próxima revetorização completa
- [ ] Comentários do `embed.rs` continuam precisos
