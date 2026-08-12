# TAREFA — teto de comprimento do `text_to_embed`

**Motivo declarado, e a ordem importa:** **densidade do vetor**. A key deve ser longa o bastante para
conter o texto que responde à busca, e curta o bastante para que esse texto seja bem representado por
1024 floats. A queda de memória é consequência, não justificativa — e é grande: pelo
`RELATORIO-MEMORIA-UPDATE.md`, o custo é **quadrático** no maior texto da coleção, então cortar paga
ao quadrado.

**Precedente que já existe no repo:** os serviços fazem exatamente isto desde sempre, com o
`snippet(300)` — e custam 9 MiB. Esta TAREFA generaliza o que uma coleção já pratica.

**O que a medição estabeleceu, e que sustenta o desenho:**

- pico ≈ base + `k × máx²`, com `k` = 1,46–1,96 ×10⁻⁵ MiB/char², medido em 2 entidades e 3 coleções;
- **um** documento fixa o pico da coleção inteira (remover o maior das FAQs derrubou 10.333 → 6.758 MiB);
- logo o `FATIA` não é o termo dominante e não adianta mexer nele;
- pico de regeneração hoje: **17,7 GiB** (TARF), e o maior acórdão já satura o teto de 8.192 tokens.

---

## Fase 0 — as duas medições que escolhem o número (portão: **pare e reporte**)

O teto não vai ser escolhido por palpite. Duas medições, nesta ordem, **sem alterar código de
produção**:

**0.a — A cauda está sendo representada?** Para os documentos que um teto de ~6.000 caracteres
cortaria (~4% do TARF, ~2% das FAQs, 0% dos pareceres RS), calcule o **cosseno entre o vetor do
documento inteiro e o vetor dos seus primeiros N caracteres**, para N em {4.000, 6.000, 8.000}.

- **≥0,98** ⇒ a cauda praticamente não move o vetor: ela já não estava sendo representada, e o corte é
  ganho puro. É o resultado que *justifica* o teto em vez de apenas tolerá-lo.
- **≤0,90** ⇒ a cauda muda o vetor de verdade; truncar altera o que é encontrável e a decisão passa a
  ter custo real — reporte antes de seguir.

Referência para calibrar a leitura: o `ESTUDO-busca-hibrida` registrou 0,55–0,58 como a assinatura de
"nada próximo".

**0.b — O teste do "conluio".** Nos 69 acórdãos em que o termo aparece, **em que posição do
`text_to_embed`** está a ocorrência? Quantas cairiam além de cada N candidato? É `grep` com offset,
minutos de trabalho. O `ESTUDO-busca-hibrida` já mostrou que termo raro solto é o ponto fraco da busca
densa; se essas ocorrências vivem na cauda, o teto piora o caso que já era o pior.

**Reporte as duas e pare.** A escolha do N é minha, com esses números na mão.

**O dial, para você saber o que cada N custa:** `máx_chars ≈ √(orçamento_MiB / 1,5×10⁻⁵)`. Um teto de
6.000 chars ⇒ termo quadrático de ~540 MiB (contra 16.700 hoje). **Ressalva honesta:** o coeficiente
foi ajustado no regime da cauda e **não vale extrapolado para baixo** — `rs-pareceres` tem máximo de
1.807 chars e Δ de 288 MiB, onde a lei prevê ~49. Há um termo não quadrático que domina embaixo.
Portanto prometa **entre 540 MiB e ~1 GiB** acima da base, não 540 exatos.

---

## Fase 1 — o corte

**Um parâmetro só, no ponto único onde a key é composta:** `compose_unificado`, em `auli-contract`.
Todas as quatro coleções passam por lá (os adaptadores por coleção chamam esse compose), então o teto
é uma const no contrato e um `.chars().take(N)` no fim — a mesma mecânica do `snippet`, que já é o
precedente vivo.

**Corte pelo FIM, sem lógica por coleção.** A ordem do compose é trilha → título → ementa → resumo:
cortar o fim preserva a parte densa (título e ementa, que já são resumo por natureza) e descarta a
cauda da fundamentação, onde mora a repetição — citação legal transcrita, relatório, fecho.

**Duas exigências de correção, ambas com precedente no `snippet`:**

- corte em **`chars()`, nunca em bytes** — cortar UTF-8 no meio de um caractere quebraria a key (e o
  `snippet` já faz assim);
