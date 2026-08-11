# TAREFA — item 7: `snapshot.json` atômico, a consolidação do `escrever_atomico`, e uma linha de doutrina

**Responde a:** `RELATORIO-write-manifest-atomico.md` (PR #142). **Base:** o PR #142 fechado.

Três coisas, na ordem: a decisão que você me devolveu, a dívida que ela resolve, e o achado do seu §3
que merece virar doutrina escrita. Nenhuma delas toca o caminho do pack.

---

## 1 — Decisão: o `snapshot.json` passa a ser atômico

Você mostrou que os meus dois critérios apontavam para lados opostos. **O problema era o critério, não
o caso.** "Reconstruível por um comando" está subespecificado: árvore, packs e índices são todos
reconstruíveis **localmente**. O snapshot é exatamente a fronteira onde a reconstrução exige a **rede**
— e não a nossa: um portal estadual, com as pausas de cortesia que a frota respeita por doutrina.

O critério correto, e que vale registrar para as próximas decisões deste tipo:

> **Reconstruível sem custo externo.** Se refazer o artefato exige bater no servidor de outra pessoa,
> ele é atômico.

Dois fatos do seu relatório confirmam: o snapshot é escrito pelos **27 scrapers** (`write_servicos`),
logo é o artefato de hand-off de toda coleta da frota — não um caso de canto; e a falha por truncamento
é **alta** (erro de parse no header ou na desserialização tipada), o que é bom, mas "falha alta que
custa uma coleta inteira" continua pior que "não falha".

---

## 2 — Passo 1: consolidar o `escrever_atomico` (é o que torna o passo 2 uma linha)

Você está certo que meu veto não se aplica aqui. "Não crie abstração para dois usos" vale para
abstração especulativa; o que existe são **quatro cópias idênticas já escritas** (`canonizar.rs`,
`canonizar_servicos.rs`, `grafo.rs`, `grafo_servicos.rs`), e o snapshot seria a quinta.

- Um helper único em **`auli-contract`** — já tem o padrão no `mddoc` e é dependência do
  `auli-collections`. Público, com doc comment dizendo por que existe (D-INC-5 e a assimetria que o
  item 6 expôs).
- Os quatro chamadores passam a usá-lo; as quatro cópias saem.
- **Comportamento idêntico**, e isso é verificável por leitura porque as quatro são iguais entre si. Se
  alguma divergir da outra em algum detalhe, **pare e diga** — divergência entre cópias que eu suponho
  idênticas é informação, não obstáculo a contornar.
- Um teste sobre o helper: o arquivo final aparece por `rename` e o `.tmp` não sobrevive.
- O `mddoc` entra **só se couber sem contorcer**. Ele escreve o resultado do `render_doc`, e se a forma
  dele exigir torcer a assinatura do helper, deixe como está e diga por quê.

## 3 — Passo 2: o snapshot

`write_servicos` (`auli-contract/src/snapshot.rs:251`) passa a escrever pelo helper. Uma linha.

**O teste que importa é o que descreve o defeito** (§4 abaixo é sobre isso): escrever o snapshot
**através de um hardlink não altera o inode original**. O teste do `.tmp` já vive no helper e não
precisa ser repetido aqui.

---

## 4 — Passo 3: a linha de doutrina no `CLAUDE.md`

O seu §3 é maior que o item 6. Você provou por mutação que o teste do `.tmp` **passa com o código
defeituoso** — `fs::write` também não deixa `.tmp` para trás. Sozinho, ele seria decorativo.

É a **segunda vez em dois dias** que essa distinção decide se a proteção existe de fato: a primeira foi
o defeito do log regenerado pelo `remover`, que só o teste de ponta a ponta pegou. Duas ocorrências
independentes, em código diferente, é padrão — não coincidência. Está sendo praticado; falta estar
escrito.

Acrescente ao `CLAUDE.md`, na seção de testes, uma linha com esta substância (a formulação é sua):

> Para cada defeito corrigido, escreva o teste que **falha com o defeito de volta**. Um teste que
> descreve a implementação pode passar com o código defeituoso; a verificação por mutação é o que
> distingue proteção de decoração.

Uma linha, sem exemplos longos — o `CLAUDE.md` é lido por inteiro em cada sessão e cada parágrafo
compete com os outros.

---

## 5 — Auditoria, sob o critério NOVO: reportar, não corrigir

O critério mudou na §1, então a auditoria do PR #142 merece uma segunda passada — mas de novo
**delimitada**, e sem corrigir nada. A pergunta agora é: *refazer este artefato exige a rede?*

Candidatos que você listou como "derivados do pipeline de coleta" e que o critério novo pode reclassificar:

| artefato | meu palpite, para você conferir ou derrubar |
|---|---|
| `tree.json` do scraper de faqs | **candidato mais forte** — parece ser o equivalente do snapshot para as FAQs |
| `tarf-anomalias.txt` | refazer custa ~46 requisições; barato, mas é o insumo de uma decisão humana |
| cache do `auli-scraper-kit` | descartável por desenho; um truncamento pode ser pior que ausência (HTML parcial parseia) |
| `servicos/mod.rs`, `derive_faqs.rs` | derivados locais, provavelmente fora |

Reporte o que cada um é, quem o consome e o que custa refazê-lo. **A decisão é minha**, e com o helper
consolidado ela é barata: se o veredito for "todos", são cinco linhas.

---

## 6 — Cercas

- **Invariante de escopo:** `auli update` de qualquer entidade continua produzindo pack byte a byte
  idêntico, e o manifesto difere apenas no `built_at`. Nada nesta TAREFA toca o caminho do pack.
- Não mexa nos scrapers além da consolidação e do snapshot. Em particular, o desempate do
  `dedup_por_slug` continua intocado, por decisão registrada.
- Se a consolidação passar de ~40 linhas líquidas removidas/adicionadas, algo saiu do escopo — pare e
  diga.

## 7 — Reportar

(a) a consolidação, com o número de linhas que saíram e se as quatro cópias eram de fato idênticas;
(b) o teste do hardlink no snapshot, com a mutação que o derruba; (c) a linha no `CLAUDE.md`; (d) a
auditoria da §5, sem correções; (e) qualquer suposição minha que não bateu.

**Depois desta**, a TAREFA da memória do RS — e ela já tem uma correção sua incorporada: como as FAQs
são a **segunda** coleção (`servicos` roda antes), isolar cada coleção só distingue "dados" de
"acumulação" se cada uma rodar **sozinha e em primeiro lugar**.
