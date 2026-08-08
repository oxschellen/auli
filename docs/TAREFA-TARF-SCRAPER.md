# TAREFA-TARF-SCRAPER — Acórdãos do TARF como coleção da entidade rs (fase 1: só a árvore .md)

**Leitura prévia obrigatória:** `DISCOVERY-TARF.md` e `ESTUDO-TARF-formato.md` (na raiz do repo).

**Escopo desta TAREFA:** implementar a COLETA dos acórdãos do TARF-RS e a materialização da árvore
`data/rs/docs/tarf/*.md` no formato .md dos pareceres, com a `## sinopse` preenchida pelo próprio
scraper (SEM LLM). **Nada além disso**: nenhum toque em update/packs/rotas/frontend/MCP — o
encanamento (`TipoJurisprudencia`, `preparar_jurisprudencia` etc.) é TAREFA futura, condicionada ao
`RELATORIO-verificacoes-TARF.md`. A árvore gerada agora é inerte para o resto do sistema (o update
não lê `docs/tarf/`) e 100% compatível com o desenho aprovado no estudo.

---

## Decisões fechadas (não rediscutir na implementação)

- **D-TARF-1 (onde vive):** módulo novo `tarf.rs` dentro de `crates/scrapers/auli-scraper-rs`
  (doutrina D-F2.1: um crate binário por entidade; TARF é RS). CLI ganha o comando `tarf`:
  `auli-scraper-rs [--usecache] tarf`. `tarf` NÃO entra no `all` (coleção nova, opt-in explícito,
  como `pareceres`); retorna antes da mensagem de snapshot, como `pareceres` faz.
- **D-TARF-2 (fonte):** API JSON do portal `tarf.sefaz.rs.gov.br` ("Legislacao 3.0"):
  - Listagem: `GET /api/PesquisaDocumentos?TamanhoPagina=&NumeroPagina=&grupos=14543`
    (`grupos` minúsculo; `NumeroPagina` começa em 1). Resposta: `totalItens`, `totalPaginas`,
    `resultado[] { id (GUID), titulo, numero, grupo, area, dataDocumento, dataCriacao,
    dataAtualizacao, ementa (vem vazia — ignorar) }`.
  - Corpo: `GET /api/documentos/{id}/DocumentoHtml?NumeroPagina=N&TamanhoPagina=4000`
    (`TamanhoPagina` aceito: 10–4000; pagina DISPOSITIVOS). Resposta no mesmo envelope
    (`totalItens`, `resultado[] { id, texto }`).
  - Escopo: SÓ o grupo `14543` ("ACÓRDÃOS A PARTIR DE 2005"). Outros grupos/áreas ficam fora.
- **D-TARF-3 (identidade e dedup):** identidade do documento = `id` (GUID). Guarda primária,
  sempre dinâmica: `GUIDs únicos coletados na listagem == totalItens` informado pelo portal
  (`bail!` se divergir — nunca hardcodar o total; hoje ~22.590). Colisão de slug de `numero` é
  detectada pelo contrato (`escrever_lote_se_ausente*`), como nas demais coleções.
- **D-TARF-4 (mapeamento para o formato dos pareceres):**
  - `numero` (frontmatter) ← `titulo` da listagem (ex.: `ACÓRDÃO 510/24` → slug `acordao-510-24`),
    espelhando o SC (`CONSULTA COPAT nº 0037/26`).
  - `assunto` ← a EMENTA extraída do cabeçalho tabelado do corpo (ver D-TARF-6). NÃO usar o campo
    `ementa` da listagem (vem vazio).
  - `link` ← `https://tarf.sefaz.rs.gov.br/documento/{id}` (deep link confirmado; concatenação
    pura, zero requisições extras).
  - `## sinopse` ← parágrafos de fundamentação do cabeçalho (D-TARF-6); fallback = a própria
    ementa quando não houver fundamentação.
  - `## corpo` ← texto integral do(s) dispositivo(s), INCLUINDO o cabeçalho tabelado convertido
    em texto (recorrentes, processo, auto de lançamento etc. viram texto pesquisável — sem
    frontmatter novo).
  - `sinopse_info` = `None`. Justificativa em doc-comment: `sinopse_info` registra proveniência
    LLM, que aqui não existe; `docs/tarf/` está fora do alcance do passo
    `auli-collections <id> sinopse` (que varre só `docs/pareceres/`), então não há risco de o
    doc ser tratado como pendente por quem gera sinopse.
- **D-TARF-5 (dispositivos):** NÃO assumir `totalItens == 1` no `DocumentoHtml`. Ler
  `totalItens`; se > 1, paginar (`TamanhoPagina=4000`) e concatenar os `texto` na ordem
  devolvida. Truncamento silencioso é proibido (mesma doutrina do invariante).
