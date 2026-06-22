# Fase 0 — Baseline e decisões canônicas

Artefato da Fase 0 do [roteiro_integracao_data.md](roteiro_integracao_data.md). Captura o
**baseline funcional** (o "antes") e materializa as **decisões de inventário e fonte canônica** que
a Fase 1 exige. Gerado em 2026-06-22, na branch `feat/data-root-integration`.

> **Status:** baseline capturado ✅. As decisões abaixo precisam de **sign-off** antes da Fase 1.

---

## 1. Baseline de boot (server atual, `rs`)

```
🏛️  Entidades carregadas: [rs]
🔎 Manifesto de 'rs' validado contra a identidade local.
📦 rs-services — 627 registros
📦 rs-faqs — 1734 registros
📦 rs-pareceres — 331 registros
📦 rs-notas — 1 registros
✅ Server started successfully at 0.0.0.0:3000
```

Contagens-alvo a reproduzir após a Fase 1: **services 627, faqs 1734, pareceres 331, notas 1.**

As 5 respostas de referência ("antes") estão no Apêndice A. Após a Fase 1, re-rodar as mesmas 5
perguntas e comparar — devem ser **equivalentes** (mesmos serviços/links citados).

---

## 2. Inventário canônico de entidades

Conjunto canônico reconciliado das 4 fontes (server `./entities`, `auli-cli` ENTITIES,
`auli-collections` domain + `src/entities`, frontend `entities.ts`):

| id | name | uf | state | coleções (frontend) | no server hoje? | scraper hoje? |
| --- | --- | --- | --- | --- | --- | --- |
| `rs` | SEFAZ-RS | RS | Rio Grande do Sul | servicos, faqs, pareceres, notas, conteudos | **sim** | faqs, servicos |
| `sc` | SEF-SC | SC | Santa Catarina | servicos | **não** (sem `entities/sc/`) | servicos |

**Kinds (vetoriais, `auli-core::corpus::ALL`):** `services`, `faqs`, `pareceres`, `notas`.
**Divergências de nomenclatura a travar no `registry.toml`:**

- `servicos` (frontend/scraper) ↔ `services` (backend, nome da coleção vetorial `rs-services`).
  **Decisão recomendada:** manter `services` como o kind vetorial (não re-vetorizar à toa); o
  `registry.toml` mapeia o rótulo de UI `servicos` → kind `services`.
- `conteudos` é **só do frontend** (aba Conteúdos), **sem kind vetorial** e **sem scraper** —
  é conteúdo autorado (`conteudo_site_tree.json`). Entra no registry como coleção *de referência*,
  não como kind de RAG.

---

## 3. Mapa de fonte por arquivo (`rs`) — md5 nas 3 cópias

Prefixos de md5; `—` = ausente. (Conferido em 2026-06-22.)

| arquivo | server/entities | collections/data | frontend/public | situação | **canônico (recom.)** | destino |
| --- | --- | --- | --- | --- | --- | --- |
| `portal-servicos.txt` | `d781…` | `8dc0…` | `8dc0…` | **server diverge** | ⚠️ **decisão #1** | fonte de packs |
| `portal-faqs.txt` | `19b3…` | `538c…` | — | **server diverge** | ⚠️ **decisão #1** | fonte de packs |
| `portal-pareceres.txt` | `b55c…` | `b55c…` | `b55c…` | idênticos | qualquer | `ref/` (autorado) |
| `portal-notas.txt` | `1cb4…` | `1cb4…` | `1cb4…` | idênticos | qualquer | `ref/` (autorado) |
| `faqs.json` | — | `e1a5…` | `e1a5…` | coll = public | scraper | `raw/` |
| `servicos.json` | — | `6ccc…` | `6ccc…` | coll = public | scraper | `raw/` |
| `servicos-index.json` | — | — | `851a…` | **só public** | public (regen. scraper) | `raw/` |
| `conteudo_site_tree.json` | — | — | `800f…` | **só public** | public | `ref/` (autorado) |
| `prompt.txt` (rs) | `e951…` (14 ln) | `082e…` (13 ln) | n/a | **divergem** | ⚠️ **decisão #2** | `data/prompts/rs.txt` |

