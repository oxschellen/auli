# Integração: auli-scraper-ba

Código escrito contra o workspace real (padrão `auli-scraper-pr`, schema v3), com
**fixtures reais** capturadas via view-source (listagem completa + ficha `senha`).

## Status de verificação

- ✅ Parsing validado: 5/5 testes verdes contra as fixtures reais (harness edition 2021,
  `scraper 0.19` — API idêntica à usada; sandbox sem rustc ≥1.85 para edition 2024).
- ✅ Achado do parsing real: dos 206 hrefs `?id=` no fonte, **2 estão comentados**
  (`<!--li ...-->`, serviços desativados) — catálogo ativo = **204**. Regex contaria 206;
  o parser HTML acerta.
- ⚠️ Não linkado contra `auli-contract`/`auli-scraper-kit` reais aqui; gate verde de
  verdade = `cargo test -p auli-scraper-ba` no desktop.
- ⚠️ Só a ficha `senha` foi inspecionada; fichas de outros públicos ("Serviços às
  Empresas"?) são hipótese mapeada com fallback (D-BA1) — o real-scrape confirma.

## Passos (em `~/Desktop/auli_new`, workspace `auli-server/`)

1. `tar xzf auli-scraper-ba.tar.gz` na raiz do workspace → `crates/auli-scraper-ba/`.
2. Adicionar `"crates/auli-scraper-ba",` aos `members` do `Cargo.toml`.
3. `cargo test -p auli-scraper-ba` — deve dar 5/5.
4. Real-scrape: `cargo run -p auli-scraper-ba -- servicos`
   (~205 GETs, cortesia 500ms ≈ 2min; cache em `../data/ba/raw/cache/servicos/`).
5. Inspecionar `../data/ba/ba-servicos-snapshot.json`: 204 itens; conferir a distribuição
   de públicos impressa no console (rótulos desconhecidos geram warning D-BA1 — se
   aparecer algum, adicionar ao mapa `publico_conhecido`).
6. Amostrar 3 fichas do snapshot contra o site; depois `auli-collections ba`.

## Decisões embutidas (D-BA)

- **D-BA1** público por ficha (`panel-title`), mapa de rótulos conhecidos
  (Cidadãos/Empresas/Municípios) + fallback slugificado com warning.
- **D-BA2** classe = subtítulo `<small>` do título da ficha (ex.: "Requerimento");
  ausente → "Geral".
- **D-BA3** ficha que falha degrada (público Cidadãos, classe Geral, corpo vazio) com
  warning — não derruba a coleta.
- **D-BA4** etiqueta: UA de navegador, 500ms de cortesia, cache; robots restritivo,
  coleta de baixíssima frequência.
- Charset: guarda de runtime — UTF-8 válido passa; bytes inválidos caem para latin-1 com
  aviso (ASP clássico), nunca derrubam a coleta.
- Identidade `(link, titulo)` com título da **listagem** (canônico); a ficha fornece
  público/classe/corpo. Links do corpo normalizados `texto "url"` (padrão PR).
