# TAREFA — `tools/anon-tarf`: rodar o `auli-anon` sobre o TARF-RS e medir

**Natureza:** programa **one-shot**, à parte do sistema do app. Ele lê o acervo do TARF-RS, produz uma
**cópia editada** (anonimizada) para comparação, e gera os dados que decidem três coisas que hoje são
suposição. **Não anonimiza o corpus de produção** e não entra em nenhum pipeline.

**Decisão de arquitetura que enquadra isto:** o `auli-anon` fica **biblioteca**, não serviço. Este
programa é o primeiro consumidor externo dela, e serve de teste da fronteira de reuso.

**Onde vive:** `tools/anon-tarf` — o bairro declarado do código que existe para uma janela e sai de
cena depois. Herda o workspace.

---

## 1 — Por que vale a pena, em ordem de valor

1. **O acórdão é um corpus auto-rotulado.** As linhas rotuladas do `## corpo` (`RECORRENTE:`,
   `RECORRIDO(S):`, `PROCESSO Nº:`) **dizem** qual é a entidade. Extrair o valor e procurá-lo no texto
   livre dá um *gold set* de milhares de razões sociais e nomes reais **sem uma linha de anotação
   manual**. É evidência de outra ordem que as fixtures sintéticas.
2. **Mede a lacuna 5 pela primeira vez.** "Razão social sem sufixo **e** sem termo de segmento" é hoje
   uma lacuna *assumida e não medida*. Com os `RECORRENTE` extraídos, sai a distribuição real: quantos
   têm sufixo societário, quantos têm termo de segmento, quantos têm nenhum dos dois.
3. **Testa a conclusão da §6.1 do `auli-anon_pendencias`.** Ela afirma que "para o acervo de hoje, não
   há o que anonimizar" — mas foi medida **só nos pareceres**, que vêm saneados da fonte (RS usa o
   literal `XYZ`; SC, PR e SP usam o papel). O TARF **nomeia as partes**. Se a medição confirmar, o
   TARF é a exceção àquela conclusão e a seção precisa ser reescrita com números.
4. **Fixtures commitáveis.** Acórdão é documento público: ao contrário do log de auditoria, o material
   colhido aqui pode ir para o repo e ser auditado por terceiros.
5. **Dicionário minerado dos dados** em vez de curado à mão (§5).

---

## 2 — Linha de base: **Fases 1–4**, e uma divergência a reportar

O que existe em `main` são as **Fases 1–4**. As Fases 5 (dicionário de prenomes) e 6 (razão social por
termo de segmento) são **patches reprovados** em `docs/`, não código — a Fase 5 foi medida com **397
falsos positivos** e a 6 caiu por empilhar sobre ela.

**Divergência de documentação a reportar, não a corrigir sozinho:** o `ESTUDO-anon-ner-v2` §1 lista as
Fases 5 e 6 como cobertura vigente do determinístico; o `auli-anon_pendencias` diz que são patches
reprovados; e o código concorda com o pendencias (não há `nome_dicionario` nem `razao_segmento` em
`reconhecedores/`). Confirme qual está certo e **reporte** — num repo que trata documentação como
contrato, essa divergência decide o que "baseline" significa em qualquer medição futura.

**Não aplique os patches.** Meça o que está em `main`.

**Não rode NER.** O NER é estudo com gates próprios; este programa mede o determinístico.

---

## 3 — Os quatro produtos

**3.1 — A cópia editada.** Uma árvore paralela, um arquivo por acórdão, mesma estrutura, com o
`## corpo` anonimizado. Sai em diretório **novo** — nunca dentro de `data/rs/docs/tarf/`. Serve para o
Carlos ler lado a lado com o original.

**3.2 — O relatório agregado.** Por reconhecedor e por classe: quantas detecções, em quantos
documentos, distribuição de confiança. Mais os números da §1: recall sobre as entidades rotuladas, e a
distribuição de forma das razões sociais (com sufixo / com termo de segmento / com nenhum).

**3.3 — Os candidatos a dicionário.** Dois, e o segundo talvez valha mais:

- **termos de segmento** minerados dos valores de `RECORRENTE` — tokens recorrentes não-próprios
  (`transportes`, `distribuidora`, `comércio`…), que hoje são ~120 curados à mão na Fase 6 reprovada;
