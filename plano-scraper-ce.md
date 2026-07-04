# Plano — Scraper de serviços da SEFAZ-CE (`auli-scraper-ce`)

- **Fonte:** https://portalservicos.sefaz.ce.gov.br/ (entrada dada: a listagem
  `servico-geral+648af76264778b7336c470a3?params={"filters":{"_tags":"66cf4902…"},"sorters":…}`)
- **Repo base:** `oxschellen/auli` @ `e2a5ad7` — frota com **9 entidades** (rs, sc, pr, sp, mg,
  pe, ba, rj + …); o CE será a 10ª. Doutrina vigente: *discovery-first; API JSON > HTML
  server-side; navegador nunca* (selada no D-RS-OBSCURA).

---

## 1. O que a descoberta remota já estabeleceu

1. **SPA pura.** O HTML servido é um shell ("Esta aplicação requer que o JavaScript esteja
   ativado") — não há conteúdo server-rendered. Diferente de RJ/PR, aqui **não existe** o caminho
   "ureq + parse HTML": ou achamos a API, ou nada (navegador está fora de questão).
2. **Assinatura do backend.** Os ids são ObjectId do MongoDB; o `params` da URL carrega
   `filters._tags` + `sorters.sort["name._current.keyword"]` — ou seja, **busca Elasticsearch**
   com campos versionados por idioma (`._current`), numa plataforma de portal multi-tenant
   (o path `/sefazCeara/home` expõe o tenant). O front quase certamente repassa esse `params`
   a um endpoint JSON de busca — que é o que queremos chamar direto.
3. **Gramática de URLs mapeada** (via páginas indexadas):
   - listagem: `/servico-geral+<id-da-página>` com `_tags` = **tema** (a categoria);
   - detalhe de serviço: `/servico-geral+<slug>+<ObjectId>` (ex.: `…+servico-de-atendimento-ao-cidadao-sac+64adca7b…`);
   - temas: `/tema-geral+<slugs-encadeados>+<ObjectId>` (ex.: `…+canais-de-atendimento+fale-com-a-sefaz+…`).
4. **Eixo de público provável.** O anúncio oficial do portal fala em "cidadão, empresa ou
   transportador" — pode existir faceta de público nos documentos (cenário SP) além dos temas.
5. **Descrições provavelmente disponíveis.** As páginas de detalhe são documentos do CMS — a API
   do detalhe deve devolver o corpo (campo `description._current` ou similar). Se confirmar, o CE
   entra com descrição rica (como MG/SP), melhor que RJ/v1.

**O limite daqui:** sem navegador não dá para observar o XHR que a SPA dispara, e a regra de
fetch não permite chutar caminhos de API. O endpoint exato, o shape do payload/resposta e a
paginação são a **Fase 0**, no desktop.

## 2. Fase 0 — Discovery executável (gate; ~1 sessão no desktop)

Objetivo: capturar o contrato da API. Roteiro:

1. Abrir a URL da listagem com DevTools → Network → XHR/Fetch. Recarregar. Identificar a(s)
   requisição(ões) que trazem a lista (procurar por `search`, `query`, `api`, `content`,
   `find` — e pelo próprio ObjectId `648af762…` no corpo).
2. Registrar para a **listagem**: método+URL, headers necessários (algum token de tenant?
   `sefazCeara` vai no path, num header ou no corpo?), corpo enviado (o `params` inteiro?),
   shape da resposta (onde ficam `name`, slug, id, tags, público?), e **paginação** (ES típico:
   `from/size` ou `search_after`; testar com `size` grande — se o total for pequeno, uma chamada
   pode bastar).
3. Repetir para: (a) a **lista de temas** (de onde a home tira os `_tags`? — deve haver uma
   chamada que enumera os temas com id+nome); (b) o **detalhe** de 2–3 serviços (o corpo/descrição
   vem? em que campo? HTML ou texto?); (c) confirmar se há **faceta de público** nos documentos.
4. Reproduzir TUDO com `curl` puro (sem cookies de sessão) e salvar as amostras em
   `~/Desktop/poc-ce/` (uma resposta JSON de cada endpoint + o comando curl exato). **Gate:** se
   a API exigir sessão/anti-bot que `curl` não reproduz de forma estável e respeitosa, PARAR e
   registrar (D-CE-DISCOVERY reprovado) — não haverá scraper CE por ora.

## 3. Modelagem provisória (a confirmar pela Fase 0)

- **Coleção:** `servicos`; snapshot v3 `data/ce/ce-servicos-snapshot.json`.
- **Cenário A (só temas):** público único "Serviços" (padrão RJ), `classe` = tema; um serviço em
  N temas = N ocorrências.
- **Cenário B (faceta de público existe):** públicos = facetas (padrão SP: cidadão/empresa/
  transportador…), `classe` = tema.
- **Identidade:** o ObjectId do documento é a chave técnica perfeita para dedupe interno, mas o
  contrato usa `link` — usar a URL canônica de detalhe `https://…/servico-geral+<slug>+<id>`
  (estável e única por construção). `titulo` = `name._current`.
- **`descricao`:** corpo do detalhe se a Fase 0 confirmar (limpo de HTML, padrão MG); senão vazia.
- **`orgao` = "SEFAZ-CE"** salvo campo melhor na API.
- **Cache:** JSON por URL lógica (kit), 1 arquivo por página de listagem/tema/detalhe; cortesia
  entre detalhes (padrão MG); `--usecache` padrão.
- **Guards:** mínimos definidos após a Fase 0 medir o catálogo real (nº de temas e serviços);
  princípio D-RJ5 (falhar alto, cache só depois dos guards da listagem).

## 4. Decisões provisórias (confirmar/registrar no PR)

- **D-CE1:** fonte = API do portalservicos (não raspar o shell nem usar navegador).
- **D-CE2:** identidade = URL canônica de detalhe (slug+ObjectId); ObjectId como dedupe interno.
- **D-CE3:** eixo público/classe conforme cenário A ou B da Fase 0.
- **D-CE4:** descrição do detalhe incluída se disponível anonimamente via curl.

## 5. Fases

- **Fase 0** — discovery no desktop (gate, §2). Saída: `poc-ce/` com curls + amostras + veredito.
- **Fase 1** — TAREFA de implementação (só após o gate): crate `auli-scraper-ce` no padrão
  MG (fetch JSON paginado + detalhe) ou SP (uma busca só), conforme o que a API der; integração
  = checklist padrão (registry, prompt, grep da última entidade integrada).
- **Fase 2** — primeira coleta + derivados + amostra manual (padrão RJ).

## 6. Riscos

| Risco | Mitigação |
|---|---|
| API exige token/sessão/anti-bot | Gate da Fase 0 reprova; sem plano B com navegador (doutrina) |
| Paginação `search_after`/scroll complexa | Testar `size` grande primeiro; catálogo de SEFAZ costuma caber em 1–3 páginas |
| Campos `._current` mudarem com i18n | Fixar `_current` no parse e registrar no doc do crate |
| Plataforma multi-tenant muda contrato sem aviso | Guards de contagem + cache permite reproduzir o snapshot antigo |