- determinismo byte a byte preservado: mesma entrada, mesma key, sempre. É pré-condição do pack
  reproduzível.

**Nada de tokenizer aqui.** Cortar em tokens seria exato, mas o tokenizer vive em `auli-core/embed` e
o `text_to_embed` nasce em `auli-contract` — inverteria a direção de dependência entre as camadas. O
corte em caracteres é aproximado (3 a 4 chars/token em texto jurídico) e é o suficiente: o objetivo é
densidade, não um teto exato de bytes.

**Os testes de fórmula congelada vão falhar, e isso é esperado.** Existem testes que provam que o
`compose_unificado` reproduz as fórmulas antigas byte a byte, e o `mod formulas_antigas` / `mod
blocos_antigos` guarda as cópias à mão. Eles falharão para entradas acima do teto. **Não os apague:**
o certo é que passem a afirmar a equivalência **até o teto** e o corte acima dele — a doutrina de que
a fórmula é congelada continua valendo, com um limite novo e explícito.

---

## Fase 2 — o `avisar_truncados` muda de sentido

Hoje ele avisa quem bate nos 8.192 tokens do modelo — evento **raro**, e por isso o aviso significa
"aconteceu algo excepcional". Com o teto novo ele dispararia em ~4% do TARF a cada rodada e viraria
ruído que ninguém lê.

Passa a ser **estatística da rodada**, não alarme: quantos documentos foram cortados, e talvez o maior
comprimento visto — uma linha por coleção. O aviso de exceção pelo teto do **modelo** fica impossível
por construção (nenhuma key chega perto de 8.192 tokens), então essa checagem sai junto: um alarme que
não pode disparar é código morto (CLAUDE.md §3).

---

## Fase 3 — a re-vetorização

O teto muda vetores ⇒ **`STRATEGY_VERSION` sobe** ⇒ o cache é invalidado inteiro ⇒ re-embed das 48.408
keys. **É a guarda da Fase 5 da TAREFA-UPDATE-INCREMENTAL sendo exercida pela primeira vez de verdade**
— e é bom que seja num caso em que o reembed é mesmo obrigatório.

Não rode as 27 entidades por conta própria. Faça **uma** entidade de verificação e reporte:

- o pico medido bate com a previsão do dial para o N escolhido?
- quantos documentos foram cortados, por coleção?
- o pack novo difere do antigo **exatamente** nos documentos cortados (os demais registros com vetor
  byte a byte idêntico)? Este é o teste que prova que o corte fez só o que devia.

A execução das 27 é minha, como da última vez.

---

## O que NÃO fazer

- **Não mexa no `EMBED_MAX_TOKENS = 8192`.** Ele é capacidade do modelo, não política nossa, e baixá-lo
  sozinho não economizaria nada: a alocação segue o comprimento real do texto, não o `max_length` — o
  `rs-servicos`, com ~130 tokens, custou 79 MiB acima da base, não 8 GiB.
- **Não mexa no `batch_size = 1`** nem em nada que o afete. Está registrado no código que, com lote
  automático, o mesmo texto saía com cosseno 0,978 entre lotes diferentes.
- **Não mexa no `FATIA`.** A medição mostrou que não é o termo dominante.
- **Não implemente chunking.** Se a Fase 0.a der ≤0,90, o remédio para diluição é dividir, não cortar —
  e isso é plataforma (`ESTUDO-pos-documento`), não esta TAREFA. Reporte e pare.
- Não toque no `dedup_por_slug`, que segue fora por decisão.

## Reportar

(a) as duas medições da Fase 0, e **pare aí**; (b) depois do meu N: o corte, os testes de fórmula
congelada reformulados, o `avisar_truncados` novo; (c) a entidade de verificação com o pico medido
contra o previsto; (d) qualquer suposição minha que não bateu com o código — pare e diga, não
contorne.

**Anotado como melhoria de fundo, fora do escopo:** o `## resumo` do TARF é a fundamentação inteira do
acórdão, não um resumo — é a causa dos 32.722 caracteres. Uma sinopse de verdade ali resolveria
densidade e memória juntas, ao custo de uma passada de LLM em 22.476 acórdãos, que foi justamente o
que se evitou quando o TARF foi desenhado sem LLM.