**Achados que viram decisão:**

- Os **`portal-servicos.txt`/`portal-faqs.txt` que GERARAM os packs atuais** (em
  `auli-server/entities/rs/`) **diferem** dos que o scraper produziu (`auli-collections/data/rs/` =
  `public/rs/`). Ou seja, o baseline atual **não** foi construído da última raspagem.
- O `prompt.txt` do server (14 linhas) é o que está **em produção** (gera as respostas do
  baseline); o de collections (13 linhas) é mais antigo.

---

## 4. Decisões para sign-off

**Decisão #1 — fonte canônica de `portal-servicos.txt` / `portal-faqs.txt`** (alimentam os packs):

- **(1a) Preservar o baseline** → adotar as versões do **server** (`auli-server/entities/rs/`) como
  canônicas, mover para `data/rs/ref-source/` (ou `raw/`, ver #3), re-vetorizar e confirmar que as
  contagens (627/1734) e as 5 respostas batem. **Recomendado** — Fase 1 fica "refactor puro", sem
  mudança de conteúdo.
- **(1b) Adotar a raspagem nova** → usar as versões de `auli-collections/data/rs/` (mais recentes),
  re-vetorizar e **re-capturar o baseline** (as respostas podem mudar). Faz sentido se a raspagem
  nova for de fato melhor — mas vira mudança de conteúdo, não só de plumbing.

**Decisão #2 — `prompt.txt` de `rs`:** recomendado **server (14 linhas)** como canônico (é o que está
em produção), movido para `data/prompts/rs.txt`. O de collections (13 ln) é descartado.

**Decisão #3 — `update --source` após o split `ref/` + `raw/`** (os `portal-*.txt` ficam em pastas
diferentes): recomendado **(3a)** um passo no script que monta um dir "source" agregando `ref/` +
`raw/` por symlink/cópia (zero código no binário); alternativa **(3b)** ensinar o `update` a resolver
a origem por kind (mais código). 

**Decisão #4 — destino de `pareceres`/`notas`/`conteudos`:** `data/<id>/ref/` **versionado** (são
autorados, sem scraper). Confirmado pela ausência deles na saída do scraper.

> `sc`: hoje só tem `servicos` (scraper) e **não** é entidade do server. Entra no server na Fase 2
> (registry). Em `sc`, `auli-collections/data/sc/` já tem `servicos-index.json` (ao contrário de
> `rs`), então a migração de `sc` é mais limpa.

---

## Apêndice A — Respostas de referência (baseline "antes")

5 perguntas contra `rs`, server atual. Texto integral salvo; usar para comparar pós-Fase 1.
(Resumo dos serviços/links citados — o texto completo está em `/tmp/auli-baseline-qa.txt` e abaixo.)

1. **Certidão negativa de débitos** → serviço "Obter Certidão PENDENTE ou POSITIVA de TERCEIROS";
   links `servico=1698` (cidadão e empresa).
2. **Parcelar débitos de ICMS** → e-CAC → Parcelamentos; Anexo L-34; `servico=1701`.
3. **Emitir NF-e** → "Emitir Nota Fiscal Eletrônica para Contribuinte"; `servico=1715`.
4. **Documentos para inscrição estadual** → "Inscrição – Regra Geral (CGC/TE)"; `servico=1661`;
   IN DRP 045/98 Seção 6.0.
5. **Consultar situação cadastral** → "Consulta Pública ao Cadastro de Empresas (CGC/TE)";
   `servico=1259`.

> Critério de equivalência pós-Fase 1: cada resposta deve citar o **mesmo serviço principal e o
> mesmo `servico=NNNN`**. Variação de redação do LLM é aceitável; troca de serviço/links não é.

