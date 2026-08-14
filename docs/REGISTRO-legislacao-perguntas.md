# REGISTRO — Coleção Legislação: datasets de perguntas e respostas

Documenta as **fontes** dos textos legais e as **convenções** usadas na geração dos
datasets P/R da coleção `legislacao` do Auli. Companheiro do `TAREFA-LEGISLACAO.md`
(que rege a implantação no código); este arquivo rege o **conteúdo**.

Última atualização: 14/08/2026 (4ª entrega — arts. 153–154).

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

### 2.3 Próximas (planejadas, ordem do geral ao específico)

- CF arts. 145 a 162 restantes (Sistema Tributário Nacional completo);
  fronteiras a decidir: art. 195 e dispositivos tributários do ADCT.
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
  `chave_dispositivo` do índice usa para ordenar e, desde o **D-LEG-12a**, ele
  desce até a alínea: `(artigo, sufixo, parágrafo, inciso, alínea)`. Regras que
  quem escreve a ementa precisa saber, porque governam a posição do par na aba:
  1. Chaveia pelo **1º de cada nível que estiver escrito**. Plural e faixa são
     aceitos e valem pelo primeiro — "Arts. 85 a 89" → art. 85; "§§ 5º a 9º" →
     § 5º; "§ 1º, IV; § 2º, IV e V" → § 1º, IV.
  2. **Ausência de subdispositivo, ou a palavra `caput`, vale o menor valor** —
     "Art. 155", "Art. 155, caput" e "Art. 155 (escopo do acervo)" empatam no
     topo do artigo. O empate entre elas é aceito de propósito.
  3. Os níveis são separados por **vírgula, nunca por espaço**. Em
     "Art. 138, II, e" o `e` é a **alínea**; em "Art. 155, III e § 6º" o mesmo
     `e` é **conjunção**. Só a pontuação os distingue — a vírgula certa é o que
     põe o par no lugar certo.
  4. Sufixo de letra do artigo ("Art. 17-A") é o 2º componente: 17 < 17-A < 18.
  5. Ementa que não comece por artigo reconhecível **não ganha chave** e vai
     para o fim do grupo da lei, sem posição inventada.
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
  artigo do início.

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

| Lei | Pares | Situação |
|---|---|---|
| Lei 6.537/1973 (RS) | 134 | Implantados (PR #149) |
| CF art. 155 | 41 | Gerados 14/08/2026 — em revisão |
| CF arts. 150–152 | 23 | Gerados 14/08/2026 — em revisão |
| CF arts. 145–149-A | 16 | Gerados 14/08/2026 — em revisão |
| CF arts. 153–154 | 12 | Gerados 14/08/2026 — em revisão |

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
<!-- A numeração dos blocos é CONTÍNUA entre as seções 5-b/5-c/5-d de propósito: o bloco é a
     unidade de rastreio da revisão, e o §2.2-b remete a "blocos 9–12", o 2.2-c a "13–16", o
     2.2-d a "17–19". O `--fix` do MD029 renumeraria cada seção a partir de 1 e quebraria essas
     remissões. Regra desligada só daqui até o fim do arquivo. -->

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