- **D-TARF-6 (extração da ementa e da fundamentação):** parsing ESTRUTURAL da primeira `<table>`
  do texto do dispositivo, por linhas `<tr>`:
  - A linha cuja célula-rótulo (1ª `<td>`, após strip de tags e colapso de espaços) casa com
    `EMENTA` (case-insensitive, dois-pontos opcionais) fornece a ementa: TODO o texto das células
    restantes daquela `<tr>` (juntar os `<p>`, colapsar espaços/`&nbsp;` numa string só —
    "célula inteira", nunca "primeiro parágrafo").
  - As `<tr>` SEGUINTES cuja célula-rótulo é vazia (`&nbsp;`/whitespace) fornecem os parágrafos
    de fundamentação (concatenados com quebras de parágrafo), até aparecer uma linha com rótulo
    não-vazio ou a tabela acabar. Âncora estrutural, não textual — robusta às variações já
    observadas (141/26: recursos ex officio + voluntário, 3 recorrentes incluindo FAZENDA
    ESTADUAL, "RECORRIDO(s): os mesmos").
  - **Sem fundamentação** ⇒ `## sinopse` = ementa (fallback, contado e relatado).
  - **Sem linha EMENTA** ⇒ ANOMALIA: não emite `.md` (nunca gerar documento com `assunto` vazio),
    registra `numero`, `id` e link em `data/rs/raw/tarf-anomalias.txt` e no relatório final. A
    coleta continua; a decisão sobre anômalos é humana.
- **D-TARF-7 (HTML→texto):** o `texto` do dispositivo embute um bloco `<style>` próprio (classe
  `tarfHack`) e um wrapper `<span class="tarfHack">`. A conversão remove `<style>...</style>`,
  `<script>...</script>`, comentários e tags, com quebras nos fechamentos de bloco
  (`</p>`, `</tr>`, `<br>`, ...), decodifica entidades e normaliza espaços — reutilizar
  `auli_scraper_kit::{clean, clean_decoded, decode_entities}` onde couber; regexes locais no
  módulo (como o SC faz) onde não couber. Verificar por teste que NENHUM fragmento de CSS
  (`tarfHack`, `{`, `text-align`) vaza para o corpo do `.md` de fixture.
- **D-TARF-8 (incremental):** "existe ⇒ pula" da árvore `.md` (doutrina G5, idêntica aos
  pareceres): a listagem é sempre re-percorrida por inteiro (barata: ~46 GETs a
  `TamanhoPagina=500`), e o corpo só é buscado para itens cujo slug ainda não existe em
  `data/rs/docs/tarf/`. `dataAtualizacao` é IGNORADA na v1 — decisão documentada em doc-comment:
  correção de conteúdo não chega pelo produtor (mesmo corolário do `escrever_se_ausente`);
  reversível por remoção manual do `.md` + recoleta.
- **D-TARF-9 (rede e cortesia):** UA institucional `AuliBot/0.1 (+https://github.com/oxschellen/auli; <email>)`
  (o mesmo do SC), cortesia de 1 s entre requisições, retry/backoff do kit
  (`http::get_string` + `GetOpts`), cache agressivo em `data/rs/raw/cache/tarf/` (módulo
  `cache` do kit; `--usecache` respeitado — coleta 100% offline do cache quando possível).
  Doc-comment ROBOTS/ACESSO no topo do módulo, no molde do SC: registrar o que o
  `https://tarf.sefaz.rs.gov.br/robots.txt` responde (verificar durante a implementação),
  que os acórdãos são documentos públicos do portal e que a coleta atende demanda institucional
  do NAVI. Carga inicial: ~22.6k corpos ≈ 6h15 a 1 req/s — o "existe ⇒ pula" + cache tornam o
  job retomável por natureza; nenhum mecanismo extra de resume é necessário.
- **D-TARF-10 (toque único no contrato):** `mddoc::escrever_lote_se_ausente` grava com sinopse
  hardcoded `None` (pareceres nascem pendentes). Adicionar ao `auli-contract`:

  ```rust
  /// Variante do lote que aceita sinopse por documento. Produtores cuja sinopse é
  /// autorada na coleta (TARF: fundamentação do cabeçalho) emitem por aqui; os de
  /// pareceres continuam emitindo pendente pela função existente.
  pub fn escrever_lote_se_ausente_com_sinopse(
      dir: &std::path::Path,
      docs: &[(DocHeader, Option<String>, String)], // (header, sinopse, corpo)
  ) -> Result<(usize, usize)>
  ```

  A checagem de colisão de slug é a MESMA (extrair para um helper interno ou reimplementar a
  existente como wrapper fino da nova com sinopse `None` — sem mudança de comportamento; os
  testes atuais do mddoc permanecem verdes, byte a byte). `escrever_se_ausente` ganha (ou
  delega a) uma variante interna com `Option<&str>` de sinopse; escrita atômica preservada.
  `mddoc.rs` dos pareceres NÃO muda em mais nada.

