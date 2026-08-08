# PEDIDO — Verificações de leitura para o estudo TARF

**Contexto:** leia antes `DISCOVERY-TARF.md` e `ESTUDO-TARF-formato.md` (na raiz do repo, ao lado das TAREFAs).

**Escopo:** SOMENTE leitura de código e relatório. NÃO escreva, NÃO modifique e NÃO crie nenhum arquivo de código. A saída é um único relatório em markdown respondendo às três perguntas abaixo, com caminhos de arquivo e números de linha.

## Pergunta 1 — O nome `Consulta` vaza em dados serializados?

Queremos saber se renomear a struct `Consulta` para `Jurisprudencia` é um refactor puro ou se afeta dados persistidos.

- Localize a struct `Consulta` no `auli-contract` e todos os pontos onde ela é serializada/desserializada (serde) ou onde seus dados são gravados: packs de embeddings, manifesto, JSONs do frontend, snapshots, árvore .md.
- Responda: o **nome da struct** (ou algum identificador derivado dele) aparece em algum formato gravado em disco ou trafegado (JSON, pack, rota, manifesto)? Ou só os **nomes dos campos** (`numero`, `assunto`, `link`, `sinopse`, `corpo`) aparecem?
- Conclusão esperada: "rename é refactor puro" OU "rename exige `serde(rename)`/cuidado em X, Y, Z".

## Pergunta 2 — Onde `assunto` é assumido curto?

A ementa dos acórdãos do TARF (que ocuparia o campo `assunto`) é longa e em CAIXA ALTA, diferente do assunto dos pareceres.

- Mapeie todos os usos do campo `assunto` da `Consulta`: compose do text_to_embed, prompt/etapa de sinopse, JSONs de metadados do frontend, componente da tab Pareceres, MCP (`buscar_pareceres`/`obter_parecer`), logs.
- Para cada uso, diga se um `assunto` de ~500–1500 caracteres causaria problema (truncamento, layout, custo de tokens, limite validado) ou se passa sem ajuste.

## Pergunta 3 — Onde o kind "pareceres" está hardcoded?

Para dimensionar a parametrização por `TipoJurisprudencia` (§4–5 do estudo).

- Liste as ocorrências da string/identificador "pareceres" fora do domínio servicos/faqs: `preparar_pareceres`, rotas, nomes de coleção/pack, manifesto, registry, MCP, frontend (ids de aba, labels), scripts (build-packs.sh, build-frontend-public.sh).
- Classifique cada uma: (a) resolvida pela parametrização `tipo.kind()`/`tipo.dir()` proposta no estudo; (b) precisa de instância nova (ex.: aba do frontend, manuais); (c) não precisa mudar.

## Formato da resposta

Um arquivo `RELATORIO-verificacoes-TARF.md` com uma seção por pergunta, cada achado como `caminho/arquivo.rs:linha — descrição curta`, e a conclusão de cada pergunta em uma frase no topo da seção.
