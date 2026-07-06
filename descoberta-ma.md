# Relatório de descoberta — Portal de Serviços SEFAZ-MA (portal-sgc.sefaz.ma.gov.br)

**Data:** 2026-07-06 · **Escopo:** descoberta + base para a 19ª entidade (descrição rica). · **Órgão:**
Secretaria de Estado da Fazenda do Maranhão (SEFAZ-MA).

> **TL;DR — Angular SPA + API REST Spring Boot (`/sgc/api`), catálogo JSON acessível com token
> ANÔNIMO.** O front loga com **credenciais públicas baked no bundle** (`{id_cliente:"41",
> senha:"<bcrypt>", portal:true}` → `POST /sgc/api/login`) e chama o catálogo com o token no header
> **`AuthorizationPortal`** (não o `Authorization` padrão). Catálogo = **`GET /sgc/api/portal/servicos`**
> (com filtros obrigatórios) → **38 serviços**; a descrição rica vem de **`GET /sgc/api/portal/conteudos/{idConteudo}`**
> (27 têm; 11 são link-only). **JSON é UTF-8** (só corpos de erro são latin1). **⚠️ Gotcha TLS:** o servidor
> manda cadeia incompleta (falta o intermediário GlobalSign) → curl/ureq rejeitam; solução = empacotar
> o intermediário como trust anchor (provado).

---

## Plataforma e auth

- **Angular SPA** (`runtime/polyfills/vendor/main.*.js`, `<base href="/">`) + **API Spring Boot** em
  `/sgc/api` (`apiUrl:"/sgc/api"` no bundle). Erros no formato Spring (`{timestamp,status,error,path}`).
- **Auth = client_credentials ANÔNIMO** (molde GO): o `environment` do bundle traz
  `clientId:"41"`, `secret:"$2a$12$…"` (bcrypt, **público** — servido a todo visitante). O front faz
  `POST /sgc/api/login` body `{id_cliente:"41", senha:"<secret>", portal:true}` → `{authtoken:"Bearer …",
  refreshtoken, …}`. **Sem login de usuário.** O token (JWT efêmero) é re-obtido a cada carga.
- **Header do token:** o interceptor Angular envia `AuthorizationPortal: Bearer <jwt>` (NÃO
  `Authorization` — este último dá 401). Descoberto no `intercept()`: `setHeaders:{AuthorizationPortal:e}`.

## ⚠️ TLS — cadeia de certificado incompleta (o gotcha)

O servidor (`*.sefaz.ma.gov.br`, GlobalSign, TLS 1.3 AES-128-GCM) manda **só a folha** — falta o
intermediário **`GlobalSign GCC R3 DV TLS CA 2020`** (a raiz GlobalSign R3 está no store do sistema).
curl/ureq/rustls rejeitam ("unable to verify the first certificate"); o browser passa via AIA-fetch.
**Solução (provada):** baixar o intermediário do AIA
(`http://secure.globalsign.com/cacert/gsgccr3dvtlsca2020.crt`) e **adicioná-lo como trust anchor**. No
Rust: `ureq::tls::RootCerts::new_with_certs(&[intermediário])` (rustls trata como anchor; a folha
encadeia direto nele). **Não** precisa de native-tls (o cipher é moderno). O PEM fica embutido no crate.

## Catálogo — `GET /sgc/api/portal/servicos`

Params OBRIGATÓRIOS (sem eles → 500): `flgPublicado=true&flgLocal=PORTAL&notOutros=false&page=0&pageSize=N`
(+ `nomeServico=&flgTipoServico=&flgDestaqueNovo=&flgPaginaPrincipal=&sortOrder=&sortField=`). Resposta
`{items:[…], total:38}`. **38 serviços.** `pageSize=1000` traz todos numa GET; **guard dinâmico = `total`**.

Campos do item (listagem magra):

| campo | → snapshot | observação |
|---|---|---|
| `id` | identidade | inteiro |
| `nomeServico` | `titulo` | — |
| `flgTipoServico` | **público** | `COMPANY`/`CITIZEN`/`PUBLIC_AGENCY`/`CERTIFICATE` |
| `idConteudo` | busca da descrição rica | 27/38 têm; 11 são link-only |
| `linkExterno` | `link` (quando houver) | destino externo (11) |
| `idServicoCategoria` | — | **0 em todos** → sem categoria (classe única "Geral") |

## Descrição rica — `GET /sgc/api/portal/conteudos/{idConteudo}`

Para os 27 com `idConteudo`: → `{titulo, descricao (HTML), introducao, …}`. A `descricao` é **HTML** →
`html_to_text` (html5ever, lição GO/ES/TO). Os 11 sem `idConteudo` (link-only) ficam só com título +
`linkExterno`.

## Modelagem (Cenário B por público, classe única)

- **titulo** = `nomeServico`; **descricao** = conteúdo (`conteudos/{idConteudo}.descricao`, HTML→texto);
  vazia p/ link-only.
- **público** = `flgTipoServico` mapeado: `COMPANY`→Empresa, `CITIZEN`→Cidadão, `PUBLIC_AGENCY`→Órgão
  Público, `CERTIFICATE`→Certidões. **1 público por serviço**. `classe` = "Geral" (não há categoria).
- **link** = `linkExterno` quando houver; senão `…/portal/conteudo/{idConteudo}`; senão `…/portal/servicos`.
- **identidade** = `id`; **órgão** = "SEFAZ-MA". `total` do catálogo = guard.

## Pontos de decisão (D-MA*)

1. **D-MA-TLS** — empacotar o intermediário GlobalSign como trust anchor (rustls). Documentar como
   exceção (o servidor deveria mandar a cadeia completa). Se o cert for reemitido por outro
   intermediário, o guard/erro de handshake avisa.
2. **D-MA-AUTH** — creds públicas baked (id_cliente=41 + secret bcrypt) → token anônimo; header
   `AuthorizationPortal`. **Não são segredo** (bundle público) — comentar no código p/ scanners (lição GO).
3. **D-MA-RICO** — buscar os 27 conteúdos (descrição rica) — decisão do usuário.
4. **D-MA-CONTAGEM** — guard = `total` da resposta (38 hoje), dinâmico.
5. **Escopo** — só SEFAZ (portal é da própria SEFAZ; `flgLocal=PORTAL`). Não multi-órgão.

## Evidência

Em `scratchpad/ma/`: `main.*.js` (bundle Angular, creds + apiUrl + interceptor), `all.json` (38
serviços), conteudo 3171, `inter.pem` (intermediário GlobalSign), `cap.mjs` (captura do XHR real).
