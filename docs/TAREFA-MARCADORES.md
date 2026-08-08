# TAREFA-MARCADORES — Unificação dos invólucros do bloco RAG e das instruções de marcador nos prompts

**Leitura prévia:** `ESTUDO-colecoes-trait.md`, Adendo 3 (§3.3 e §3.4) — a decisão e sua justificativa.
**Pré-requisito de fila:** entrar SOMENTE depois do merge da TAREFA-TARF-SCRAPER (sem paralelismo).

**Escopo:** unificar os invólucros externos do contexto RAG num único template
`## documento {i}: {rotulo}` e atualizar a frase de marcador nos prompts. Mudança de SERVING
apenas — nenhum vetor, pack, snapshot ou árvore .md muda; zero re-embed.

**Fora do escopo (não tocar):** o template INTERNO dos blocos (`## pergunta` / `## resposta` /
`Link:` — já é unificado e permanece byte a byte); as fórmulas de `text_to_embed` e qualquer
`STRATEGY_VERSION`; os prefixos `P:`/`R:` do embed de FAQ (gate P5); a struct `Documento`
(fase 3); qualquer coisa da TAREFA-TARF-SCRAPER.

---

## Decisões fechadas

- **D-MARC-1 (o template único):** os quatro invólucros de `rag.rs` —
  `\n## servico\n{i}\n{doc}\n`, `\n// Resultado: {i}\n{doc}\n`, `\n## PARECER\n{i}\n{bloco}\n`
  e `\n## PARECER RELACIONADO\n{i}\n{bloco}\n` — são substituídos por UM:

  ```text
  \n## documento {i}: {rotulo}\n{conteudo}\n
  ```

- **D-MARC-2 (rótulos):** o rótulo é DADO, não template. Tabela desta tarefa:
  `"Serviço"`, `"FAQ"`, `"Parecer"`, `"Parecer relacionado"`. O rótulo `"Acórdão TARF"` fica
  DECLARADO na mesma tabela/const com comentário `// fase 2 — serving ainda não liga esta
  coleção`, para o ponto de extensão já existir nomeado.
- **D-MARC-3 (numeração):** a numeração continua POR COLEÇÃO dentro do contexto (serviços
  1..n, depois FAQs 1..m; pareceres 1..k e relacionados 1..r) — como hoje. Motivo: a seção
  ADERÊNCIA do log de auditoria rotula por coleção+índice (`aderencia("...", ...)`) e a
  correspondência 1:1 com o bloco deve se manter sem tocar na aderência. Unifica-se o
  template, não a contagem.
- **D-MARC-4 (um render só):** as duas funções `montar_rag_servicos_faqs` e
  `montar_rag_pareceres` PERMANECEM com suas assinaturas (os call sites do chat e do MCP não
  mudam), mas internamente passam a delegar a um único
  `fn envolver(rotulo: &str, i: usize, conteudo: &str) -> String` que materializa D-MARC-1.
  A paridade chat×MCP segue automática (mcp.rs:334 chama a mesma `montar_rag_servicos_faqs`).
- **D-MARC-5 (travas de formato):** os testes que pinam byte a byte
  (`montar_rag_servicos_faqs_pina_o_formato`, `montar_rag_servicos_faqs_com_listas_vazias`,
  `montar_rag_pareceres_pina_o_formato`, `montar_rag_pareceres_anexa_secao_relacionados`,
  rag.rs ~853–900) são ATUALIZADOS no mesmo commit para pinar o formato NOVO, com o
  comentário do bloco (rag.rs:847) ganhando uma linha: a mudança de 2026-08 foi deliberada
  (TAREFA-MARCADORES) — as travas impedem mudança acidental, não decidida. Nenhuma trava é
  removida ou afrouxada.
- **D-MARC-6 (prompts):** em TODOS os arquivos de `data/prompts/` que instruem marcadores:
  - Estaduais (27, linha ~14): as DUAS linhas atuais — `### Cada serviço do texto inicia com
    o marcador: ## servico` e `### Cada serviço do texto inicia com o marcador: ## pergunta`
    (a segunda é um bug latente de copy-paste que esta tarefa corrige de graça) — viram:

    ```text
    ### Cada documento do contexto inicia com o marcador: ## documento (seguido do número e do tipo — Serviço ou FAQ).
    ### Dentro de cada documento, o título vem após o marcador ## pergunta e o conteúdo após o marcador ## resposta.
    ```

  - `*-pareceres.txt` (4): a linha `### Cada parecer inicia com o marcador: ## PARECER` vira:

    ```text
    ### Cada parecer inicia com o marcador: ## documento (seguido do número e do tipo — Parecer ou Parecer relacionado).
    ```

    A linha seguinte (`## pergunta`/`## resposta`) desses arquivos já está correta e não muda.
  - NADA além das linhas de marcador é alterado nos prompts (Resumo/Detalhes, links
    markdown, limites de palavras etc. ficam intocados).
  - `sinopse.txt`, `extracao*.txt` não instruem marcadores de contexto — não tocar.
- **D-MARC-7 (commit único):** código (rag.rs) + travas + os 31 prompts no MESMO commit.
  Motivo: os invólucros são gerados pelo código para todas as entidades de uma vez — prompt
  e contexto não podem divergir nem por um deploy. Rollback = `git revert` desse commit
  (nada persiste, nada re-embedda).

## Fases

**M1 — `envolver` + delegação.** Implementar D-MARC-1/D-MARC-4 em rag.rs; rótulos D-MARC-2
numa const/tabela local com o comentário da fase 2.

**M2 — Travas.** Atualizar os 4 testes de pinagem (D-MARC-5) com os bytes novos, escritos à
mão no assert (não gerados pela função sob teste — a trava só trava se o esperado for
literal). Acrescentar 1 teste novo pinando `envolver` isolada.

**M3 — Prompts.** D-MARC-6 nos 31 arquivos. Conferir com `grep` que nenhuma menção a
`## servico`, `// Resultado:` ou `## PARECER` sobrou em `data/prompts/`.

**M4 — Verificação de ponta a ponta (manual, Carlos).** Antes do deploy no VPS: rodada de
perguntas reais no RS (Serviços+FAQs E Pareceres), conferindo (a) qualidade das respostas
sem degradação perceptível, (b) links citados como antes, (c) o log de auditoria mostrando o
contexto novo com a seção ADERÊNCIA ainda correspondendo índice a índice. Só então deploy.

## Critérios de aceite

- `cargo test` e `cargo clippy -D warnings` verdes.
- `grep -rn "## servico\|// Resultado:\|## PARECER" crates/ data/prompts/` retorna apenas
  ocorrências históricas em comentários/docs (se houver) — nenhuma em código vivo ou prompt.
- O contexto de uma consulta de teste (log de auditoria) exibe
  `## documento 1: Serviço … ## documento 1: FAQ …` e, nos pareceres,
  `## documento 1: Parecer …` — com o template interno `## pergunta`/`## resposta`/`Link:`
  byte a byte como antes.
- MCP `consultar_servicos_faqs` devolve exatamente o mesmo bloco do chat (paridade
  preservada por construção — conferir com uma chamada manual).
- Nenhum arquivo fora de `crates/auli-cli/src/rag.rs` (e seus testes) e `data/prompts/*.txt`
  é tocado.
