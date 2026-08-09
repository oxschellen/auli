# ESTUDO-TARF-formato — Padronização do formato: pareceres + acórdãos TARF

**Data:** 08/08/2026
**Status:** ~~Nenhuma implementação~~ — **implantado** (PRs #127 e #132). O registro da fonte, que
era o `DISCOVERY-TARF.md`, foi absorvido em `docs/REGISTRO-scrapers.md#tarf` junto com o que a
implementação decidiu e o que só apareceu ao rodar.
**Pergunta do estudo:** o formato dos pareceres pode servir aos acórdãos do TARF, de modo que, depois do scraping, o código seja o mesmo até o frontend?

**Resposta:** sim — via struct genérica `Jurisprudencia` + enum `TipoJurisprudencia`, com os acórdãos gravados no formato .md dos pareceres byte a byte, dentro da entidade **rs** (nova coleção, não nova entidade).

---

## 1. Decisões de recorte

1. **TARF NÃO é uma nova entidade.** A via entidade-nova (uma "uf" chamada tarf) foi avaliada e **descartada**: maximizava o reuso por configuração, mas torcia a semântica (TARF ao lado de UFs; acórdãos com kind "pareceres"). Decisão: TARF é uma **coleção irmã** de servicos/faqs/pareceres dentro da entidade rs.
2. **O formato .md dos pareceres é adotado sem alteração** para os acórdãos: frontmatter `numero`/`assunto`/`link`, seções `## sinopse` e `## corpo`, nome do arquivo = slug do numero. Árvore fonte: `data/rs/docs/tarf/*.md`.
3. **Campos específicos do acórdão não ganham frontmatter.** Recurso, recorrentes, processo, auto de lançamento, decisão de 1ª instância ficam como texto no `## corpo` (que é onde o retrieval lê o conteúdo integral). Nada de esquema novo.

## 2. Mapeamento acórdão → formato dos pareceres

| Campo | Fonte no portal TARF | Obs |
|---|---|---|
| `numero` | "510/24" (do cabeçalho/titulo) | slug "510-24" como nome de arquivo; único no grupo (numeração anual sequencial) |
| `assunto` | EMENTA extraída do cabeçalho tabelado do `DocumentoHtml` | caixa alta, telegráfica — mesmo papel semântico da ementa dos pareceres; **atenção: é longa** (ver §6) |
| `link` | deep link do documento na SPA | rota a descobrir — provavelmente `/documento/{guid}` (ver §6) |
| `## corpo` | HTML→texto do dispositivo, **incluindo o cabeçalho tabelado** | recorrentes, processo etc. viram texto pesquisável |
| `## sinopse` | etapa LLM idêntica à dos pareceres | prompt é config (`data/prompts/sinopse.txt`), não código; variante para acórdãos, se precisar, continua sendo config |

**Pré-condição estrutural:** toda a diferença do portal morre no scraper `tarf-rs` — ele entrega o snapshot já no shape esperado (ementa extraída, HTML→texto, link resolvido). É exatamente para isso que a fronteira scraper/collections (formato snapshot) foi desenhada.

## 3. A struct genérica

### 3.1 Onde mora o `tipo` — Forma A descartada

**Forma A (descartada):** `tipo: TipoJurisprudencia` como campo da struct. Problemas: (a) se serializa no frontmatter, invalida os ~20k .md existentes de pareceres (RS/SC/SP/PR) ou exige default, e muda o hash da árvore/manifesto sem mudança de conteúdo; (b) redundância — coleções são homogêneas, todos os itens de `docs/tarf/` são Tarf.

**Forma B (adotada):** o tipo vive na **coleção**, não na linha. A struct é a `Consulta` de hoje renomeada, shape idêntico, sem campo novo.

### 3.2 Proposta

```rust
/// Uma peça de jurisprudência no formato .md padrão:
/// frontmatter numero/assunto/link + ## sinopse + ## corpo.
/// (é a antiga Consulta, renomeada — shape idêntico, árvores existentes intocadas)
pub struct Jurisprudencia {
    pub numero: String,
    pub assunto: String,   // ementa, no caso do TARF
    pub link: String,
    pub sinopse: String,
    pub corpo: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TipoJurisprudencia {
    Pareceres,
    Tarf,
}

impl TipoJurisprudencia {
    /// kind de domínio usado em rotas, packs e coleções
    pub fn kind(self) -> &'static str {
        match self {
            Self::Pareceres => "pareceres",
            Self::Tarf => "tarf",
        }
    }
    /// subdiretório da árvore fonte: data/<uf>/docs/<dir>/
    pub fn dir(self) -> &'static str { self.kind() }

    /// rótulo humano para frontend/manuais
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::Pareceres => "Pareceres",
            Self::Tarf => "Acórdãos TARF",
        }
    }
    pub fn todos() -> &'static [Self] { &[Self::Pareceres, Self::Tarf] }
}
```

## 4. Efeito em cascata (o "mesmo código")

- `Table<Jurisprudencia>` única para as duas coleções.
- `mddoc.rs` **intocado** — parseia `Jurisprudencia` de qualquer diretório.
- `compose_jurisprudencia_text_to_embed` única (a atual, renomeada).
- `preparar_pareceres` → `preparar_jurisprudencia(tipo: TipoJurisprudencia)`; o update itera `TipoJurisprudencia::todos()`, pulando diretório ausente com o ⏭️ padrão (SC/SP/PR não têm `docs/tarf/` e seguem funcionando).
- Sinopse, packs, serving, downloads: dirigidos por `tipo.dir()`/`tipo.kind()`.
- **Packs separados por kind** — nunca misturar acórdãos no pack de pareceres (retrieval de um contaminaria o outro).
- Extensão futura (Súmulas TARF, COPAT etc.) = **uma variante no enum**; o `match` exaustivo faz o compilador apontar cada ponto a preencher.
- MCP: parâmetro `colecao` validado contra o enum em `buscar_*`/`obter_*`, em vez de ferramentas gêmeas.
- Downloads: o zip do rs ganha a pasta `tarf/` de graça (a rotina descobre por diretório — decisão de jul/2026).

## 5. Encanamento que ainda é código (honestidade do estudo)

Pontos onde "pareceres" está hoje hardcoded como nome de coleção e que precisam da parametrização acima ou de uma instância nova:

1. Loop do `update` (a parametrização do §4 resolve).
2. Manifesto/packs: a coleção nova precisa existir e ganhar pack próprio.
3. Chat: seletor de fonte da caixa de mensagem (entrega 2 do redesenho) passa a incluir a coleção TARF.
4. Frontend: aba "Acórdãos TARF" no grupo Acervo da sidebar; a listagem pode reutilizar a componente da tab Pareceres apontando para outro JSON de metadados.
5. MCP: `listar_entidades` reporta o kind novo; `buscar/obter` ganham o parâmetro `colecao`.
6. Manuais MCP: totais e rótulos mantidos à mão precisam da coleção nova.

## 6. Pendências e ressalvas

1. **Generalização deliberada, contrária à doutrina dos irmãos** (mddoc_servico/mddoc_faq não generalizados): justificada porque aquela doutrina cobre *formatos divergentes* — aqui os formatos são **idênticos por decisão**, então o genérico não abstrai diferença nenhuma, só remove duplicação futura. Fronteira do genérico: `Jurisprudencia` cobre a família formato-pareceres; servicos/faqs ficam fora, como estão.
2. **Rename Consulta→Jurisprudencia é o segundo rename da struct** (Parecer→Consulta foi o primeiro). Conferir se `Consulta` vaza em dados serializados (nomes de campo em JSON/packs): se não vaza, refactor puro; se vaza, `serde(rename)` ou manter `Consulta` com doc-comment.
3. **`assunto` longo:** a ementa do TARF é comprida e em caixa alta. Conferir lugares que assumem assunto curto (tab do frontend, compose, sinopse prompt).
4. **Deep link:** descobrir a rota da SPA para um documento (abrir um acórdão no navegador e olhar a URL).
5. **Mutabilidade:** acórdãos têm `dataAtualizacao` (pareceres são append-only). v1 pode ignorar atualizações (diff de conjuntos por id), como decisão documentada e reversível por re-scrape.
6. **Dados pessoais no corpo** (recorrentes pessoas físicas): documento público, mas considerar na sinopse LLM e na exposição em chat/frontend.
