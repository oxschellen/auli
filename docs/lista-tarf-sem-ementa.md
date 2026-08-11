# Acórdãos do TARF que ficaram fora da árvore

**Data:** 09/08/2026 · **Fonte:** varredura completa dos 22.590 acórdãos do portal.

Dos 83 que o scraper descartou por não achar a linha `EMENTA`, **54 foram recuperados** pelos
quatro ajustes do extrator (PR #137). Este arquivo lista os **29 que sobram** — os que nenhum
ajuste de parser alcança, porque o defeito não está do nosso lado. Nenhum deles existe em
`data/rs/docs/tarf/`, e é assim que deve ser.

É a evidência da afirmação que fecha a investigação: **nenhum dos 29 é defeito de parser.** Cada
linha traz o link do portal e o começo do texto extraído, para conferência sem refazer a
varredura. Para reproduzir, o corpo limpo de qualquer um sai de
`auli-scraper-rs` sobre o cache — o `<uuid>` do link é a chave.

Os nomes das partes aparecem como o tribunal os publicou: alguns acórdãos vêm com o recorrente
redigido pela própria origem (`RECORRENTE: (...)`), outros não. Não mexemos em nenhum dos dois.

---

## A — sem ementa escrita pelo tribunal (19)

Acórdãos íntegros, com corpo completo, publicados sem ementa. São os candidatos ao passo de
**sinopse por LLM** (o mesmo que os pareceres usam).

| # | Acórdão | Corpo | Link |
|---|---|---|---|
| 1 | `ACÓRDÃO 054/20` | 1.813 c | [portal](https://tarf.sefaz.rs.gov.br/documento/383fa55c-7361-4b3e-7b85-08da5912d737) |
| 2 | `ACÓRDÃO 142/20` | 25.073 c | [portal](https://tarf.sefaz.rs.gov.br/documento/77dff884-fc83-4f5b-840b-08da59c3aecf) |
| 3 | `ACÓRDÃO 629/25` | 45.795 c | [portal](https://tarf.sefaz.rs.gov.br/documento/4ba2e2b5-8e55-42c9-dd7a-08deb7fea46a) |
| 4 | `ACÓRDÃO 630/25` | 45.795 c | [portal](https://tarf.sefaz.rs.gov.br/documento/e4c81462-5718-4419-3092-08deb7f799d7) |
| 5 | `ACÓRDÃO 692/25` | 8.318 c | [portal](https://tarf.sefaz.rs.gov.br/documento/365cb01b-22f3-463e-d987-08deb7f92f26) |
| 6 | `ACÓRDÃO 693/25` | 8.521 c | [portal](https://tarf.sefaz.rs.gov.br/documento/dacda097-6632-4ea9-db93-08deb7f88abc) |
| 7 | `ACÓRDÃO 694/25` | 8.520 c | [portal](https://tarf.sefaz.rs.gov.br/documento/5503d4ce-cf66-4fd1-ad99-08deb7f7f86a) |
| 8 | `ACÓRDÃO 695/25` | 8.734 c | [portal](https://tarf.sefaz.rs.gov.br/documento/87188bed-6e59-4975-ad90-08deb7f7f86a) |
| 9 | `ACÓRDÃO DO PLENO Nº 005/19` | 1.025 c | [portal](https://tarf.sefaz.rs.gov.br/documento/0745c953-ef8e-47af-9f4a-331630165d10) |
| 10 | `ACÓRDÃO DO PLENO Nº 054/12` | 60.834 c | [portal](https://tarf.sefaz.rs.gov.br/documento/3d708251-4ead-4f95-ad1a-229c5c89d14e) |
| 11 | `ACÓRDÃO DO PLENO Nº 074/18` | 3.865 c | [portal](https://tarf.sefaz.rs.gov.br/documento/3cd21afd-c7a2-48b6-97ab-07312741a472) |
| 12 | `ACÓRDÃO DO PLENO Nº 078/15` | 4.948 c | [portal](https://tarf.sefaz.rs.gov.br/documento/70de38fb-2b16-4d70-ae7b-f8e9b4447066) |
| 13 | `ACÓRDÃO DO PLENO Nº 079/15` | 4.944 c | [portal](https://tarf.sefaz.rs.gov.br/documento/a5058443-aa04-46e7-8810-5617ce047e8b) |
| 14 | `ACÓRDÃO DO PLENO Nº 146/16` | 5.397 c | [portal](https://tarf.sefaz.rs.gov.br/documento/f66fc576-02b3-4f30-b094-71834c164dd4) |
| 15 | `ACÓRDÃO DO PLENO Nº 212/12` | 4.760 c | [portal](https://tarf.sefaz.rs.gov.br/documento/32843bcb-3f28-4aba-b490-86e7bca265f8) |
| 16 | `ACÓRDÃO Nº 001/18` | 114.567 c | [portal](https://tarf.sefaz.rs.gov.br/documento/59e98ec4-6eaa-4b74-b352-b35c2ec24f86) |
| 17 | `ACÓRDÃO Nº 002/18` | 77.130 c | [portal](https://tarf.sefaz.rs.gov.br/documento/101dbcf2-3b1a-47f5-9668-05086e1349b4) |
| 18 | `ACÓRDÃO Nº 059/17` | 35.636 c | [portal](https://tarf.sefaz.rs.gov.br/documento/db17e54a-18b0-4d51-9e82-7625091decf4) |
| 19 | `ACÓRDÃO Nº 471/17` | 1.476 c | [portal](https://tarf.sefaz.rs.gov.br/documento/9470fbff-7317-4d71-aba4-641dde77b11b) |

### Primeiras linhas de cada um

**ACÓRDÃO 054/20**  
`ACÓRDÃO 054/20 RECURSO Nº: 432/19 RECORRENTE(S): FAZENDA ESTADUAL RECORRIDO(S): CENTRO DE COMÉRCIO E SERVIÇOS ESTETHUS LTDA. PROCESSO Nº: 18/1404-0021638-1 AUTO DE LANÇAMENTO Nº: 8228795 DECISÃO DE 1ª INSTÂNCIA Nº: 1463190017 EMENTA: A autuação, acompanhada da exclusão do Simples Nacional, decorreu …`

**ACÓRDÃO 142/20**  
`ACÓRDÃO 142/20 ACÓRDÃO Nº: 142/20 RECURSO Nº 101/19 RECORRENTE(S): STIHL FERRAMENTAS MOTORIZADAS e FAZENDA ESTADUAL RECORRIDO(S): STIHL FERRAMENTAS MOTORIZADAS e FAZENDA ESTADUAL PROCESSO Nº: 17/1404-0019365-3 AUTO DE LANÇAMENTO Nº: 0017657490 DECISÃO DE 1ª INSTÂNCIA Nº: 0922180031 ACÓRDÃO Vistos, r…`

**ACÓRDÃO 629/25**  
`CLIQUE AQUI PARA VISUALIZAR O ARQUIVO NO FORMATO ORIGINAL. ACórdão Nº: 629/25 RECURSO Nº 626/25 RECORRENTE(S): LOJAS QUERO-QUERO S.A. RECORRIDA(s): FAZENDA ESTADUAL PROCEsso nº: 25/1404-0012894-4 AUTO DE LANÇAMENTO Nº: 55094040 DECISÃO DE 1ª INSTÂNCIA Nº: 1483250285 ICMS. GLOSA DE CRÉDITOS EXTEMPORÂ…`

**ACÓRDÃO 630/25**  
`CLIQUE AQUI PARA VISUALIZAR O ARQUIVO NO FORMATO ORIGINAL. ACórdão Nº: 630/25 RECURSO Nº 628/25 RECORRENTE(S): LOJAS QUERO-QUERO S.A. RECORRIDA(s): FAZENDA ESTADUAL PROCEsso nº: 25/1404-0011925-2 AUTO DE LANÇAMENTO Nº: 55092144 DECISÃO DE 1ª INSTÂNCIA Nº: 1483250191 ICMS. GLOSA DE CRÉDITOS EXTEMPORÂ…`

**ACÓRDÃO 692/25**  
`CLIQUE AQUI PARA VISUALIZAR O ARQUIVO NO FORMATO ORIGINAL. ACórdão Nº: 692/25 RECURSO Nº 443/25 RECORRENTE(S): YARO COMÉRCIO E IMPORTAÇÃO LTDA. RECORRIDA(s): FAZENDA ESTADUAL PROCEsso nº: 24/1404-0014251-8 AUTO DE LANÇAMENTO Nº: 51561409 DECISÃO DE 1ª INSTÂNCIA Nº: 1310240204 ACÓRDÃO Nº: 136/25, DA …`

**ACÓRDÃO 693/25**  
`CLIQUE AQUI PARA VISUALIZAR O ARQUIVO NO FORMATO ORIGINAL. ACórdão Nº: 693/25 RECURSO Nº 465/25 RECORRENTE: DMTOP COMÉRCIO DE MEDICAMENTOS E COSMÉTICOS LTDA. RECORRIDA: FAZENDA ESTADUAL PROCEsso nº: 24/1404-0020148-4 AUTO DE LANÇAMENTO Nº: 53620186 DECISÃO DE 1ª INSTÂNCIA Nº: 1483240157 ACÓRDÃO Nº: …`

**ACÓRDÃO 694/25**  
`CLIQUE AQUI PARA VISUALIZAR O ARQUIVO NO FORMATO ORIGINAL. ACórdão Nº: 694/25 RECURSO Nº 469/25 RECORRENTE: DMTOP COMÉRCIO DE MEDICAMENTOS E COSMÉTICOS LTDA. RECORRIDA: FAZENDA ESTADUAL PROCEsso nº: 24/1404-0020139-5 AUTO DE LANÇAMENTO Nº: 53620097 DECISÃO DE 1ª INSTÂNCIA Nº: 1483240150 ACÓRDÃO Nº: …`

**ACÓRDÃO 695/25**  
`CLIQUE AQUI PARA VISUALIZAR O ARQUIVO NO FORMATO ORIGINAL. ACórdão Nº: 695/25 RECURSO Nº 489/25 RECORRENTE: DMTOP COMÉRCIO DE MEDICAMENTOS E COSMÉTICOS LTDA. RECORRIDA: FAZENDA ESTADUAL PROCEsso nº: 24/1404-0020146-8 AUTO DE LANÇAMENTO Nº: 53620151 DECISÃO DE 1ª INSTÂNCIA Nº: 1483240154 ACÓRDÃO Nº: …`

**ACÓRDÃO DO PLENO Nº 005/19**  
`ACÓRDÃO DO PLENO Nº 005/19 RECURSO Nº: 784/18 RECORRENTE: GERTRUDES SCHNITZLER RECORRID A : FAZENDA ESTADUAL PROCE SSO: 143-1400/8-9 AUTO DE LANÇAMENTO Nº: 23058188 DECISÃO DE 1ª INSTÂNCIA Nº: 1331170124 ACÓRDÃO Nº: 038/18 Primeira Câmara EMENTA: ACÓRDÃO Vistos relatados e discutidos estes autos, AC…`

**ACÓRDÃO DO PLENO Nº 054/12**  
`RECURSO Nº 1362/11 ACÓRDÃO DO PLENO Nº 54/12 RECORRENTE: (...) RECORRIDA: FAZENDA ESTADUAL (Proc. nº 109019-14.00/11-5) PROCEDÊNCIA: SANTA MARIA - RS DECISÃO DE 1ª INSTÂNCIA: 922110010 AUTO DE LANÇAMENTO: 22538470 ACÓRDÃO Nº 732/11, Primeira Câmara EMENTA: ICMS. CRÉDITOS FISCAIS. GLOSA. MATERIAL ALH…`

**ACÓRDÃO DO PLENO Nº 074/18**  
`ACÓRDÃO DO PLENO Nº 074/18 RECURSO Nº 475/18 RECORRENTE : Dr. OETKER BRASIL LTDA RECORRIDA: FAZENDA PÚBLICA ESTADUAL PROCESSO Nº 1044.1400/18.0 DECISÃO DE 1ª INSTÂNCIA Nº: 0922160046 AUTO DE LANÇAMENTO N º: 0034339175 ACÓRDÃO DO PLENO Nº 42/18 ICMS. PROCESSUAL. PEDIDO DE ESCLARECIMENTO INTEMPESTIVO.…`

**ACÓRDÃO DO PLENO Nº 078/15**  
`RECURSO Nº 12/15 ACÓRDÃO ACÓRDÃO PLENO DO PLENO Nº 78/15 RECORRENTE: (...) RECORRIDA: FAZENDA ESTADUAL (Proc. nº 145668-1400/14-1) PROCEDÊNCIA: PORTO ALEGRE - RS DECISÃO DE 1ª INSTÂNCIA Nº: 0821140067 AUTO DE LANÇAMENTO Nº: 29537681 e EMENTA: ICMS. PROCESSUAL. RECURSO EXTRAORDINÁRIO. PRESSU- POSTOS …`

**ACÓRDÃO DO PLENO Nº 079/15**  
`RECURSO Nº 64/15 ACÓRDÃO ACÓRDÃO PLENO DO PLENO Nº 79/15 RECORRENTE: (...) RECORRIDA: FAZENDA ESTADUAL (Proc. nº 145682-1400/14-9) PROCEDÊNCIA: CAXIAS DO SUL - RS DECISÃO DE 1ª INSTÂNCIA Nº: 821140066 AUTO DE LANÇAMENTO Nº: 29537711 e EMENTA: ICMS. PROCESSUAL. RECURSO EXTRAORDINÁRIO. PRESSU- POSTOS …`

**ACÓRDÃO DO PLENO Nº 146/16**  
`RECURSO Nº 587/16 ACÓRDÃO DO PLENO Nº 146/16 RECORRENTES: CONTERRA CONSTRUÇÕES E TERRAPLANAGENS LTDA. RECORRIDA: FAZENDA ESTADUAL (Proc. 60414-14.00/16-2) PROCEDÊNCIA: CACHOEIRINHA - RS DECISÃO DE 1ª INSTÂNCIA: 0956150182 AUTO DE LANÇAMENTO: 26028506 Acórdão Nº: 413/16, da Primeira C âmara EMENTA: P…`

**ACÓRDÃO DO PLENO Nº 212/12**  
`RECURSO Nº 1114/12 ACÓRDÃO DO PLENO Nº 212/12 RECORRENTE: (...) RECORRIDA: FAZENDA ESTADUAL (Proc. nº 4598-14.00/12-4) PROCEDÊNCIA: VACARIA - RS acórdão nº 084/12 , da Segunda Câmara, AUTO DE LANÇAMENTO: 23759488 DECISÃO DE PRIMEIRA INSTÂNCIA: 99111897 RECURSO EXTRAORDINÁRIO. PROCESSUAL AJUIZAMENTO …`

**ACÓRDÃO Nº 001/18**  
`RECURSO Nº 350/17 ACÓRDÃO Nº 001/18 RECORRENTE: JOÃO CARLOS DOS SANTOS FARIAS, HÉLIO NAZARI, CLEBER GUSTAVO QUEVEDO FARIAS E PEDRO HENRIQUE NAZARI RECORRIDA: FAZENDA ESTADUAL (Proc. nº 4473-1400/17-0) PROCEDÊNCIA: PELOTAS - RS DECISÃO DE 1ª INSTÂNCIA Nº: 0922170020 AUTO DE LANÇAMENTO Nº: 0034604260 …`

**ACÓRDÃO Nº 002/18**  
`RECURSO Nº 497/17 ACÓRDÃO Nº 002/18 RECORRENTE: TELEFÔNICA BRASIL S/A RECORRIDA: FAZENDA ESTADUAL (Proc. nº 7136-1400/17-8) PROCEDÊNCIA: PORTO ALEGRE - RS DECISÃO DE 1ª INSTÂNCIA Nº: 0922170038 AUTO DE LANÇAMENTO Nº: 0006985580 EMENTA: ICMS. AUDITORIA. PRESTAÇÃO DE SERVIÇOS DE TELECOMUNICAÇÃO. ICMS …`

**ACÓRDÃO Nº 059/17**  
`RECURSO Nº 716/16 ACÓRDÃO Nº 059/17 RECORRENTE: AVÍCOLA PENA SUL LTDA. RECORRIDA: FAZENDA ESTADUAL (Proc. nº 63.352-14.00/16-8) PROCEDÊNCIA: CANOAS - RS DECISÃO DE 1ª INSTÂNCIA Nº: 0743160033 AUTO DE LANÇAMENTO N°: 33378509 ICMS. OPERAÇÕES DE SAÍDAS DE MERCADORIAS REALIZADAS SEM A EMISSÃO DE DOCUMEN…`

**ACÓRDÃO Nº 471/17**  
`RECURSO Nº 457/17 ACÓRDÃO Nº 471/17 RECORRENTE: COARROZ COOPERATIVA AGROINDUSTRIAL ROSARIENSE RECORRIDA: FAZENDA ESTADUAL (Proc. nº 7127-1400/17-9) PROCEDÊNCIA: ROSÁRIO DO SUL - RS DECISÃO DE 1ª INSTÂNCIA Nº: 722170021 AUTO DE LANÇAMENTO Nº: 0038819970 EMENTA: ICMS - REDUÇÃO DE ICMS A PAGAR, FACE A …`

---

## B — o portal declara ACÓRDÃO INEXISTENTE (10)

O corpo devolvido pela API é a frase `ACÓRDÃO INEXISTENTE`. O descarte está correto.

| # | Acórdão | Corpo | Link |
|---|---|---|---|
| 1 | `ACÓRDÃO 126/19` | 14 c | [portal](https://tarf.sefaz.rs.gov.br/documento/2d318bec-34a1-4247-a5c6-08da5b80a2d1) |
| 2 | `ACÓRDÃO 162/21` | 14 c | [portal](https://tarf.sefaz.rs.gov.br/documento/9cd83707-bc39-4b4d-11ba-08da5b5871c3) |
| 3 | `ACÓRDÃO 400/24` | 14 c | [portal](https://tarf.sefaz.rs.gov.br/documento/a88853ef-3f40-42e3-b2d9-08de90161353) |
| 4 | `ACÓRDÃO Nº 116/16` | 17 c | [portal](https://tarf.sefaz.rs.gov.br/documento/9654c036-4a06-44a3-b074-e049c22c561f) |
| 5 | `ACÓRDÃO Nº 202/12` | 19 c | [portal](https://tarf.sefaz.rs.gov.br/documento/5215b494-c952-4439-8a8f-81b2153a1221) |
| 6 | `ACÓRDÃO Nº 775/12` | 17 c | [portal](https://tarf.sefaz.rs.gov.br/documento/1245d7d4-ef16-48e7-805d-072266d26e76) |
| 7 | `ACÓRDÃO Nº 843/10` | 19 c | [portal](https://tarf.sefaz.rs.gov.br/documento/64702a54-552a-486d-8836-dc3357aac382) |
| 8 | `ACÓRDÃO Nº 949/09` | 19 c | [portal](https://tarf.sefaz.rs.gov.br/documento/05a41cb3-ee2d-4247-867b-fc1e4699230b) |
| 9 | `ACÓRDÃO PLENO 001/24` | 20 c | [portal](https://tarf.sefaz.rs.gov.br/documento/6e39137d-67c5-4a1f-070f-08dcbaf9a37a) |
| 10 | `ACÓRDÃO PLENO 045/23` | 20 c | [portal](https://tarf.sefaz.rs.gov.br/documento/147e3404-eddd-46cc-d688-08dbbb984e1e) |
