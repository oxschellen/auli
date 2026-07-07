# Descoberta — SEFAZ-RR (Roraima), 27ª entidade

## Fonte
- Portal: `https://www.sefaz.rr.gov.br/` = site **custom/estático** ("Portal de Aplicações"; JS puro
  `/script.js`, sem CMS/framework). O nav institucional só tem Ouvidoria / Transparência / Downloads —
  **nenhuma seção "Serviços" server-rendered**, nenhuma rota `/servicos` (404).
- Os serviços são apps transacionais **GeneXus/SIATE** em `portalweb.sefaz.rr.gov.br` (CND, DARE,
  Sintegra, CIPE, GIM…) — sem página "Central de Serviços" (tudo 404) e sem catálogo descritivo próprio.

## Achado central — catálogo embutido no `script.js`
O `script.js` da home traz um **array `const apps = [{category, title, description, href}, …]`** — um
catálogo **estruturado com descrição** (molde AP, mas array limpo, chaves não-minificadas):
```js
const apps = [
  { category: "cidadao", title: "Certidão Negativa",
    description: "Emite a certidão negativa de débitos estaduais para consulta e comprovação…",
    href: "https://portalweb.sefaz.rr.gov.br/cnd/servlet/wp_siate_emitircndcentralservicopublica" },
  …
];
```
- **20 entradas** → **16 hrefs distintos** (4 serviços aparecem em `cidadao` E `empresa`: Certidão
  Negativa, Consultar Pagamento DARE, Emissão CIPE, Validação CIPE → 2 ocorrências cada).
- `category`: empresa (14) / cidadao (6). `description`: 1 linha real (~71–108 chars, "curta").
  `href`: o app real (portalweb GeneXus, ou externo: Detran-RR, IBGE CNAE).

## Modelagem (molde AP/RN, parse do array JS)
- `titulo` = `title`; `descricao` = `description` (curta); **público** = `category` (Cidadão/Empresa);
  `classe` = "Serviços" (não há eixo de tema); `link` = `href` (identidade, dedup); `ocorrencias` =
  público × classe (o serviço em 2 categorias vira 2 ocorrências).
- 16 serviços, 20 ocorrências, 2 públicos.

## FAQ (não usado)
Há um chatbot `faq-chat.php?q=` com Q&A, mas é um **matcher** (retorna a melhor resposta), não um
catálogo listável — fora de escopo por ora.

## Rede
- Apache/HTTP2, UA AuliBot aceito. `script.js` é pequeno; sem gotcha aparente.

## Decisão
Fonte única e limpa (o `apps[]`) → parse direto, sem headless. Descrições curtas (o teto da fonte; os
apps GeneXus são transacionais, sem detalhe rico). Público = categoria.
