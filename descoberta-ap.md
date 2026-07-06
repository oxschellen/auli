# Relatório de descoberta — Portal SEFAZ-AP (www.sefaz.ap.gov.br)

**Data:** 2026-07-06 · **Escopo:** descoberta + base para a 20ª entidade (descrição rica). · **Órgão:**
Secretaria da Fazenda do Amapá (SEFAZ-AP).

> **TL;DR — SPA Angular (FUSE) cujos serviços com descrição rica estão HARDCODED no bundle JS** (não em
> API). A página `#/categorias/{cat}/{servico}` renderiza, em runtime, a partir de arrays `mock*`
> embutidos no chunk lazy `categorias_routes`. **49 serviços** em 5 categorias, cada um com `titulo` +
> `descricao` (blob HTML autocontido: "o que é" + Quem Pode Utilizar + Setor + Tipo). O scraper baixa o
> chunk (headless-free) e parseia os `mock*`. Fonte mais frágil da frota (JS webpack), mas as **chaves
> NÃO estão minificadas** (`route`/`titulo`/`descricao`), então o parse é estável; só o **hash do chunk
> muda por build** → descoberto dinamicamente via `runtime.js`.

---

## Plataforma

- **Angular 17 + template FUSE 19**, nginx. Home = shell SPA (2,3 KB) — o conteúdo é client-rendered.
- Há um **gateway anônimo** (`api-gateway.sefaz.ap.gov.br`, microserviços `links-api`/`noticias-api`/…)
  MAS os "serviços" da home (`links-api/links/acesso-rapido`) são **só links sem descrição** (≈22).
  O catálogo RICO é outra coisa: a página **`#/categorias`**.
- **`#/categorias/{cat}/{servico}`** (ex.: `/categorias/cadastro/mei`) mostra descrição, "Quem Pode
  Utilizar", "Setor Responsável", "Tipo de Atendimento". **Nenhuma API dispara** ao abrir — os dados são
  **hardcoded no chunk Angular** (arrays `mock*`), renderizados por `<app-page-servicos [dados]>`.
- **Por que não pegar do HTML renderizado:** o HTML servido é o shell vazio; o `<p>` com o conteúdo só
  existe DEPOIS do Angular rodar. Pegá-lo exigiria headless por página (~50 renders). O MESMO HTML já
  está no campo `descricao` do mock → parseamos o chunk (headless-free).

## Descoberta do chunk (hash dinâmico)

1. `GET https://www.sefaz.ap.gov.br/` → o shell referencia `runtime.<hash>.js` (num `<script src>`).
2. `GET /runtime.<hash>.js` → contém o mapa nome→hash:
   `"src_app_modules_landing_page_categorias_categorias_routes_ts":"<hash>"`.
3. `GET /src_app_modules_landing_page_categorias_categorias_routes_ts.<hash>.js` → o chunk com os `mock*`.

O NOME do chunk é estável; só o hash muda por deploy → sempre re-descobrir via runtime.

## Estrutura dos dados (`mock*`)

5 arrays, um por categoria (nome do array → slug de rota → nome exibido):

| array | slug (URL) | categoria | nº serviços |
|---|---|---|---|
| `mockCadastro` | `cadastro` | Cadastro | 10 |
| `mockIcms` | `icms` | ICMS | 15 |
| `mockItcd` | `itcmd` | ITCMD | 2 |
| `mockRegimeEspecial` | `regime-especial` | Regime Especial | 5 |
| `mockVeiculo` | `veiculos` | Veículos | 17 |

**Total: 49 serviços.** Cada item:
```js
{ route: 'mei',                          // (mockCadastro usa chaves JS; os outros usam "route" JSON)
  introducao: [{
    titulo: 'Pedido de Inscrição Estadual … MEI',
    descricao: `Este serviço permite … (NUIEF).<br><br><b>Quem Pode Utilizar:</b> …<br><br><b>Setor
                Responsável:</b> …`     // template literal (backtick), HTML autocontido
  }],
  documentos: [...], requisitos: [...], legislacao: [...], canais: [...]   // ignorados na v1
}
```

**Parser:** por categoria (fatia entre `const mock<X> =` e o próximo), casar por serviço
`route → introducao.titulo → introducao.descricao`:
- chaves com/sem aspas: `["']?route["']?\s*:` etc.;
- `descricao` é template literal — capturar entre backticks (0 backticks escapados no dado; alguns
  `${}` aparecem literais, cosmético). O HTML da `descricao` → `html_to_text` (html5ever, lição GO/TO).

## Modelagem (v1)

- **titulo** = `introducao.titulo`; **descricao** = `html_to_text(introducao.descricao)` (rica,
  mediana ~260 chars, com Quem Pode/Setor/Tipo embutidos).
- **classe** = a categoria (Cadastro/ICMS/ITCMD/Regime Especial/Veículos) — enriquece o `text_to_embed`.
- **público** = único "Serviços" (não há eixo de audiência). `ocorrencias` = {Serviços × categoria}.
- **link** = `https://www.sefaz.ap.gov.br/#/categorias/{slug}/{route}`; identidade = o link.
- **órgão** = "SEFAZ-AP". Guard = ≥ ~45 (piso; os `mock*` são estáticos).

## Pontos de decisão (D-AP*)

1. **D-AP-FONTE** — o catálogo rico é **hardcoded no chunk JS** (`mock*`), não em API. Parse do chunk
   (headless-free). Chaves estáveis; hash do chunk descoberto via `runtime.js` a cada rodada.
2. **D-AP-ESCOPO** — v1 = as 5 categorias de `#/categorias` (49 serviços, com descrição). Os links da
   `acesso-rapido` (≈22, sem descrição) ficam de fora (redundantes / pobres).
3. **D-AP-CAMPOS** — v1 usa só `introducao` (titulo+descricao, que já traz Quem Pode/Setor/Tipo). Os
   arrays `documentos`/`requisitos`/`legislacao` ficam para uma eventual v2.
4. **Fragilidade** — se a estrutura do `mock*` mudar (webpack renomear chaves, virar API), o parse
   quebra; o guard de contagem avisa.

## Evidência

Em `scratchpad/ap/`: `home.html`, `main.*.js`, `runtime.js` (mapa de chunks), `cat.js` (o chunk
`categorias_routes` com os `mock*`), `cap*.mjs` (capturas headless que provaram o zero-XHR do detalhe).
