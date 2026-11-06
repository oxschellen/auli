# ESTUDO-RS-Faqs-Indice — o índice novo `/perguntas-frequentes` como fonte das FAQs do RS

**Data:** 2026-11-06 (1 GET do índice com o UA AuliBot, comparado com a coleta de 06/11 pela raiz `-antigo`). ·
**Estado:** **estudo — nada implementado.** A coleta segue pela caminhada da raiz `/perguntas-frequentes-antigo`
(#201). · **Origem:** [TAREFA-RS-FAQS-RAIZ.md](TAREFA-RS-FAQS-RAIZ.md) §1 e
[REGISTRO-scrapers.md#rs-faqs](REGISTRO-scrapers.md#rs-faqs), armadilha 2 ("candidato a fonte futura… estudo à
parte"). · **Pendência:** [auli_scraper_pendências.md](auli_scraper_pendências.md) §20.

> **TL;DR — o índice é uma fonte boa e completa para o que mostra, mas não mostra tudo e não é ao vivo.**
>
> Uma página só (5,05 MB, **1 GET de ~3 s**, passa com o AuliBot e sem o desafio do F5) com **2.701 entradas =
> 1.854 perguntas canônicas** (link `/<página>#panel-N`) **+ 847 cópias** (`#panel-N-K`, a mesma pergunta repetida
> noutro tema), de **233 páginas**, em 8 seções. Os `panel-N` são os mesmos do AJAX de cada página — chave exata.
>
> Contra a coleta de 06/11 (1.990 itens de 251 listas): **todo painel do índice está na coleta**, e das 1.846
> perguntas em comum, **1.845 têm pergunta e resposta idênticas** ao que a página serve. A diferença 1.990 → 1.854
> se explica inteira: **107 itens da coleta são cópias** (o mesmo painel publicado em duas URLs, que a caminhada colhe
> duas vezes) e **29 são do Piloto RTC – IBS**, que não tem seção no índice. A trilha do breadcrumb sai do índice
> igual em **219 de 232 páginas (94%)**; as outras 12 caem noutro ramo, também válido.
>
> Contra o índice: ele é **gerado, não ao vivo** (publicação mais recente 30/10/2026, contra 06/11/2026 nas páginas;
> 1 resposta defasada) e **não traz o Piloto RTC**. Recomendação (D-RSFAQ-1..6): **não trocar agora**; usar o índice
> como **guarda de completude** da caminhada (1 GET a mais por rodada) e deixá-lo pronto como **plano B** se o
> `-antigo` sair do ar. A comparação achou três defeitos da coleta atual que valem com ou sem troca (§7).

---

## 1. O que é o índice

`GET https://atendimento.receita.rs.gov.br/perguntas-frequentes` — HTTP 200, 5.050.984 bytes, ~3 s, com o UA
`AuliBot/0.1 (+https://github.com/oxschellen/auli)` pelo `curl`. Nem desafio do Bot Defense nem página de bloqueio:
o índice é página, não `_service/*` (o trecho onde o F5 de out/2026 barra o ureq). O host não tem `robots.txt`
(404) e as páginas declaram `<meta name="robots" content="index, follow">`. Não tem `data-matriz-source-uri` nem `panel panel-default`: é por isso
que a caminhada o classifica como `Geral` e colhe zero (o que levou ao #201).

Estrutura, de cima para baixo:

1. **Blocos antes das seções:** uma lista de perguntas recentes (data `dd/mm/aa` + "em `<página>`"), e **"Perguntas
   mais acessadas"** ("as páginas de perguntas mais consultadas nos últimos 12 meses"), com as perguntas dessas
   páginas inline. É aqui que aparece o link **canônico** das páginas populares — as seções, mais abaixo, trazem
   cópias delas.
2. **"Perguntas por assunto" — 8 seções** (`#ass-*`): ICMS (Pessoa Jurídica e Produtor), IPVA ou Veículos, e-CAC,
   site e PPF (inclui DTE e App Minha Empresa), ITCD (Herança ou Doação), Certidão / Protesto / Serasa, Prefeituras,
   Taxas, Nota Fiscal Gaúcha e Devolve. (O REGISTRO falava em seis acordeões; são oito — `#abx-cert` e `#abx-pref`
   também.)
3. **Grupos de tema** (`a.btn-block[href="#gN"]` + `div#gN`), aninháveis, com o total declarado no botão
   ("Autorregularização · 3 temas"); o nível mais baixo é a **página** ("Alerta de Divergência · 7 perguntas").
4. **Pergunta:** `a.faq-perg` com `href="/<página>#panel-N"` (canônica) ou `href="#panel-N-K"` (cópia K, sem a
   página); a resposta logo depois, em `div#panel-N[-K].faq-resp` — o HTML da resposta e a linha "Publicação:
   dd/mm/aaaa às HHhMMm".

O `panel-N` é o mesmo `id` do painel no AJAX (`pagedlistfilho`) da página: `/alerta-de-divergencia#panel-1695` é o
`div#panel-1695` do corpo que a caminhada lê. O próprio portal usa esses links por pergunta (o botão "Copiar link" de
cada resposta chama `copiarLink(1695)`).

## 2. Contagens — índice × coleta de 06/11

| | Índice | Coleta (caminhada `-antigo`, 06/11) |
|---|---|---|
| Entradas | 2.701 (1.854 canônicas + 847 cópias) | 1.990 itens |
| Páginas | 233 | 251 listas |
| Painéis distintos | 1.854 | 1.883 (+107 cópias) |
| Cópias consistentes | 847 de 847 com a pergunta e a resposta da canônica; nenhuma sem canônica | — |
| Contagens declaradas | 317 grupos "· N perguntas", **317 batem** com o que se lê | — |

Por painel: **índice ⊆ coleta** (os 1.854 estão lá); só na coleta, 29.

## 3. A diferença 1.990 → 1.854, explicada

Por par (página, painel) são 1.846 em comum, 144 só na coleta e 8 só no índice. Os 144 são páginas **inteiras** que
o índice não lista — nenhuma página aparece pela metade:

**115 — o mesmo painel em outra URL** (a página tem dois endereços; a caminhada colhe os dois, o índice lista o
canônico):

| Na coleta (alias) | No índice (canônica) | Painéis |
|---|---|---|
| `/integracao` | `/notaintegrada` | 26 |
| `/1-2-declaracao-de-conteudo-eletronica-dc-e` | `/faq-dc-e` | 18 |
| `/faq-nff-produtor-rural` | `/faq-produtor-rural-aplicativo-nota-fiscal-facil-nff-para-produtor` | 16 |
| `/1-6-novo-apuracao-assistida-do-simples-nacional-asn` | `/faq-asn` | 14 |
| `/1-5-novo-simples-nacional-e-reforma-tributaria` | `/simples-nacional-e-reforma-tributaria` | 9 |
| `/link-erros-na-assinatura` e `/erros-na-assinatura-digital` | `/erros-na-assinatura` | 8 + 8 |
| `/faq-ct-e-os-conhecimento-de-transporte-eletronico-para-outros-servicos` | `/ct-e-os-conhecimento-de-transporte-eletronico-para-outros-servicos` | 7 |
| `/1-4-novo-o-grupo-especializado-setorial-do-simples-nacional-ges-sim` | `/faq-o-grupo-especializado-setorial-do-simples-nacional-ges-sim` | 5 |
| `/1-3-novo-auto-de-infracao-simples-nacional` | `/faq-auto-de-infracao-do-simples-nacional` | 4 |

O caso da assinatura é o único em que a canônica não está na árvore do `-antigo`: os 8 painéis entram duas vezes,
pelas duas URLs alternativas, e `/erros-na-assinatura` — os "8 só no índice" — não é visitada. Somando: 99 cópias
cuja canônica também foi colhida + 8 da assinatura = **107 cópias**.

**29 — o Piloto RTC – IBS, ausente do índice.** As 9 páginas do menu `/faq-piloto-rtc-ibs` (o 1º item do `-antigo`):
`/participacao` 6, `/questoes-operacionais` 5, `/sobre-o-piloto-rtc-ibs` 4, `/formalizacao` 4,
`/envolvimento-e-dedicacao` 3, `/legalidade-e-penalidades` 3, `/suporte` 2, `/cronograma` 1, `/transparencia` 1. O
índice só cita o piloto como uma linha datada ("15/12/25 Piloto RTC - IBS") na lista de recentes, sem perguntas e
sem link para essas páginas.

Conta: 1.990 − 107 (cópias) − 29 (piloto) = **1.854**.

## 4. Conteúdo — igual, com uma defasagem

- **Perguntas:** 1.845 de 1.846 idênticas. A exceção é a `/recibo-gia-inconsistente#panel-3787`, cujo título tem
  um `<` cru no HTML do portal ("…Vl.Deb(Cd3-Anx15-GIA Ant.) `<` Vl.Crd(Cd1-Anx14). O que fazer?"): o índice corta
  ali, e o snapshot atual também (§7).
- **Respostas:** 1.845 de 1.846 idênticas, comparadas sem espaços e sem o texto dos botões (§7). A exceção é a
  `/duvida-sobre-pagamento#panel-652`: a página a republicou em **06/11/2026 às 09h34** (agora cita o DETRAN-RS), e
  o índice ainda tem a versão de **09/12/2021**.
- **Defasagem:** a publicação mais recente do índice é de **30/10/2026 15h43**; a das páginas, 06/11/2026 09h34 — só
  aquela resposta é posterior. Então o índice é **gerado de tempos em tempos**, não a cada edição. Com uma amostra
  só, não dá para saber a frequência.

## 5. A trilha — não se perde

O REGISTRO dava a perda da trilha do breadcrumb como o custo de trocar de fonte. Medido: para cada página, tomando a
**primeira ocorrência dentro de uma das 8 seções** (a canônica pode estar no bloco "mais acessadas", fora delas), o
caminho seção → grupos → página reproduz o breadcrumb, abaixo da seção, em **219 de 232 páginas (94%)** — tirando
"Inicial", a numeração ("1. ", "---- 7.1 ") e o nome da seção, que muda de forma ("1. ICMS (Pessoa Jurídica e
Produtor)" × "ICMS — Pessoa Jurídica e Produtor"). Em 12 a folha é a mesma e o ramo é outro: são páginas que o
índice pendura em vários temas (51 páginas aparecem em mais de um caminho), e o breadcrumb escolheu outro deles —
p.ex. `/regimes-especiais-faq`, breadcrumb "Transportes e Regimes Especiais | Regimes Especiais", primeira ocorrência
no índice sob "Documentos Fiscais / Nota Fiscal > Dúvidas Setoriais… > GES Transportes". Sobra 1 caso diferente.

## 6. O que o índice dá de graça

- **1 GET em vez de ~650** (330 páginas + 314 AJAX por rodada da caminhada — os 644 arquivos do cache), e sem passar pelo `_service/*` — o
  único trecho do portal atrás do F5.
- **Tudo ou nada:** uma página só, que vem inteira ou falha. A caminhada perde subárvores em silêncio (armadilha 3
  do REGISTRO); hoje quem segura isso é só a guarda de encolhimento do `update-rs-faqs.sh`.
- **Invariantes:** as 317 contagens declaradas ("· N perguntas") batem com as perguntas lidas; toda cópia tem
  canônica; toda cópia repete pergunta e resposta da canônica.
- **Sem duplicata:** cada painel uma vez, com a página canônica.
- **Links por pergunta** (`/<página>#panel-N`), que o portal já usa.

## 7. Achados sobre a coleta atual (valem com ou sem troca)

1. **O texto dos botões entra em toda resposta.** Cada painel tem, depois da linha "Publicação:",
   `<button onclick="copiarLink(N)">Copiar link</button>` e `<button onclick="copiar(N)">Copiar resposta</button>`;
   o `extract_panel_body_text` guarda o texto deles. **As 1.990 respostas** do snapshot de 06/11 terminam em
   "Copiar link Copiar resposta" — e as 1.947 dos backups de agosto também. Vai para os `.md`, para os vetores e
   para o contexto do chat.
2. **107 cópias** (§3): a mesma pergunta duas vezes no acervo, com links diferentes. Ruído na busca; não é erro de
   conteúdo.
3. **Título cortado no `<` cru** (1 pergunta, a 3787): o portal não escapa o `<`, e a extração para nele. O índice
   corta igual, então trocar de fonte não resolve.
4. **Link da página, não da pergunta:** todo `FaqItem` sai com `url: None` e o `FaqRaw.url` é o da página, embora o
   portal tenha `#panel-N` por pergunta (como as âncoras `#N` que o PE usa onde existem).

## 8. Opções

| | O que é | A favor | Contra |
|---|---|---|---|
| **A** | Ficar como está (caminhada `-antigo`) | verificada em 06/11; tem o Piloto RTC; conteúdo ao vivo | ~650 requisições; queda parcial silenciosa; 107 cópias; "-antigo" é nome de página em despedida |
| **B** | A + o índice como **guarda** (1 GET por rodada) | pega a queda parcial por painel; custo mínimo; nenhuma mudança de conteúdo | não muda o custo da caminhada |
| **C** | Trocar para o índice + a caminhada só do menu `/faq-piloto-rtc-ibs` | 1 GET + ~20; tudo ou nada; sem cópias; links por pergunta; trilha 94% igual | defasagem do índice (§4) não medida; troca a identidade dos `.md` das 107 cópias (somem) e, com links por pergunta, de todos (re-vetorização única) |
| **D** | Trocar para o índice sozinho | 1 GET | perde as 29 do Piloto RTC |

## 9. Decisões propostas (do mantenedor)

- **D-RSFAQ-1 — A fonte segue sendo a caminhada pela raiz `-antigo`.** Não trocar agora: o índice é gerado, com
  defasagem de frequência desconhecida, e não tem o Piloto RTC; a caminhada foi verificada em 06/11.
- **D-RSFAQ-2 — O índice vira guarda da caminhada (opção B).** A cada rodada, 1 GET do índice; antes de gravar,
  conferir **por `panel-N`** que todo painel canônico do índice está na coleta. Faltar uma **página inteira** é `Err`
  (é a queda de subárvore da armadilha 3); faltar um painel isolado é aviso (pode ser painel retirado depois da
  geração do índice). Índice que não vem, ou que não parseia, é aviso — a guarda não pode depender de um arquivo
  gerado para a coleta acontecer.
- **D-RSFAQ-3 — Tirar o texto dos botões das respostas** (§7.1): ignorar os `<button>` no `extract_panel_body_text`.
  Muda as 1.990 respostas → re-vetorização única do `rs-faqs`.
- **D-RSFAQ-4 — Deduplicar por `panel-N`** (§7.2): ficar com uma ocorrência por painel — a da página canônica quando
  o índice a conhece, senão a primeira da caminhada. −107 itens. Fica por decidir se a página-alias some da árvore
  da aba de FAQs ou fica nela vazia.
- **D-RSFAQ-5 — Links por pergunta** (§7.4): `FaqItem.url = <página>#panel-N`. Muda o nome dos `.md` (o hash é de
  link + título) → re-vetorização única; juntar com a D-RSFAQ-3 para pagar uma vez só.
- **D-RSFAQ-6 — Plano B: se o `-antigo` sumir, opção C.** Antes de precisar dela, medir a defasagem do índice — 1 GET
  semanal por 2–3 semanas, comparando a publicação mais recente do índice com a das páginas. A trilha sai da primeira
  ocorrência em seção (§5); o Piloto RTC, da caminhada do menu `/faq-piloto-rtc-ibs` (o código atual, com outra
  raiz).

As D-RSFAQ-2..5 cabem num PR só no `auli-scraper-rs`; a D-RSFAQ-6 só se o `-antigo` sair.

## 10. Reprodução

1. `curl -sS -A 'AuliBot/0.1 (+https://github.com/oxschellen/auli)' -o indice.html
   https://atendimento.receita.rs.gov.br/perguntas-frequentes` (1 GET; sem `robots.txt` no host — 404).
2. Parse do índice (`html.parser` do Python): pilha de `div`, título de cada `#gN`/`#ass-*` pelo botão que o abre,
   `a.faq-perg` (canônica `/<página>#panel-N`, cópia `#panel-N-K`), texto do `div.faq-resp` correspondente.
3. Parse da coleta: as folhas `Faq` de `data/rs/raw/rs-faqs-tree.json` e o `<url>_body.html` de cada uma no cache
   (`a#titulo-N`, `div#panel-N`), recém-preenchido por uma rodada com cache limpo.
4. Comparar por (página, `panel-N`) e por `panel-N`; textos sem espaços e sem "Copiar link"/"Copiar resposta"; trilha
   como no §5.

Os scripts da análise ficaram fora do repositório (descartáveis); os números acima são da coleta de 06/11/2026 e do
índice baixado no mesmo dia.
