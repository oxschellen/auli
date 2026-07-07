# Descoberta — SEFAZ-DF (Distrito Federal), 22ª entidade

## Fonte
- Portal institucional: `https://www.economia.df.gov.br` (Secretaria de Economia / Subsecretaria da Receita — a Fazenda do DF fica sob a SEEC).
- **Carta de Serviços** (fonte real): `https://www.receita.fazenda.df.gov.br/aplicacoes/CartaServicos/`
  - App **ColdFusion** (`.cfm`), HTML server-rendered.
  - `/` e `/index.cfm` → 403 / erro CF (não existem). Entrada útil:
    - `listaSubCategorias.cfm?codCategoriaServico=X&codTipoPessoa=Y` → **página de listagem**
    - `servico.cfm?codServico=Z&codTipoPessoa=Y&codSubCategoria=W` → **detalhe do serviço**

## Achado central — a listagem já é o catálogo inteiro
Qualquer `listaSubCategorias.cfm?...` (independente de `X`/`Y`) devolve **a mesma árvore completa**
de serviços embutida como **objeto JS** na página (~228 KB). Estrutura:

```
'TEMA DE TOPO': { 'item': [
    'Subcategoria - Nome': { 'item': [
        {'url':'/aplicacoes/CartaServicos/servico.cfm?codTipoPessoa=6&codServico=298&codSubCategoria=272',
         'desc':'Solicitar Inclusão de Imóveis'},
        ...
    ]},
    ...
]}
```

- **472 serviços distintos** (`codServico`), com **título** no `'desc'`.
- **163 subcategorias** (chaves `'...': {'item':[...]}`) → candidata natural a **classe**.
- ~25 **temas de topo** (IPTU/TLP, IPVA, ITBI, ITCD, ISENÇÃO ICMS VEÍCULO, PROGRAMA NOTA LEGAL,
  CONTRIBUINTES DE ICMS/ISS, NOTA FISCAL AVULSA, CERTIDÃO CIDADÃO/EMPRESA, ...).
- ⇒ **1 único fetch** enumera todo o catálogo. Sem paginação, sem headless.

## Detalhe do serviço (`servico.cfm`) — descrição rica
Cada detalhe (~70 KB) tem um **accordion** (`div.panel-group#accordion` → `div.panel-body`) com ~5 painéis:
**Descrição**, prazo, requisitos/documentação, canais/como acessar, legislação, arquivos p/ download.
Concatenar os `panel-body` (strip de tags/entidades via html5ever) dá descrição limpa de ~400–1.300 chars.
(Há também um `var states=[...]` de autocomplete com os 472 títulos — ignorar; é ruído.)

## Público (`codTipoPessoa`)
Frequência nos 472: **7**=287, **6**=145, **22**=26, **8**=21. Semântica observada:
- `6` = Pessoa Física / Cidadão (ex. tema "IPTU/TLP")
- `7` = Pessoa Jurídica (títulos com sufixo "- PJ"; tema "IPTU/TLP - PJ")
- `8` = pessoa jurídica/negócios (REDESIM, Junta Comercial, SEBRAE, TERRACAP, TARF)
- `22` = nichos (NOTA FISCAL AVULSA, PRODUTOR RURAL, FEIRANTE AMBULANTE, REFORMA TRIBUTÁRIA)

## TLS / rede — ⚠️ WAF por fingerprint (JA3)
- O host **reseta a conexão do `ureq`** (rustls e native-tls: `Connection reset by peer`), mas responde
  **200 ao `curl`** (OpenSSL) com o mesmo UA/URL → allowlist por **fingerprint TLS (JA3)**, exatamente
  como o GO. Solução: toda a coleta via `kit::http::get_via_curl` (subprocess curl; requer curl no PATH).
- A cadeia de certificados em si fecha (curl `ssl_verify_result=0`); o bloqueio é do ClientHello do ureq.

## Identidade / chaves
- Identidade estável do serviço = `codServico`.
- Link = `https://www.receita.fazenda.df.gov.br/aplicacoes/CartaServicos/servico.cfm?codServico={cs}&codTipoPessoa={tp}&codSubCategoria={sc}`.
- Órgão = "SEFAZ-DF" (Subsecretaria da Receita / SEEC-DF).

## Molde de referência
Novo molde "ColdFusion CartaServicos": listagem = árvore JS única (parse por regex dos tuplos
`{'url':...,'desc':...}` + chave-pai = classe), detalhe = accordion `panel-body`. Extração de texto
= mesmo `html_to_text` (strip + html5ever) de GO/ES/TO/MA/AP/AC.

## Decisões pendentes (levar ao usuário)
1. **Riqueza**: rico (472 fetches de detalhe, ~5 min, cacheado) vs listagem-só (472 títulos + classe, instantâneo).
2. **Público**: split Cidadão/Empresa (via `codTipoPessoa`) vs público único "Serviços".
3. **classe**: subcategoria imediata (parse confiável, 163) — default proposto.