---

## Apêndice B — Respostas integrais (baseline)

```text
### P: Como obtenho certidão negativa de débitos?
Para solicitar a **Certidão Negativa de Débitos** siga o serviço “**Obter Certidão PENDENTE ou POSITIVA de TERCEIROS**”:

* **Cidadãos (pessoa física)** – abra o protocolo eletrônico no Portal Pessoa Física ou acesse diretamente:  
  https://www.fazenda.rs.gov.br/servicos-ao-cidadao/servicos?servico=1698  

* **Empresas (pessoa jurídica)** – faça a solicitação no Portal e‑CAC ou via protocolo eletrônico:  
  https://www.fazenda.rs.gov.br/servicos-a-empresas/servicos?servico=1698  

A certidão será emitida como **Negativa** quando não houver débitos, pendências de GIA ou arquivos da Escrituração Fiscal Digital (EFD). Caso a certidão não seja gerada imediatamente, anote o número de autenticação e consulte novamente após 10 minutos.

**Perguntas Frequentes relacionadas**  
* Como é feita a emissão? – https://atendimento.receita.rs.gov.br/certidao-de-situacao-fiscal  
* Certidão Negativa – https://atendimento.receita.rs.gov.br/certidao-de-situacao-fiscal (seção “Certidão Negativa”).  

Se precisar de apoio adicional, abra um protocolo eletrônico no portal correspondente.

---
### P: Como faço para parcelar débitos de ICMS?
Para parcelar débitos de ICMS siga os passos abaixo:

1. **Acesse o e‑CAC** (portal da Receita Estadual) com login, senha ou certificado digital.  
    Link: https://www.sefaz.rs.gov.br/Receita/PortaleCAC.aspx  

2. No menu rápido escolha **“Parcelamentos”** → **“Parcelamento de Crédito Tributário”**.  

3. Informe o CNPJ (ou CPF) e selecione o débito que deseja parcelar (ICMS, multas, IPVA, etc.).  

4. Defina o número de parcelas (máximo ≤ 120, conforme a modalidade) e a data de pagamento da parcela inicial (até o último dia útil do mês corrente).  

5. Clique em **“Simular Parcelamento”** para visualizar o valor de cada parcela.  

6. Se concordar, confirme o pedido (formulário Anexo L‑34) e gere a Guia de Arrecadação da parcela inicial.  

7. Opcional: autorize débito automático em conta corrente (Banrisul) – há link específico para a autorização.  
    Link: https://atendimento.receita.rs.gov.br/parcelamentos-autorizar-debito-em-conta-corrente  

**FAQ e informações complementares**  
- Guia geral de parcelamento: https://atendimento.receita.rs.gov.br/parcelamentos-debitos  
- Como fazer o pedido de parcelamento pela internet: https://atendimento.receita.rs.gov.br/pessoa-juridica/servicos?servico=1701  
- Prazos, condições e dívidas elegíveis: https://atendimento.receita.rs.gov.br/parcelamentos-debitos  

Se precisar de ajuda adicional, entre em contato pelo canal de atendimento da Receita Estadual.

---
### P: Como emitir uma nota fiscal eletrônica?
**Serviço indicado**

- **Emitir Nota Fiscal Eletrônica para Contribuinte** – permite a emissão de NF‑e por empresa com inscrição ativa no Estado do Rio Grande do Sul (CGC/TE).  
  **Link:** https://www.fazenda.rs.gov.br/servicos-a-empresas/servicos?servico=1715  

**Passos resumidos**

1. Acesse o portal e‑CAC: https://atendimento.receita.rs.gov.br/pessoa-juridica/servicos?servico=1715  
2. Selecione **“Emitir Nota Fiscal Eletrônica”**.  
3. Informe os dados do emitente e do destinatário (CNPJ/CPF, endereço, inscrição estadual, etc.).  
4. Preencha os itens da nota (código NCM, CFOP, quantidade, valor, alíquotas).  
5. Valide a nota; corrija eventuais mensagens de erro.  
6. Transmita a NF‑e; aguarde a autorização da SEFAZ.  
7. Após a autorização, imprima o DANFE e baixe o arquivo XML (menu **Extras**).

**Perguntas Frequentes (FAQ) úteis**

- Como funciona a emissão de NF‑e: https://atendimento.receita.rs.gov.br/nota-fiscal  
- Requisitos de certificado digital: https://atendimento.receita.rs.gov.br/certificacao-digital  
- Cancelamento ou complementação de NF‑e: https://atendimento.receita.rs.gov.br/modelo-operacional  

Caso precise de suporte operacional, ligue para a Central NF‑e 0800 978 2338 ou acesse o Plantão Fiscal Virtual: https://www.sefaz.rs.gov.br/Site/FaleConosco.aspx.

---
### P: Quais os documentos para inscrição estadual?
**Serviço indicado**  
- **Inscrição – Regra Geral – Pedir inscrição de Contribuinte de ICMS (CGC/TE)** – a solicitação pode ser feita pelo Portal REDESIM RS ou pelo e‑CAC da Receita Estadual.  
  👉 [https://www.fazenda.rs.gov.br/servicos-a-empresas/servicos?servico=1661](https://www.fazenda.rs.gov.br/servicos-a-empresas/servicos?servico=1661)

**Documentação necessária**  

- No momento da entrega da solicitação **não é exigida a apresentação de documentos**.  
- Após o envio, a Receita Estadual pode solicitar documentos previstos na **IN DRP 045/98 – Seção 6.0** e na **Lei 8.820/89 (arts. 38 a 41)**, como:  
  - Contrato/social da empresa ou contrato de prestação de espaço (ex.: coworking);  
  - Comprovante de endereço do estabelecimento;  
  - Dados do contabilista ou empresa de contabilidade (CRC, CNPJ);  
  - Documentos de identidade e CPF dos responsáveis legais.  

**Perguntas Frequentes (FAQ) relacionadas**  

- Dúvidas Frequentes sobre Cadastro de Contribuintes – <https://atendimento.receita.rs.gov.br/cadastro-de-contribuintes>  
- Consultas Formais Frequentes sobre Inscrições Estaduais – <https://atendimento.receita.rs.gov.br/consultas-formais-frequentes>  

Caso precise de orientações específicas (ex.: inscrição para coworking, associações civis ou EC 87/15), consulte os links de serviço correspondentes nas FAQs acima.

---
### P: Como consultar a situação cadastral de uma empresa?
Para consultar a situação cadastral de uma empresa (inscrição estadual, situação ativa, baixa, suspensão etc.) utilize a **Consulta Pública ao Cadastro de Empresas da Receita Estadual do RS – CGC/TE**.

**Como acessar:**  
- Acesse o portal de serviços da Receita Estadual e clique em “Consulta Pública ao Cadastro de Empresas”.  
- Informe o CNPJ ou a Inscrição Estadual da empresa que deseja consultar.  

**Link direto:**  
https://www.fazenda.rs.gov.br/servicos-a-empresas/servicos?servico=1259  

**Outras opções de consulta:**  
- Consulta completa pelo e‑CAC: https://www.sefaz.rs.gov.br/sat/ICS-CON-COM.aspx  
- Consulta por NIRE, Razão Social, Sócio, etc., disponíveis nas páginas indicadas na mesma seção de serviço.

Essas ferramentas permitem visualizar a situação atual da empresa, histórico de baixas, suspensões e demais informações cadastrais.  

**FAQ relacionada:**  
- Pergunta “Onde posso consultar o cadastro de uma empresa?” – mesma página de serviço acima.  

Caso precise de mais detalhes ou tenha dúvidas específicas, acesse a seção de Perguntas Frequentes no portal: https://atendimento.receita.rs.gov.br/perguntas-frequentes.

---
```
