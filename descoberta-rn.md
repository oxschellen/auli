# Descoberta — SEFAZ-RN (Rio Grande do Norte)

## Fonte
- Portal: `https://www.sefaz.rn.gov.br/` = **WordPress** (tema `govrn_adi`) hospedando um **SPA React**
  (create-react-app: `static/js/main.chunk.js`). Conteúdo renderizado client-side; o mesmo shell de
  8,6 KB é servido para qualquer rota (`/servicos/...` etc.).
- **WP REST API pública** (`/wp-json/`, 200). O React consome custom post types via `wp/v2`.

## O que existe de "serviço"
- CPT **`servicos`** (`/wp-json/wp/v2/servicos`, **X-WP-Total = 15**): são **cards de atalho** da home,
  não uma Carta. Cada item = `title` + `acf.categories` (taxonomia WP) + `acf.link` (destino) +
  `acf.local_exibicao` (DESTAQUE / MAIS ACESSADOS). **Sem `content`/`excerpt`** (ambos vazios).
  - Destinos: **5** apontam para `/postagem/...` (posts WP com corpo rico em `acf['Matéria']`);
    **9** para apps externos (`uvt.sefaz.rn.gov.br/#/services/...`, `sei.rn.gov.br`, `np.sefaz...`);
    **1** ("Carta de Serviços") tem `acf.link = false` (sem destino).
- CPT **`central-servico`** = 1 item ("Central de serviços SEFAZ"), `acf = []` (vazio).
- CPT **`postagem`** = posts informativos (notícias/calendários/legislação); corpo em **`acf['Matéria']`**
  (o `content.rendered` é `null`). Categoria "Finanças e Impostos" tem 143 posts. É conteúdo de
  notícia/informação, **não** um catálogo de serviços estruturado.

## UVT (o "catálogo transacional")
- `uvt.sefaz.rn.gov.br` = app **AngularJS / IIS** (Microsoft-IIS/10.0; `js/core.js`+`components.js`
  hand-built, rotas `#/services/...` com `templateUrl` hardcoded) sobre `usuarios-api.sefaz.rn.gov.br`.
- É **transacional** (login, emitir certidão, consultar contribuinte, agendamento) — **não** expõe uma
  lista/catálogo público de serviços com descrições. `usuarios-api` sem swagger; `/api/servicos` = 404.

## Conclusão
RN **não tem uma Carta de Serviços descritiva** (como DF/CE/BA/TO). O único catálogo estruturado é o
CPT `servicos` = **15 cards menu-only** (título + categoria + link, sem descrição própria). Isso é o
molde **"vazia (menu-only)"** da frota (RJ/PE) — só que via API JSON limpa (WP REST), não HTML.

## Decisões (levar ao usuário)
1. **A — Menu-only (15 cards, descrição vazia):** molde RJ/PE. Título + categoria + link via WP REST.
   Rápido, consistente, mas 15 itens e a maioria são só links externos.
2. **B — Menu-only + enriquecer os 5 que apontam para `/postagem/`** (buscar `acf['Matéria']`): 5 ricos,
   10 só link. Aproveita o conteúdo disponível, mas fica inconsistente (descrição só em 1/3).
3. **C — Não integrar RN agora:** não há Carta descritiva; o catálogo real (UVT) é transacional e fora
   do escopo. Reavaliar se/quando o RN publicar uma Carta.
