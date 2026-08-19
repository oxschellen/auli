# REGISTRO — Coleção Legislação: datasets de perguntas e respostas

Documenta as **fontes** dos textos legais e as **convenções** usadas na geração dos
datasets P/R da coleção `legislacao` do Auli. Companheiro do `TAREFA-LEGISLACAO.md`
(que rege a implantação no código); este arquivo rege o **conteúdo**.

Última atualização: 19/08/2026 — Fase B do `TAREFA-LEGISLACAO-LACUNAS`: o §4
deixou de duplicar contagens, a LC 123 ganhou o §2.4 e o §3.2 acompanhou a chave
estendida na #151. Conteúdo: 6ª entrega da CF (arts. 157–162), parte tributária
COMPLETA, ressalvadas as fronteiras 195/ADCT.

---

## 1. Escopo temporal — regra pré-reforma

O acervo é especialista no regime **anterior à reforma tributária**. Regra geral,
decidida em 13/08/2026:

> **Redação vigente = última redação anterior à EC 132/2023.**

Consequências práticas:

- Alterações da EC 132 (IBS, CBS, transição, ITCMD progressivo, IPVA sobre
  embarcações/aeronaves etc.) **não entram** nos pares.
- Cada lei (ou artigo-piloto) recebe **1 par sentinela** explicitando o corte
  temporal ao usuário do chat.
- Fica documentada em cada dataset qual foi a última emenda/lei alteradora
  considerada.

## 2. Fontes por lei

### 2.1 Lei 6.537/1973 (procedimento tributário administrativo — RS)

- **Fonte:** texto compilado no site da SEFAZ-RS (`inpKey=109403`), usado como
  `link` de todos os 134 pares.
- **Texto integral de referência:** `data/rs/ref/rs-lei-6537-1973-texto-vigente.md`
  (fora de `docs/`; ver §3.7).
