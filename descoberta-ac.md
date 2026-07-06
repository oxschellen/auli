# Relatório de descoberta — Carta de Serviços SEFAZ-AC (sefaz.ac.gov.br)

**Data:** 2026-07-06 · **Escopo:** descoberta + base para a 21ª entidade. · **Órgão:** Secretaria de
Estado da Fazenda do Acre (SEFAZ-AC). · **Robots:** desconsiderado (decisão do usuário).

> **TL;DR — WordPress + Elementor, HTML server-rendered (sem wp-json).** A "Carta de Serviços"
> (`?page_id=6732`) lista **17 serviços** agrupados em categorias (Notas Fiscais / Cadastros / IPVA +
> Geral); cada serviço é um **post** (`?p=NNNNN`) com descrição rica. O scraper: parseia a Carta →
> serviços (título, categoria, post) → busca cada post → extrai o corpo (`.elementor-widget-theme-post-content`).
> **⚠️ Gotcha TLS:** o servidor manda o **intermediário ERRADO** (Sectigo RSA OV antigo) faltando o
> **R36** (emissor real do leaf) → curl/rustls/certifi rejeitam. Fix: embutir o R36 como trust anchor
> (igual MA; provado).

---

## Plataforma

- **WordPress + Elementor** (título "Carta de Serviços"; `wp-content`, classes `elementor-*`). HTML
  server-rendered. **`wp-json` = 404** (REST API desativada) → não há JSON; parseamos o HTML.
- URL da Carta: `https://sefaz.ac.gov.br/2021/?page_id=6732`.

## ⚠️ TLS — cadeia quebrada (intermediário errado)

O servidor envia 2 certs: o **leaf** (`*.sefaz.ac.gov.br`, emitido por *Sectigo … CA OV **R36***) e um
intermediário **ERRADO** (*Sectigo RSA Organization Validation Secure Server CA*, um CA antigo que NÃO
emitiu o leaf). Falta o **R36** → nem o store do sistema nem o bundle Mozilla (certifi/rustls) fecham a
cadeia (`CERTIFICATE_VERIFY_FAILED`). Fix (provado): baixar o R36 do AIA do leaf
(`http://crt.sectigo.com/SectigoPublicServerAuthenticationCAOVR36.crt`) e **embuti-lo como trust
anchor** no rustls (`RootCerts::new_with_certs(&[R36])`) — o leaf encadeia direto nele. Mesmo padrão do
MA (PEM embutido no crate). O R36 é emitido pela *Sectigo Public Server Authentication Root R46*.

## Catálogo — a Carta (`?page_id=6732`)

Seção "Lista de Serviços": cards agrupados por **categoria** (heading `Serviços …`), cada card com
título + link `?p=NNNNN` (o post do serviço) + "Acesse a descrição completa do serviço »". **17 serviços:**

| categoria | nº | exemplos |
|---|---|---|
| (Geral) | 6 | Isenção de ICMS para PCD, Consulta Tributária, Certidão Negativa de Débitos, Notas Fiscais, Gráficas, Diversos |
| Notas Fiscais e Documentos Eletrônicos | 3 | Emitir Nota Fiscal Avulsa (Floresta / Mercadorias / Conserto) |
| Cadastros | 4 | Domicílio Eletrônico, Sefaz Online, Cadastro de Contribuintes, Cadastro de Credores |
| IPVA | 4 | IPVA Isenção (Táxi/Mototáxi, PCD, PJ), IPVA Baixa de débito |

Parse (regex): na seção "Lista de Serviços", casar `href="…?p=(\d+)">TÍTULO` (ignorar "Acesse…"),
atribuir a categoria pelo heading `Serviços …` anterior; dedup por post id.

## Detalhe — o post (`?p=NNNNN`)

O corpo do serviço (descrição rica) está no container **`.elementor-widget-theme-post-content`**
(aparece **1× por post** — isola do header/footer/sidebar). Extrair com o crate `scraper` (DOM):
`select(".elementor-widget-theme-post-content").text()` → `clean`. Ex. (IPVA Isenção Táxi): *"São
isentos de IPVA, além dos veículos reconhecidos pela Lei Complementar nº 114/2002, os veículos
destinados à condução de passageiros…"*.

## Modelagem (v1)

- **titulo** = título do card na Carta; **descricao** = corpo do post (`.elementor-widget-theme-post-content`).
- **classe** = a categoria (Geral / Notas Fiscais e Documentos Eletrônicos / Cadastros / IPVA).
- **público** = único "Serviços" (sem eixo de audiência). `ocorrencias` = {Serviços × categoria}.
- **link** = `https://sefaz.ac.gov.br/2021/?p={post}`; identidade = o post. **órgão** = "SEFAZ-AC".
- Guard = piso ~15 (os 17 da Carta).

## Pontos de decisão (D-AC*)

1. **D-AC-TLS** — embutir o intermediário R36 (Sectigo) como trust anchor (rustls), pois o servidor
   manda a cadeia errada. Documentar como exceção; se o cert for reemitido, o handshake avisa.
2. **D-AC-FONTE** — HTML (WordPress/Elementor), sem wp-json. Carta (`page_id=6732`) → 17 posts de serviço.
3. **D-AC-ROBOTS** — desconsiderado (decisão do usuário); coberto pela política D-PA-ROBOTS (UA AuliBot).
4. **Fragilidade** — parse de HTML Elementor (classes estáveis: `elementor-widget-theme-post-content`,
   headings `Serviços …`, links `?p=`); guard de contagem avisa se a Carta mudar.

## Evidência

Em `scratchpad/ac/`: `page.html` (a Carta), `post.html` (um serviço), `r36.pem` (intermediário Sectigo),
`chain.txt` (a cadeia quebrada). wp-json = 404.
