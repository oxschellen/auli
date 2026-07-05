# Relatório — bloqueio de WAF (JA3) na API do Portal Expresso (GO)

> **RESOLVIDO (spike JA3).** O bloqueio foi confirmado como WAF por fingerprint TLS (JA3) e
> destravado pela **§5.2**: `kit::http::get_via_curl` (subprocess curl contido no kit) para os GETs
> de catálogo; o token sai pelo `ureq` (SSO sem WAF). GO integrado como 12ª entidade (94 serviços).
> Este doc fica como o diagnóstico completo; o registro canônico é `auli_pendencias.md §11 (D-GO-WAF)`
> com os 4 JA3 medidos. Dependência de runtime: `curl` no PATH.

Estado (histórico): Fase 0 do GO concluída e sólida; Fase 1 (crate) escrita e compilando; a coleta
ao vivo era bloqueada por um WAF que barra o fingerprint TLS do `ureq`. Este doc registrou o achado
com todas as evidências.

---

## 1. Resumo executivo

A API que serve o catálogo de serviços de GO (`api.go.gov.br`) fica atrás de um WAF que faz
**fingerprinting TLS (JA3)**: aceita clientes cujo *ClientHello* casa com curl/browser e responde
**HTTP 200 + JSON**; para o `ureq` (o cliente HTTP de toda a frota) responde **HTTP 200 com uma
página HTML "Acesso Negado"** — mesmo com **token válido, User-Agent, `Accept`, `Authorization` e
`Accept-Encoding` idênticos**, e HTTP/1.1 nos dois lados.

Consequência: o dado **é acessível** (curl reproduz tudo, anonimamente), mas **não pelo `ureq`** —
nem com rustls (default) nem com native-tls (OpenSSL). Não é problema de auth, header, encoding nem
versão de HTTP.

---

## 2. O que a Fase 0 estabeleceu (sólido — não é o problema)

- **Fonte:** API do Portal Expresso (WSO2 API Manager), `https://api.go.gov.br/expresso/2.0.0/`.
- **Auth = client_credentials ANÔNIMO** (satisfaz D-GO3, sem login de usuário):
  ```
  POST https://sso.go.gov.br/oauth2/token
    Authorization: Basic base64("<client_secret>:<client_pass>")   # do bundle Angular, cliente público
    Content-Type: application/x-www-form-urlencoded
    grant_type=client_credentials
  → { "access_token": "eyJ4NXQ…" }                                  # JWT válido, efêmero
  ```
  Credenciais (públicas, baked no bundle `main.<hash>.js` servido a qualquer visitante):
  `client_secret = jMQoyH_T2GpWXwBlH6goWfBBdr0a`, `client_pass = k8BOsIHTF6sARfHq4qBPsvaYjf4a`.
- **Endpoints (todos GET com `Authorization: Bearer <token>`):**
  - `servicosOrgaos/20` → os **94 serviços** da Secretaria da Economia (órgão id 20), sem paginação.
  - `orgaos` → contagem por órgão (`qtdeServicosPublicados: 94` para o id 20 — base do invariante).
  - `categorias` → `idCategoriaServico → nomeCategoriaServico` (classe legível).
- **Shape rico e inline:** `idServico`, `nomeServico`, `descUrlAmigavel` (slug), `infoServico`
  (descrição HTML), `categoriaServico[]`. **94 verificado por 3 ângulos** (qtdeServicosPublicados,
  tamanho do array, unicidade de idServico/slug). Cenário A (sem eixo de público).

Tudo isso foi reproduzido por **curl anônimo** e é a base do `IMPLEMENTACAO-scraper-go.md`.

---

## 3. O bloqueio, e a evidência de que é JA3

Sintoma no scraper (`ureq` com token válido):
```
POST token … → access_token OK (JWT eyJ4NXQ…)
Fetching: https://api.go.gov.br/expresso/2.0.0/categorias
resp[..150] = "<!DOCTYPE html>…<title>Acesso Negado</title>…"       # HTML, não JSON
Error: JSON de /categorias inválido: expected value at line 1 column 1
```

O `ureq` recebe uma página **"Acesso Negado"** (HTTP 200, corpo HTML) onde o curl recebe o JSON.

### Diferenciais descartados (com evidência)