## Fases

**T0 — Acesso responsável.** Verificar e transcrever o `robots.txt` do portal no doc-comment
ROBOTS/ACESSO do módulo (molde do `auli-scraper-sc/src/pareceres.rs`). Confirmar no navegador ou
via curl os dois endpoints com os exemplos do DISCOVERY (smoke test manual, sem código).

**T1 — Contrato.** Implementar D-TARF-10 no `auli-contract` com testes: (a) lote misto
(com e sem sinopse) round-trip via `parse_doc`; (b) colisão de slug continua `bail!`;
(c) a função antiga produz bytes idênticos aos de antes (trava de regressão).

**T2 — Esqueleto do módulo.** `tarf.rs` no `auli-scraper-rs`: registrar no `main.rs`
(`mod tarf;` + braço `"tarf"` no match + help atualizado), constantes (URLs, `GRUPO_ACORDAOS:
&str = "14543"`, `DOCS_DIR = "../data/rs/docs/tarf"`, `CACHE_DIR = "../data/rs/raw/cache/tarf"`,
UA, cortesia), structs de desserialização da listagem e do envelope (serde; campos além dos
usados podem ser ignorados — `#[serde(default)]` onde fizer sentido, mas `id`, `titulo` e
`totalItens` são obrigatórios).

**T3 — Listagem.** Paginação completa do grupo 14543 (`TamanhoPagina=500`; se o portal recusar,
reduzir e comentar o motivo). Acumular em mapa por GUID; ao fim, guarda D-TARF-3
(`únicos == totalItens`, com `bail!` explicando os dois números). Log de progresso a cada página.

**T4 — Corpo.** Para cada item inédito (slug ausente na árvore): `DocumentoHtml` com paginação
D-TARF-5, cache-first, cortesia 1 s só em requisição real (cache hit não dorme — como o resto da
frota). Conversão HTML→texto D-TARF-7.

**T5 — Extração e emissão.** Parser D-TARF-6 sobre o HTML BRUTO do dispositivo (antes da
conversão para texto — a estrutura `<table>/<tr>/<td>` é a âncora). Montar
`(DocHeader, Option<String>, String)` conforme D-TARF-4 e emitir com
`escrever_lote_se_ausente_com_sinopse`. Anomalias para o arquivo + stderr.

**T6 — Relatório e verificação.** Relatório final no padrão da frota: total da listagem,
inéditos, pulados, emitidos, fallbacks-ementa, anomalias, cache hits/misses. Rodada de
verificação manual (Carlos): coletar com `--limit`? NÃO — sem flag nova; para a rodada de
verificação, instruir no PR a rodar com a árvore vazia e interromper após alguns minutos
(o incremental retoma), conferindo 510/24 e 141/26 contra o portal.

## Testes obrigatórios (fixtures no repo, sem rede)

1. Fixture com o HTML real do dispositivo do **510/24** (do DISCOVERY): ementa extraída ==
   linha ICMS/INDUSTRIALIZAÇÃO...; sinopse == os 5+ parágrafos de fundamentação; corpo contém
   "RECORRENTE" e "RELATÓRIO" e NÃO contém `tarfHack` nem `{`.
2. Fixture sintética no formato do **141/26** (rótulos variáveis, "os mesmos", 3 recorrentes):
   parser não quebra; ementa e fundamentação corretas.
3. Fixture SEM parágrafos de fundamentação ⇒ sinopse == ementa (fallback).
4. Fixture SEM linha EMENTA ⇒ anomalia (nenhum `.md`), demais documentos do lote emitidos.
5. Ementa multiparágrafo na MESMA célula ⇒ concatenada inteira no `assunto` (regra
   "célula inteira").
6. Round-trip: `.md` emitido parseia via `mddoc::parse_doc` com sinopse `Some(..)` não-vazia
   (satisfaz por construção a guarda `recusar_pareceres_sem_sinopse` do update futuro).

## Critérios de aceite

- `cargo test -p auli-contract -p auli-scraper-rs` e `cargo clippy -D warnings` verdes.
- Nenhum arquivo fora de `auli-contract/src/mddoc.rs` (D-TARF-10) e
  `crates/scrapers/auli-scraper-rs/` é tocado.
- `auli-scraper-rs tarf` interrompido no meio e re-executado converge sem duplicar nem refazer
  trabalho (cache + existe⇒pula).
- Amostra manual: `data/rs/docs/tarf/acordao-510-24.md` confere com o portal
  (`https://tarf.sefaz.rs.gov.br/documento/ee87f4bc-12b9-4666-02cb-08dee4034157`).