- **stop-list do domínio jurídico** — o que aparece capitalizado e multi-token mas **não** deve ser
  mascarado: `SEFAZ`, `TARF`, `STJ`, nomes de câmara, de comarca, `Lei Estadual`. Como o gate é **FP
  zero**, e os controles de FP hoje são sintéticos, uma lista tirada de 22 mil documentos reais é o que
  protege aquele gate de verdade. Ela transfere parcialmente para as perguntas, porque analista cita
  dispositivo e órgão ao perguntar.

**3.4 — A amostra para revisão humana.** Spans com contexto, **estratificados por reconhecedor**,
limitados a algo como 200–300 no total. Ver §4.

---

## 4 — As duas metades da medição têm métodos diferentes

**Recall é automático.** Extraia o valor da entidade das linhas rotuladas, rode o anonimizador,
verifique se aquele valor foi mascarado no texto. Milhares de casos, zero revisão.

**Falso positivo não é automático** — não existe rótulo negativo. Precisa de olho humano sobre as
detecções, e é por isso que a amostra da §3.4 é estratificada e limitada. **Não** produza um arquivo
com 22 mil detecções para alguém revisar: produza a amostra e os agregados.

**Espere falsos positivos nativos e separe-os desde o relatório.** A varredura anterior do acervo pegou
`URL` 3.447×, `IP_ADDRESS` 58× e `CREDIT_CARD` 1×, disparados por numeração legal e endereço de
portal. No corpo de acórdão isso vem em volume. São classe própria — não misture com o que interessa,
ou o relatório fica ilegível.

---

## 5 — A cerca metodológica: **não minere e meça no mesmo texto**

Se os termos de segmento saírem dos `RECORRENTE` e o recall for medido nos `RECORRENTE`, você ajustou
ao conjunto de teste e o número não vale nada.

**Divida o corpus em duas metades, deterministicamente** (por hash do `doc_path`, não por ordem
alfabética — a ordem alfabética correlaciona com ano e com layout). Minere numa, meça na outra, e diga
no relatório qual metade produziu cada número. É a mesma disciplina do `C-cal × C-dec` que o estudo de
NER já adota, aplicada a um corpus novo.

---

## 6 — Um gate que sai de graça: round-trip em escala

O crate promete `anonimizar → restaurar == original`. Rodar isso sobre 22.476 documentos reais é o
teste mais severo que essa propriedade já recebeu — as fixtures são dezenas.

Verifique o round-trip **em todos** os documentos e reporte qualquer falha com o `doc_path`. Uma falha
aqui é defeito no crate encontrado em dado real, e vale mais que todo o resto desta TAREFA.

---

## 7 — Prático

- **Só o `## corpo`.** É lá que as partes aparecem; ementa e resumo raramente as nomeiam. Cortar o
  escopo também reduz o ruído do relatório.
- **Um documento por vez** — ler, anonimizar, escrever, soltar. São ~22 mil acórdãos inteiros; a
  doutrina de memória constante do resto do pipeline vale aqui.
- **Duas eras de layout no TARF.** O cabeçalho antigo é solto e o moderno vem empacotado; a extração
  das linhas rotuladas precisa aguentar as duas, ou o gold set fica enviesado para uma época. Se a
  extração falhar em parte do acervo, **reporte a taxa** — gold set parcial é utilizável, gold set
  silenciosamente enviesado não.
- **Saída sob `data/`** (que é gitignored) por padrão. O material é público e *pode* ser commitado, mas
  a decisão do que entra no repo é do Carlos, não do programa.
- Se o programa passar de ~300 linhas, provavelmente está reimplementando algo do `auli-anon` ou do
  `auli-contract`. Pare e reveja.

## 8 — O que NÃO fazer

- Não escrever em `data/rs/docs/tarf/` nem em pack nenhum.
- Não aplicar os patches das Fases 5 e 6.
- Não rodar NER.
- Não alterar o `auli-anon` para acomodar este programa. Se algo não couber pela API pública, **isso é
  o achado** — reporte, não contorne. Era essa a pergunta da fronteira de reuso.
- Não mexer no `dedup_por_slug`, que segue fora por decisão.

## 9 — Reportar

(a) o resultado do round-trip em escala; (b) recall por tipo de entidade sobre o gold set rotulado, com
a metade de medição identificada; (c) a distribuição de forma das razões sociais — o tamanho da lacuna
5; (d) se o TARF confirma ser exceção à §6.1, com contagem de documentos que nomeiam parte; (e) os dois
dicionários candidatos; (f) a divergência de documentação da §2; (g) qualquer suposição minha que não
bateu — inclusive sobre a estrutura das linhas rotuladas, que estou inferindo de dois acórdãos.