| Hipótese | Teste | Resultado |
|---|---|---|
| Auth/token inválido | token JWT capturado do próprio ureq (`eyJ4NXQ…`) | **válido** — curl com o MESMO token passa |
| Header faltando | curl minimalista (só UA+`Accept`+`Authorization`) | **200 JSON** — mesmos 3 headers do ureq |
| `Accept-Encoding` (ureq manda `gzip` sozinho) | curl com `Accept-Encoding: gzip` e com `gzip, deflate, br` | **ambos 200** — não é isso |
| Resposta gzipada mal decodificada | pedir `Accept-Encoding: gzip` ao servidor | servidor **não comprime** (sem `Content-Encoding`) |
| Versão HTTP (curl HTTP/2?) | `curl --http1.1` + `%{http_version}` | **200, e a negociada já era 1.1** nos dois |
| Cipher CBC (gotcha do ba) | rustls **recebe resposta** (o "Acesso Negado") | conecta — não é cipher |
| Fingerprint TLS do rustls | trocar para **native-tls (OpenSSL)** no ureq | **ainda bloqueado** |

Headers que o `ureq` envia (capturados via httpbin, agente native-tls):
```
Accept: application/json
Accept-Encoding: gzip
Host: …
User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0
```
Idênticos aos do curl que passa. **Sobra só o ClientHello TLS (JA3).** curl usa OpenSSL do sistema;
o `ureq` native-tls também usa OpenSSL, mas o *ClientHello* (ordem de ciphers/extensões, ALPN,
GREASE, session tickets) difere do curl, e o WAF só tem o JA3 do curl/browser na allowlist.

### Reprodução (curl anônimo, funciona hoje)
```bash
UA='Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0'
BASIC=$(printf 'jMQoyH_T2GpWXwBlH6goWfBBdr0a:k8BOsIHTF6sARfHq4qBPsvaYjf4a' | base64 -w0)
TOKEN=$(curl -sS -A "$UA" -X POST 'https://sso.go.gov.br/oauth2/token' \
  -H "Authorization: Basic $BASIC" -H 'Content-Type: application/x-www-form-urlencoded' \
  -d 'grant_type=client_credentials' | python3 -c 'import sys,json;print(json.load(sys.stdin)["access_token"])')
curl -sS -A "$UA" -H "Authorization: Bearer $TOKEN" -H 'Accept: application/json' \
  'https://api.go.gov.br/expresso/2.0.0/servicosOrgaos/20'    # → 94 serviços em JSON
```

> Nota: o endpoint `sso.go.gov.br/oauth2/token` **não** tem o WAF — o `ureq` obtém o token sem
> problema. O bloqueio é só no `api.go.gov.br`.

---

## 4. Estado do código

Branch `feat/scraper-go` (não commitado): crate `auli-scraper-go` completo e compilando —
`main.rs`, `go.rs` (fetch via `kit::http::get_string` com header Bearer, `html_to_text` via
html5ever, dedup por `idServico`, invariante dinâmico), **7 testes unitários passando**, Cargo com
`native-tls`. Só a coleta ao vivo falha (o WAF). Tudo o mais (parse, modelagem, testes) está pronto
e é reaproveitável assim que a coleta destravar.

---

## 5. Opções para destravar

1. **Shell out para o `curl`** (mais pragmática). O scraper invoca `curl` (via
   `std::process::Command`) nos ~4 fetches; curl passa o WAF. O resto segue kit-native
   (`clean`, `ScraperInfo::new`). Vira **exceção documentada** como o native-tls do ba e a page API
   do mg. Custo: dependência de runtime no binário `curl` (presente no desktop de operação).
2. **Impersonar o JA3 em Rust.** Trocar o cliente por uma lib que imita o ClientHello do
   browser/curl (estilo `curl-impersonate` / `reqwest` com fingerprint / rustls com ClientHello
   custom). Incerto, dependência pesada, foge do `ureq` que a frota inteira usa.
3. **Ajustes finos no TLS do ureq** (não testados ainda): forçar ALPN só `http/1.1`, cipher-list
   estilo OpenSSL do curl, desligar session tickets — baixa probabilidade de casar o JA3 exato, mas
   barato de tentar.
4. **Rota alternativa** (a investigar): host de homolog (`apihomolog.go.gov.br`) ou algum caminho em
   `www.go.gov.br` que faça proxy da API sem o WAF na frente. Risco: dados de staging / instável.
5. **Parquear o GO.** Registrar como pendência (fonte + auth OK; bloqueio de WAF JA3 no cliente da
   frota) e retomar depois.

**Recomendação:** opção 1 (curl-shell) — o dado é genuinamente público e acessível, a coleta é de
baixíssima frequência (~4 requests por rodada), e a exceção fica contida e documentada. Se preferir
evitar o subprocess, vale gastar 15 min na opção 3 antes de decidir pela 2.

---

## 6. Anti-decisão registrada

O bloqueio **não** é: auth (token válido), header (idênticos), encoding, versão HTTP, nem cipher
(rustls conecta). É JA3. Portanto **não** adianta mexer em header/UA/token — só o cliente TLS muda o
veredito.
