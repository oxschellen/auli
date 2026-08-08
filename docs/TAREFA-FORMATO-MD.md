# TAREFA-FORMATO-MD — Unificação do vocabulário de armazenamento das árvores .md

**Leitura prévia:** `ESTUDO-colecoes-trait.md` (Adendo 3 — a tabela de papéis funcionais é a base
deste formato) e `TAREFA-MARCADORES.md` (o degrau anterior, de serving).
**Posição na fila:** TERCEIRA — somente após o merge do scraper TARF e da TAREFA-MARCADORES.
O scraper TARF NÃO é alterado: ele emite no formato atual dos pareceres e sua árvore é
convertida junto com as demais (D-FMT-8).

**Escopo:** unificar o formato dos `.md` das árvores `data/<uf>/docs/*/` num vocabulário único,
com UM parser no contrato substituindo os três irmãos; adaptar emissores (scrapers/derive/
sinopse) e consumidores (update, downloads, manuais, curso); conversor one-shot para as
árvores append-only. **Nada de re-scrape; vetores inalterados** (fórmulas byte-idênticas —
o update regenera packs, e o re-embed mecânico produz os mesmos vetores; custo de minutos,
medido baixo na TAREFA-TEMPOS).

**Fora do escopo:** marcadores de serving (já feitos), prefixos P:/R: do embed FAQ (gate P5),
trait `Colecao`/struct `Documento` em código (fase 3 — mas este formato é a espinha dela),
qualquer mudança de conteúdo dos documentos.

---

## O formato unificado (D-FMT-1)

```markdown
---
titulo: <identificação humana — numero | titulo do serviço | pergunta>
trilha: <classificação — "" | "tipo | classe" | origin>
ementa: <síntese junto ao título — assunto | "" | "">
link: <URL oficial>
orgao: <opcional — passthrough de serviços; ausente nas demais>
resumo_modelo: <opcional — trio de proveniência, só jurisprudência com sinopse LLM>
resumo_prompt_versao: <opcional>
resumo_gerada_em: <opcional>
---

## resumo
<síntese embedável — sinopse | (ausente) | (ausente)>

## corpo
<conteúdo integral — corpo | descrição | resposta>
```

## Decisões fechadas

- **D-FMT-2 (mapeamento por coleção):**
  | coleção | titulo | trilha | ementa | ## resumo | ## corpo |
  |---|---|---|---|---|---|
  | pareceres/tarf | numero | *(vazio)* | assunto | sinopse | corpo |
  | serviços | titulo | `tipo \| classe` (pré-composta, byte-idêntica ao breadcrumb do compose) | *(vazio)* | *(ausente)* | descrição |
  | faqs | pergunta | origin | *(vazio)* | *(ausente)* | resposta |

  Campos vazios são permitidos e gravados como valor vazio (chave presente — o parser não
  distingue coleção por ausência de chave). `## resumo` é seção OPCIONAL, como `## sinopse`
  hoje.
- **D-FMT-3 (a única concessão):** `orgao` permanece como chave opcional de passthrough.
  Não entra em funil nenhum (verificado no Adendo 3), mas tem valor para quem baixa os zips.
  Documentar no parser como "metadado de coleção, não consumido pelos funis". Nenhuma outra
  chave específica é admitida.
- **D-FMT-4 (proveniência):** o trio `sinopse_modelo/sinopse_prompt_versao/sinopse_gerada_em`
  vira `resumo_*` (as três andam juntas; estado misto segue erro). O passo
  `auli-collections <id> sinopse` passa a gravar `## resumo` + trio `resumo_*`; a detecção de
  pendente continua sendo a AUSÊNCIA da seção (inalterada em lógica).
- **D-FMT-5 (parser único):** `mddoc_unificado` (nome final a critério; pode assumir o nome
  `mddoc`) no `auli-contract` parseia/renderiza o formato D-FMT-1 com round-trip garantido
  por teste, escrita atômica e as mesmas checagens (colisão de slug no lote, numero→slug).
  `mddoc_servico.rs` e `mddoc_faq.rs` são REMOVIDOS ao fim da migração (não deprecados —
  removidos; quem precisar do formato velho tem o git). Os testes de round-trip e os que
  congelam comportamento migram para o parser único.