- **Status:** 134 pares implantados (PR #149).

### 2.2 Constituição Federal — art. 155 (piloto da parte tributária)

- **Link oficial dos pares:** compilado do Planalto —
  `https://www.planalto.gov.br/ccivil_03/constituicao/constituicao.htm`
- **Base textual da consolidação pré-EC 132:**
  1. PDF do Senado com o art. 155 integral (retrato já com EC 3/1993, EC 33/2001
     e EC 42/2003): `https://legis.senado.leg.br/sdleg-getter/documento?dm=4345148&disposition=inline`
  2. **+ EC 87/2015** aplicada por cima: nova redação do § 2º, VII e VIII
     (alíneas "a" e "b" do VII revogadas), conferida no texto da própria emenda
     (`ccivil_03/constituicao/emendas/emc/emc87.htm`).
  3. Após a EC 87/2015 não houve alteração no art. 155 até a EC 132/2023.
- **Texto consolidado de referência:** `ref/cf-art-155-redacao-pre-ec132.md`
  (neste pacote), com anotação de emenda dispositivo a dispositivo.
- **Ferramenta de conferência para os próximos artigos:** o `normas.leg.br`
  aceita consolidação **por data** na URL — ex.
  `https://normas.leg.br/?1988@2023-12-19!art145` devolve a redação vigente
  em 19/12/2023 (véspera da EC 132). Usar como segunda fonte ao consolidar
  os arts. 145–154, 156–162.
- **Status:** 41 pares gerados em 14/08/2026 (blocos 1–8), aguardando revisão
  do Carlos no site do Auli.

### 2.2-b Constituição Federal — arts. 150 a 152 (limitações ao poder de tributar)

- **Link oficial dos pares:** o mesmo compilado do Planalto.
- **Base textual da consolidação pré-EC 132:** texto original de 1988 + EC 3/1993
  (art. 150, §§ 6º e 7º) + EC 42/2003 (art. 150, III, "c", e § 1º) + EC 75/2013
  (art. 150, VI, "e"). Última alteração antes da EC 132: **EC 75/2013**.
  Arts. 151 e 152 permanecem na redação original.
- **Armadilhas pré-EC 132 identificadas** (mudanças da EC 132 que NÃO entram):
  1. art. 150, VI, "b" — vale **"templos de qualquer culto"** (a menção a
     entidades religiosas e suas organizações assistenciais é da EC 132);
  2. art. 150, § 2º — **sem** a empresa pública prestadora de serviço postal
     (inclusão da EC 132; antes, a proteção da ECT era jurisprudencial —
     RE 601.392, anotado no par correspondente).
- **Texto consolidado de referência:** `ref/cf-arts-150-152-redacao-pre-ec132.md`.
- **Status:** 23 pares gerados em 14/08/2026 (blocos 9–12).

### 2.2-c Constituição Federal — arts. 145 a 149-A (princípios gerais)

- **Link oficial dos pares:** o mesmo compilado do Planalto.
- **Base textual da consolidação pré-EC 132:** texto original de 1988 + EC 33/2001
  (art. 149, §§ 2º–4º) + EC 39/2002 (art. 149-A) + EC 42/2003 (art. 146, III, "d",
  e parágrafo único; art. 146-A; art. 149, § 2º, II) + EC 103/2019 (art. 149,
  §§ 1º e 1º-A a 1º-C). Última alteração antes da EC 132: **EC 103/2019** —
  que ENTRA no corte (reforma da previdência é anterior à reforma tributária).
- **Armadilhas pré-EC 132 identificadas**:
  1. art. 145 — os **§§ 3º e 4º** (simplicidade, transparência, justiça tributária,
     cooperação, meio ambiente; atenuação de regressividade) são da EC 132:
     não existem neste corte;
  2. art. 146 — a EC 132 deu nova redação ao III, "d", e transformou o
     **parágrafo único em § 1º**: aqui valem a redação da EC 42 e o
     "parágrafo único".
- **Texto consolidado de referência:** `ref/cf-arts-145-149a-redacao-pre-ec132.md`.
- **Status:** 16 pares gerados em 14/08/2026 (blocos 13–16).

### 2.2-d Constituição Federal — arts. 153 e 154 (impostos da União)

- **Link oficial dos pares:** o mesmo compilado do Planalto.
- **Base textual da consolidação pré-EC 132:** texto original de 1988 + EC 20/1998
  (revogação do art. 153, § 2º, II) + EC 42/2003 (art. 153, § 3º, IV, e § 4º).
  Última alteração antes da EC 132: **EC 42/2003**. Art. 154 nunca alterado.
- **Armadilhas pré-EC 132 identificadas**: o inciso VIII do art. 153 (Imposto
  Seletivo) e o § 6º são da EC 132 — não existem neste corte (nota no par
  do caput).
- **Texto consolidado de referência:** `ref/cf-arts-153-154-redacao-pre-ec132.md`.
- **Status:** 12 pares gerados em 14/08/2026 (blocos 17–19).

### 2.2-e Constituição Federal — art. 156 (impostos dos Municípios)

- **Link oficial dos pares:** o mesmo compilado do Planalto.
- **Base textual da consolidação pré-EC 132:** texto original de 1988 + EC 3/1993
  (inciso III; revogações do IV e do § 4º) + EC 29/2000 (§ 1º) + EC 37/2002
  (§ 3º, I e III). Última alteração antes da EC 132: **EC 37/2002**.
- **Armadilhas pré-EC 132 identificadas**: o inciso III do § 1º (atualização da
  base de cálculo do IPTU pelo Executivo) é da EC 132 — não existe no corte,
  em que a majoração da base acima da correção monetária exige lei (Súmula 160
  do STJ, anotada no par); os arts. 156-A/156-B (IBS) também são da EC 132.
- **Texto consolidado de referência:** `ref/cf-art-156-redacao-pre-ec132.md`.
- **Status:** 8 pares gerados em 14/08/2026 (bloco 20).

### 2.2-f Constituição Federal — arts. 157 a 162 (repartição das receitas tributárias)

- **Link oficial dos pares:** o mesmo compilado do Planalto.
- **Base textual da consolidação pré-EC 132:** texto original de 1988 + EC 3/1993 e
  EC 29/2000 (art. 160) + EC 42/2003 (158, II; 159, III e § 4º) + EC 44/2004
  (159, III — 29%) + EC 55/2007 (159, I, "d") + EC 84/2014 (159, I, "e") +
  EC 108/2020 (158, parágrafo único, I e II) + EC 112/2021 (159, I — 50% e
  alínea "f") + EC 113/2021 (160, §§ 1º e 2º). Última alteração antes da
  EC 132: **EC 113/2021**.
- **Linha do tempo do art. 159, I** (verificada — fonte de erro comum):
  47% original → 48% (EC 55, +1% dez) → 49% (EC 84, +1% jul) → 50%
  (EC 112, +1% set). Alíneas d/e/f = dezembro/julho/setembro.
- **Armadilhas pré-EC 132 identificadas**: a EC 132 incluiu o imposto do
  art. 153, VIII na base dos incisos I e II do art. 159 (no corte, a base é
  só IR+IPI — nota no par do IPI-exportação); renumerou o parágrafo único do
  art. 158 para § 1º; e criou os arts. 159-A/159-B (FNDR) — nada disso existe
  no corte.
- **Texto consolidado de referência:** `ref/cf-arts-157-162-redacao-pre-ec132.md`.
- **Status:** 12 pares gerados em 14/08/2026 (blocos 21–23).

### 2.4 LC 123/2006 (Simples Nacional) — documentada por entrega

Fonte, corte e armadilhas vivem nos `REGISTRO-lc123-entrega{1..4}.md`, não aqui: a
lei entrou depois deste documento e é gerada em blocos, um documento por entrega. O
que vale registrar no nível do acervo é o que **diverge** das convenções gerais:

- **Corte próprio**: última redação anterior à **LC 214/2025** (a LC 227/2026
  igualmente fora) — e não a EC 132/2023 do §1, que é o corte da CF. Daí a lei ter
  sentinela própria.
- **Fonte padrão a partir da 4ª entrega** (decisão de 14/08): PDF consolidado
  Câmara/LEGIN "normaatualizada". O `lcp123.htm` do Planalto trunca no art. 18,
  § 15-A para qualquer extrator — testado com três, não insistir. Os `link` dos
  pares seguem nas âncoras do Planalto: o truncamento é do fetch, não da leitura
  em navegador.
- **Ressalva vinculante da fonte nova**: o PDF da Câmara já consolida a LC 214, então
  dispositivo com "redação dada pela LC 214" mostra o texto **pós-corte**. A redação
  anterior tem de ser recuperada na página da LC que a escreveu antes
  (`Lcp147/155/188.htm`). Essa conferência é manual e é por dispositivo.
- **Blocos ainda não gerados**: arts. 18-A a 18-F (MEI), 19–20, 21–27, 28–32, 33–41
  e os Anexos I–V — estes últimos em entrega própria e **só com fetch verificado**,
  nunca de memória (decisão da 3ª entrega).

### 2.3 Próximas (planejadas, ordem do geral ao específico)

- CF: parte tributária COMPLETA (Seções I a VI, 112 pares). Pendente apenas a
  decisão de fronteiras: art. 195 (contribuições da seguridade) e dispositivos
  tributários do ADCT.
- LC 123/2006: **em andamento** — ver §2.4. Fora desta lista porque já tem
  pares no acervo; o que falta dela são blocos, não a lei.
- CTN (Lei 5.172/1966) — fonte prevista: compilado do Planalto.
- Lei Kandir (LC 87/1996) — fonte prevista: compilado do Planalto.
- Leis estaduais RS.

## 3. Convenções do dataset

### 3.1 Estrutura de pastas

- Uma **subpasta por lei** dentro de `docs/legislacao/` (diretório sem acento).
- Nome da subpasta para leis numeradas (convenção D-LEG-4):
  `Lei {número, barra→hífen} — {nome}`. Ex.: a pasta da 6.537.
- Para a CF, que não segue o padrão "Lei nº", a subpasta é **descritiva**:
  `Constituição Federal — Sistema Tributário Nacional`
  (só a parte tributária entra no acervo; se um dia entrar outra parte da CF,
  vira outra pasta). *Registrado no TAREFA como **D-LEG-4b**.*

### 3.2 Frontmatter (vocabulário unificado)

- `titulo` = **a pergunta**, em linguagem natural de balcão NAVI.
- `trilha` = hierárquica, separador ` | `; **1º segmento = nome da subpasta**
  (invariante do doc_path, guardada por teste). Para a CF, os segmentos
  seguintes são Título/Capítulo/Seção do texto constitucional.
- `ementa` = **o dispositivo** ("Art. 155, § 2º, XII, g"). É o campo que o
  `chave_dispositivo` do índice usa para ordenar, e ele desce até o item da alínea:
  `(artigo, sufixo, parágrafo, sufixo do parágrafo, inciso, alínea, item)` —
  D-LEG-12a, estendido na #151 quando a LC 123 chegou. Seis regras que quem escreve
  a ementa precisa saber, porque governam a posição do par na aba:
  1. Chaveia pelo **1º de cada nível que estiver escrito**. Plural e faixa valem
     pelo primeiro: "Arts. 85 a 89" → art. 85; "§§ 5º a 9º" → § 5º;
     "§ 1º, IV; § 2º, IV e V" → § 1º, IV.
  2. **Ausência de subdispositivo, ou a palavra `caput`, vale o menor valor** —
     "Art. 155", "Art. 155, caput" e "Art. 155 (escopo do acervo)" empatam no topo
     do artigo. O empate entre elas é aceito de propósito.
  3. Os níveis são separados por **vírgula, nunca por espaço**. Em "Art. 138, II, e"
     o `e` é a **alínea**; em "Art. 155, III e § 6º" o mesmo `e` é **conjunção**. Só
     a pontuação os distingue — a vírgula certa é o que põe o par no lugar certo.
  4. **Sufixo de letra** conta em dois níveis. No artigo ("Art. 17-A"): 17 < 17-A <
     18. No parágrafo ("§ 5º-M"): o sufixo vem **depois** do número e **antes** do
     inciso, porque no texto legal os incisos do § 1º vêm antes do § 1º-A, que é
     parágrafo próprio. O `º` é opcional antes do hífen — de 10 em diante a lei
     escreve "§ 15-A", sem ordinal.
  5. **Item dentro da alínea** ("XIII, g.1") ordena dentro da letra dele:
     a < b < g.1 < g.2. Sem o ponto e o dígito, "g.1" não é lido como alínea nenhuma.
  6. Ementa que não comece por artigo reconhecível **não ganha chave** e vai para o
     fim do grupo da lei, sem posição inventada.
- `link` = fonte oficial do texto compilado (SEFAZ para lei estadual,
  Planalto para norma federal).

### 3.3 Corpo

- Apenas `## corpo` (sem `## resumo`) — pares P/R funcionam como as FAQs.
- Resposta autocontida, citando sempre o dispositivo e, quando relevante,
  a emenda que deu a redação ("redação da EC nº 87/2015").
- **Negrito** nos elementos decisórios da resposta (quem, quanto, condição).
- Listas com alíneas/incisos quando o dispositivo é enumerativo.

### 3.4 Notas além do texto legal

Permitidas com parcimônia, **sempre rotuladas** e curtas (1–2 linhas):

- `*Nota (jurisprudência):*` — só teses consolidadas (repercussão geral,
  precedente clássico). Ex.: Tema 825 no ITCMD-exterior; RE 134.509/RE 379.572
  no IPVA de embarcações e aeronaves.
- `*Nota (infraconstitucional):*` — a norma que concretiza o dispositivo
  (Resolução do Senado 9/1992, LC 24/1975, LC 87/1996, LC 192/2022).
- Remissões cruzadas a outros dispositivos são permitidas quando ajudam a
  resposta (aprovado na revisão do bloco 1 do art. 155).

### 3.5 Par sentinela

- 1 por lei/corte temporal; `ementa` = "Art. NNN (escopo do acervo)" —
  o rótulo entre parênteses não atrapalha o `chave_dispositivo`, que lê o
  artigo do início, então a sentinela abre o grupo.
- **A LC 123 diverge, e é o único caso**: a ementa dela é "Nota de escopo — corte
  pré-LC 214/2025", que não começa por artigo. Pela regra 6 do §3.2 ela não ganha
  chave e vai para o **fim** do grupo da lei, não para o topo — o derivador avisa
  isso a cada rodada. Não foi corrigida porque o texto atual carrega o corte
  específico da lei (LC 214/2025, não a EC 132), que a fórmula acima perderia.
  Ajustar é trocar um campo; a decisão é de conteúdo, não de código.

### 3.6 Nomes de arquivo

- Nesta entrega, nomes **provisórios** com prefixo numérico de revisão
  (`01-…`, `02-…`) para leitura na ordem dos blocos.
- Nomes **canônicos** são aplicados na ingestão via `nome_faq_de`
  (pergunta ~60 chars + hash8), como na Fase 0 da 6.537 — os provisórios
  são descartáveis.

### 3.7 Texto integral de referência

- O texto consolidado da lei **não entra** em `docs/` (não é vetorizado):
  vai para `data/<id>/ref/` seguindo a família `<id>-*-texto-vigente.md`,
  que o `copy_ref` do `build-frontend-public.sh` sabe pular.
- Para consolidações reconstruídas (caso da CF pré-EC 132), o arquivo de
  referência anota **emenda a emenda** a origem de cada redação, para
  auditoria.

### 3.8 Embed

- Fórmula da coleção: `compose_legislacao_text_to_embed` — pergunta no slot
  do título + **dispositivo no slot da ementa** (é a diferença para a key
  das FAQs; sem dispositivo, a key coincide com a da FAQ — asserção em teste).

## 4. Inventário

**318 pares** na coleção `legislacao` do `rs`, em 3 leis (integrados em 19/08/2026).

Este é o **único total declarado** em todo o acervo de documentos, e a regra vem de
ter errado duas vezes: enquanto esta seção mantinha uma contagem por lei e os
`REGISTRO-lc123-*` mantinham outra, cada entrega deixava um dos dois desatualizado
— e quem lesse não tinha como saber qual. O detalhe por entrega (fonte, corte,
armadilhas, mapa dos pares) vive nos documentos apontados abaixo; aqui fica só o
agregado.

| Lei | Pasta em `docs/legislacao/` | Onde está o detalhe |
|---|---|---|
| Lei 6.537/1973 (RS) | `Lei 6.537-1973 — Processo Tributário Administrativo` | §2.1 deste documento |
| Constituição Federal (parte tributária) | `Constituição Federal — Sistema Tributário Nacional` | §§ 2.2 a 2.2-f deste documento |
| LC 123/2006 (Simples Nacional) | `Lei Complementar 123-2006 — Simples Nacional` | `REGISTRO-lc123-entrega1..4.md` |

**O total acima é derivado, não mantido à mão** — confira em um segundo, em vez de
confiar nele:

```bash
find data/rs/docs/legislacao -name '*.md' | wc -l
```

O `auli-collections rs indice legislacao` também imprime `N perguntas de legislação,
M leis` e lista as leis pelo nome — é a conferência que já roda em toda integração.

**Não há total por lei aqui de propósito.** Se precisar de um, derive:

```bash
for d in "data/rs/docs/legislacao"/*/; do printf '%4d  %s\n' "$(ls "$d"*.md | wc -l)" "$(basename "$d")"; done
```

## 5. Divisão em blocos do art. 155 (para rastreio da revisão)

1. Caput + I–III + sentinela — pares 01–05
2. § 1º (ITCMD) — pares 06–10
3. § 2º, I–VI — pares 11–18
4. § 2º, VII–VIII (DIFAL, EC 87) — pares 19–22
5. § 2º, IX–XI — pares 23–28
6. § 2º, XII (lei complementar) — pares 29–33
7. §§ 3º–5º (monofásico, EC 33) — pares 34–38
8. § 6º (IPVA) — pares 39–41

<!-- markdownlint-disable MD029 -->
<!-- A numeração dos blocos é CONTÍNUA entre as seções 5-b a 5-f de propósito: o bloco é a
     unidade de rastreio da revisão, e as seções 2.2-b a 2.2-f remetem a ele por número
     ("blocos 9–12", "13–16", "17–19", "20", "21–23"). O `--fix` do MD029 renumeraria cada
     seção a partir de 1 e quebraria essas remissões. Desligada daqui até o fim do arquivo. -->

## 5-b. Divisão em blocos dos arts. 150–152

9. Art. 150, caput, I–V e § 1º (princípios) — pares 42–50
10. Art. 150, VI e §§ 2º–4º (imunidades) — pares 51–57
11. Art. 150, §§ 5º–7º — pares 58–60
12. Arts. 151–152 — pares 61–64

## 5-c. Divisão em blocos dos arts. 145–149-A

13. Art. 145 (espécies, capacidade contributiva, taxas) — pares 65–68
14. Arts. 146, 146-A e 147 (lei complementar, concorrência, Territórios/DF) — pares 69–73
15. Arts. 148 e 149 (empréstimos compulsórios e contribuições) — pares 74–78
16. Art. 149-A (COSIP) — pares 79–80

## 5-d. Divisão em blocos dos arts. 153–154

17. Art. 153, caput e §§ 1º–2º (competências, extrafiscais, IR, IGF) — pares 81–84
18. Art. 153, §§ 3º–5º (IPI, ITR, ouro-IOF) — pares 85–90
19. Art. 154 (residual e extraordinários) — pares 91–92

## 5-e. Divisão em blocos do art. 156

20. Art. 156 (IPTU, ITBI, ISS) — pares 93–100

## 5-f. Divisão em blocos dos arts. 157–162

21. Arts. 157–158 (receitas pertencentes) — pares 101–105
22. Art. 159 (FPE/FPM, IPI-exportação, CIDE) — pares 106–108
23. Arts. 160–162 (vedação de retenção, LC, TCU, transparência) — pares 109–112
