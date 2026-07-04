# Integração: auli-scraper-pe

Código escrito contra o workspace real (clonado de `oxschellen/auli`, commit `c01eabe`,
schema de snapshot **v3**), no padrão do `auli-scraper-pr` (ureq + `scraper`, sem headless).

## Status de verificação

- ✅ Lógica de parsing validada: 6/6 testes verdes contra fixture derivada do HTML real
  (harness edition 2021 com `scraper 0.19`, API idêntica à usada — o sandbox não tem
  rustc ≥1.85 para compilar os crates edition 2024).
- ⚠️ O crate **não foi compilado contra `auli-contract`/`auli-scraper-kit` reais** aqui.
  O gate verde de verdade é o `cargo test -p auli-scraper-pe` no desktop.
- ⚠️ Fixture é estrutural (menu da masterpage capturado de uma página de notícia).
  O real-scrape confirma a home.

## Passos (em `~/Desktop/auli_new`)

1. Extrair `auli-scraper-pe.tar.gz` na raiz do workspace (`auli-server/`):
   `tar xzf auli-scraper-pe.tar.gz` → cria `crates/auli-scraper-pe/`.
2. Adicionar `"crates/auli-scraper-pe",` aos `members` do `Cargo.toml` do workspace
   (depois de `auli-scraper-mg`).
3. `cargo test -p auli-scraper-pe` — deve dar 6/6.
4. Real-scrape: `cargo run -p auli-scraper-pe -- servicos`
   → grava `../data/pe/pe-servicos-snapshot.json` (1 GET na home, com cache em
   `../data/pe/raw/cache/servicos/`).
5. Inspecionar o snapshot; esperado: ~40 links únicos, e-Fisco com 3 ocorrências,
   DAE 10/20 e GNRE com 2.
6. `auli-collections pe` para derivar os artefatos.

## Decisões embutidas (revisar)

- **D-PE1**: fase 1 = só o menu (descrições vazias). Fase 2 (corpo de
  `div.ms-rtestate-field` das páginas `/Servicos/...`) fica para depois, se o RAG precisar.
- **D-PE2**: item de topo sem subgrupo → `classe = "Geral"`; sob subgrupo → texto do
  header (ex.: `"Tributos"`).
- **D-PE3**: header de subgrupo com href real (caso "Tributos Transferências
  Constitucionais" em municípios) vira item também, com classe "Geral".
- **D-PE4**: UA de navegador (padrão dos demais scrapers), volume mínimo — o robots.txt
  do portal é restritivo a crawlers genéricos; 1 GET por rodada + cache.
- Nomes de público canônicos: `Cidadãos` / `Empresas` / `Municípios` (títulos do portal
  "Para cidadãos" etc. são usados só para casar os blocos; bloco ausente = erro duro).
- Links externos (efisco, gnre, arevirtualws) são serviços válidos e preservados como estão.
