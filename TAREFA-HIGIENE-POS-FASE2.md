# TAREFA — Higiene pós-fase 2 (achados da verificação)

## Contexto

A verificação independente da fase 2 (build limpo dos scrapers fora do WSL, testes,
clippy `-D warnings`, smoke end-to-end do `process` com snapshot v2 sintético
multi-classe, rejeições de v1/entidade) aprovou a entrega. Restaram **dois resíduos de
documentação/CI** para corrigir e **uma observação de arquitetura** só para registrar.
Um commit único de doc/chore resolve.

## Correções

### 1. Doc do CLI no `main.rs` do collections (contradiz o código)

`auli-server/crates/auli-collections/src/main.rs`, comentário do CLI (linhas ~10–14):
ainda menciona `[--usecache]` e diz que "`servicos` raspa os serviços do SC
(temporário) e então deriva" — mas o `match` logo abaixo **rejeita** `faqs`/`servicos`
apontando para os binários `auli-scraper-*`. Resíduo de uma etapa intermediária.

Reescrever o comentário para o estado real:

```
// CLI: `auli-collections <entity> [process]`
//   <entity>   entity id (ex.: `rs`); vazio/omitido -> entidade padrão.
//   process    (padrão e único subcomando) deriva os artefatos do snapshot, offline.
//   A coleta é dos binários `auli-scraper-rs` / `auli-scraper-sc` (fase 2).
```

Conferir também se o filtro `!a.starts_with("--")` no parse ainda tem razão de existir
sem flags — se sim (tolerar flags desconhecidas silenciosamente é ruim), simplificar;
qualquer flag vira erro amigável.

### 2. Path filter órfão na CI

`.github/workflows/registry-sync.yml`, linha ~15: o filtro
`auli-server/crates/auli-collections/src/entities/**` aponta para um caminho que não
existe mais (virou `domain/` há tempos) — o gatilho nunca dispara por essa rota.

Revisar os `paths` do workflow contra o layout atual: o que o registry-sync realmente
precisa observar hoje é `data/registry.toml` (e o que mais o job usar — ler o workflow
inteiro antes de editar). Remover o caminho morto; adicionar os caminhos reais.

## Registro (sem ação agora)

- **Acoplamento collections→kit**: o `auli-collections` (offline) depende do
  `auli-scraper-kit` apenas para `snapshot::load` e o re-export do `Servico`
  per-público, arrastando `ureq`/`time` para seu grafo de build. Se um dia incomodar,
  o caminho é mover o shape per-público para o `auli-contract` (ele *é* um contrato —
  o frontend o consome) e inlinar o `load` (serde puro) no collections. Registrar como
  nota no doc de módulo do kit (`servico.rs` ou `lib.rs`), sem refatorar.

## Critérios de aceitação

- Comentário do CLI fiel ao comportamento; nenhum texto remanescente sobre coleta
  "temporária" ou `--usecache` no collections.
- Workflow de CI sem caminhos inexistentes (`git ls-files` confirma cada path do
  filtro).
- `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace`
  seguem limpos.
- Um commit: `chore: higiene pós-fase 2 (doc do CLI + CI órfã)` ou similar.