- **D-FMT-6 (fórmulas preservadas byte a byte):** o compose único do Adendo 3
  (`trilha\ntitulo\nementa\nresumo`, pulando vazios) substitui os três composes, e os testes
  de congelamento provam a igualdade byte a byte com as fórmulas atuais para: jurisprudência
  (numero+assunto+sinopse), serviços (tipo|classe+titulo+snippet300 — o snippet é computado
  no compose a partir do `## corpo`, como hoje) e FAQs na fórmula VIGENTE (origin+pergunta ⇒
  trilha+titulo com ementa/resumo vazios). `STRATEGY_VERSION` de cada coleção NÃO muda nesta
  tarefa. A troca da fórmula de FAQ continua sendo assunto exclusivo do gate P5.
- **D-FMT-7 (guardas por política):** `recusar_*_sem_sinopse` vira "recusar jurisprudência
  sem `## resumo`" — aplica-se a pareceres e tarf; serviços/faqs não têm a exigência
  (política por coleção, não por formato).
- **D-FMT-8 (conversor one-shot):** binário em `tools/` (fora dos crates de produção) que
  varre `data/*/docs/{pareceres,tarf}/*.md` no formato velho e regrava no novo — renomeia
  chaves, `## sinopse`→`## resumo`, preserva conteúdo byte a byte nos VALORES. Idempotente
  (arquivo já no formato novo ⇒ pulado), escrita atômica, e com teste de equivalência:
  parse(velho) e parse_unificado(novo) devem produzir os mesmos campos. Serviços e FAQs NÃO
  passam pelo conversor: o próximo `process`/re-scrape regenera as árvores (delete+rebuild)
  já no formato novo.
- **D-FMT-9 (roteiro de migração, nesta ordem, numa janela só):**
  1. merge do código (parser único + emissores + conversor);
  2. rodar o conversor sobre pareceres (rs/sc/sp/pr) e tarf;
  3. re-process de serviços (todos os estados) e faqs (rs) — árvores regeneradas;
  4. `auli update` de todas as entidades (docs_hash mudou para todas); conferir nos logs que
     as contagens de documentos são idênticas às anteriores, coleção a coleção;
  5. regenerar zips de downloads; deploy.
  Rollback antes do passo 2 = git revert; depois, restaurar as árvores append-only do backup
  (fazer backup de `docs/pareceres` e `docs/tarf` ANTES do conversor é pré-condição do
  roteiro).
- **D-FMT-10 (consumidores de texto):** READMEs dos zips descrevem o formato novo e ganham
  uma linha "formato v2 (ago/2026); a versão anterior usava chaves por coleção — ver
  histórico do repo"; manuais MCP conferidos (só mudar se citarem chaves); o pipeline do
  curso (Claude Code lê os .md de serviços) é avisado no próprio doc do curso com uma linha
  sobre as chaves novas.

## Fases

**F1 — Parser único** no contrato + testes (round-trip, colisão, campos vazios, `orgao`
opcional, trio `resumo_*` completo-ou-ausente).
**F2 — Compose único** + testes de igualdade byte a byte com as três fórmulas atuais
(D-FMT-6) — estes testes são a alma da tarefa: sem eles, nada de merge.
**F3 — Emissores**: `servicos::process` (materialização), `derive_faqs`, passo `sinopse`,
`docs.rs` do kit (emissão de jurisprudência) — todos gravando D-FMT-1.
**F4 — Update**: `preparar_*` lendo o parser único; guardas D-FMT-7; remoção dos parsers
irmãos e do fallback JSON legado de serviços se ainda existir (fecha a pendência D9 antiga).
**F5 — Conversor** (D-FMT-8) + teste de equivalência.
**F6 — Consumidores de texto** (D-FMT-10).
**F7 — Migração executada pelo Carlos** (D-FMT-9), com verificação manual: uma consulta de
chat por coleção no RS + um parecer e um acórdão conferidos contra o portal + um zip aberto.

## Critérios de aceite

- `cargo test` e `clippy -D warnings` verdes; nenhum teste de congelamento de fórmula foi
  afrouxado — os byte-a-byte de D-FMT-6 passam.
- `grep` não encontra `mddoc_servico`, `mddoc_faq`, `## sinopse`, `numero:`/`assunto:`/
  `pergunta:`/`origin:`/`url:` como chaves de frontmatter em código vivo (histórico em
  doc-comments é aceitável).
- Após a migração: contagens por coleção idênticas às pré-migração; espiar 3 `.md` de cada
  coleção; os vetores dos packs regenerados batem com os anteriores (amostra: comparar
  distâncias de uma consulta fixa antes/depois).
