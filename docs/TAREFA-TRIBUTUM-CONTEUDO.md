# TAREFA-TRIBUTUM-CONTEUDO — o catálogo real entra, e os links passam a ser conferidos a cada trimestre

**Data:** 22/08/2026
**Base:** `801e288` (pós #163, #165, #166). O schema de `tributum.json` e os tipos em `src/tributum/types.ts` são os dessa base.
**Doutrina:** o repositório vence.

---

## 1. O que é

O Tributum sai do placeholder. O arquivo `tributum.json` que acompanha esta TAREFA (entregue junto, já validado: JSON bem-formado, ids únicos por estante) substitui o atual. Ele tem:

| Estante | Itens | `placeholder:false` |
|---|---|---|
| artigos | 26 | 26 |
| instituicoes | 19 | 19 |
| dados | 20 | 20 |
| analises | 6 | 5 (o exemplo hospedado fica como está) |

Cada item com `placeholder:false` teve o link **aberto e confirmado em 22/08/2026** por duas rodadas de pesquisa. Itens cujo link não foi confirmado por fetch integral ficaram de fora — não entraram com `placeholder:true`, entraram em lugar nenhum.

**O `curadores` não muda.** Está preenchido (Wagner Bomfim nas quatro estantes); o arquivo entregue preserva-o.

## 2. Decisões

- **D-TRIB-17 — Todo link do catálogo é revalidado trimestralmente.** Portais `gov.br` rodam Plone e trocam UIDs de página sem redirecionar; repositórios Ipea mudam de `handle` para `items/uuid`; revistas OJS movem volumes. Um catálogo de links que não é conferido apodrece em silêncio, e um link morto numa estante editorial é pior que uma estante vazia.
- **D-TRIB-18 — `verificado_em` é um campo do catálogo, não da UI.** Data ISO (`YYYY-MM-DD`) da última conferência bem-sucedida. Entra no `ItemBase` como opcional, **não é renderizado** em nenhuma lista, e é o que o script de revalidação atualiza. O leitor não precisa saber quando o link foi conferido; o Curador precisa.
- **D-TRIB-19 — Link quebrado não apaga o item; marca.** O script não edita `placeholder`, não remove linhas. Ele produz um relatório e, para cada falha, anota `link_status: "<código ou erro>"` no item. A decisão de trocar o link, mover para placeholder ou remover é humana — é o Curador quem sabe se o documento mudou de endereço ou deixou de existir.
- **D-TRIB-20 — `link_status` presente bloqueia o merge.** A conferência do catálogo (`types.test.ts`) recusa qualquer item com `link_status`. Assim o campo só sobrevive no branch de revalidação, nunca em `main`: o Curador resolve cada um antes de abrir o PR.
- **D-TRIB-21 — Cadência por GitHub Actions, não por lembrete.** Workflow `tributum-links.yml` com `schedule: cron "0 9 1 */3 *"` (1º de jan/abr/jul/out, 09:00 UTC) e `workflow_dispatch`. Roda o script, e se houver falhas abre uma **issue** (não um PR) com o relatório — título `Tributum: N links quebrados (trimestre YYYY-QN)`. PR automático editando prosa editorial seria errado; issue é aviso.

## 3. Código

### 3.1 `types.ts`

```ts
export interface ItemBase {
  id: string;
  placeholder?: boolean;
  /** D-TRIB-18: data ISO da última conferência do link. Nunca renderizado. */
  verificado_em?: string;
  /** D-TRIB-19/20: só existe num branch de revalidação. Presença = link quebrado a resolver. */
  link_status?: string;
}
```

### 3.2 `scripts/tributum-links.mjs` (novo, Node 20+, sem dependências)

- Lê `auli-frontend/public/tributum.json`.
- Para cada item das quatro estantes, pega `link` (Artigo/Dataset/Analise) ou `homepage` (Instituicao). Itens sem nenhum dos dois (dataset só com `arquivo`) são ignorados.
- `fetch(url, { method: "HEAD", redirect: "follow", signal: AbortSignal.timeout(15000) })`; se o HEAD devolver 405 ou 403, repete com `GET` e descarta o corpo. Alguns portais (OJS, gov.br) recusam HEAD.
- `User-Agent: auli-tributum-linkcheck/1 (+https://github.com/oxschellen/auli)`. Concorrência 4, com 500 ms entre requisições ao mesmo host — são sites públicos, não se martela.
- Sucesso = status 200–299 após redirecionamentos. **Redirecionamento para outra URL final é sucesso, mas vai no relatório** como "moved → <url final>": é o sinal de que o link deve ser atualizado antes de morrer.
- Resultado: imprime tabela (id, estante, status, url final) e grava `tributum-links-report.md` no diretório de trabalho. Com `--write`, atualiza `verificado_em` nos itens OK e acrescenta `link_status` nos que falharam, preservando a formatação do JSON (2 espaços, chaves na ordem original — serializar com `JSON.stringify(obj, null, 2)` é suficiente se a leitura preservar a ordem, e preserva).
- Exit code 1 se houver falha; 0 caso contrário. "Moved" não é falha.

### 3.3 `.github/workflows/tributum-links.yml`

- Gatilhos: `schedule` trimestral (§2, D-TRIB-21) + `workflow_dispatch`.
- Passos: checkout, `node scripts/tributum-links.mjs` (sem `--write`), e se exit 1: `gh issue create` com o relatório no corpo, label `tributum`. Se já existir issue aberta com o mesmo título, comenta nela em vez de criar outra.
- Não commita nada.

### 3.4 `types.test.ts` — conferência do catálogo

Adicionar ao bloco que lê o JSON real:

- **"nenhum item publicado tem link_status"** — D-TRIB-20.
- **"todo item com placeholder:false tem verificado_em em formato ISO"** — `/^\d{4}-\d{2}-\d{2}$/`. Item publicado sem data de conferência é item publicado sem conferência.
- **"verificado_em não está no futuro"** — pega erro de digitação.

### 3.5 Pequenos ajustes que o conteúdo real pede

- `_leia_me` do JSON já foi atualizado no arquivo entregue (inclui D-TRIB-17 e o nome do script).
- O README do frontend (ou onde o Tributum estiver documentado) ganha um parágrafo: como rodar o script à mão, o que fazer quando a issue trimestral aparece (abrir branch, `--write`, resolver cada `link_status`, remover o campo, PR).
- **Não** tocar em `TributumShell`, listas, `CriteriosTributum`, `RodapeCurador`: nada muda na UI.

## 4. Verificação antes do merge

1. `node scripts/tributum-links.mjs` contra o arquivo entregue deve sair com **0 falhas** — é o critério de aceitação do conteúdo. Se algum link tiver morrido entre 22/08 e a execução, ele vai para a issue de resolução, não para o `main`.
2. `pnpm test`: os três testes novos verdes com o arquivo entregue; o teste de `link_status` fica vermelho se você inserir um à mão (verificação por mutação).
3. O site renderiza as quatro estantes sem badge de placeholder, exceto no item `exemplo-renuncia-por-uf`.
4. O rodapé com o Curador continua nas quatro estantes.

## 5. Checklist de revisão

1. O `tributum.json` em `public/` é byte a byte o entregue, exceto pelo que o script `--write` legitimamente alterou no dia do merge (só `verificado_em`)?
2. `curadores` intacto?
3. O script respeita HEAD→GET fallback, UA identificado, concorrência baixa?
4. Redirecionamento conta como OK e aparece no relatório?
5. O workflow abre issue e **não** abre PR nem commita?
6. Três testes novos presentes; mutação de `link_status` testada?
7. Nenhuma menção à unidade de atendimento nem ao nome do mantenedor em nada novo (a guarda do #165 deve continuar verde).
8. Commit: `feat(tributum): o catálogo real entra (71 itens verificados) e os links passam a ser conferidos a cada trimestre (D-TRIB-17..21)`.
