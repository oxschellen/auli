# `docs/` — os quatro prefixos e o que cada um implica

O prefixo de um documento **declara o ciclo de vida dele**. Escolher o prefixo é decidir, na hora de
criar, o que vai acontecer com o arquivo depois — e é o que permite limpar a pasta sem reabrir cada
documento para descobrir se ainda vale.

| prefixo | natureza | ciclo de vida |
|---|---|---|
| **`TAREFA-`** | instrução | **efêmera: morre executada.** Some quando o que ela mandava fazer está em código |
| **`RELATORIO-`** | prestação de contas de uma execução | **arquivo.** Fica enquanto os números que traz forem os melhores que temos sobre aquilo |
| **`REGISTRO-`** | o que se aprendeu operando | **arquivo.** Fica; não tem "executado" |
| **`ESTUDO-`** | pergunta em aberto | **decisional.** Vive enquanto a decisão não foi tomada; ao ser tomada, vira código + `RELATORIO` |

Fora dos quatro: os `auli_*` (`code`, `features`, `operations`, `pendencias`) são **referência viva**
— editados, não substituídos —, e são o destino natural do que sobrevive a uma TAREFA apagada.

## A regra que a limpeza de 12/08/2026 destilou

Naquele dia saíram **16 documentos** e a pasta caiu de 32 para 18. Apagar foi barato **porque o
durável foi movido antes**, não porque era descartável. Nos dois casos em que isso não era verdade, a
mudança de conteúdo foi maior que a remoção:

- a `TAREFA-UPDATE-INCREMENTAL` guardava os **`D-INC-1..14`**, que são o registro de decisão do
  *formato do pack*. Sem eles, "por que `id = doc_path`?" vira arqueologia — foram para a §3.6.1 do
  [auli_code.md](auli_code.md);
- o `RELATORIO-PRE-ANON-TARF` trazia cifras que a própria execução **refutou**, e elas estavam
  repetidas no `RECOMENDACOES-CONSOLIDADAS`. Apagar sem tocar no consolidado o deixaria como único
  portador da leitura errada.

**A ordem, portanto: descobrir o que só existe ali → mover para a referência viva → então apagar.**
Nunca a inversa. O texto integral fica no git de qualquer forma; o que não pode acontecer é um
documento sobrevivente citar um removido, ou herdar sozinho um número já corrigido.

**Corolário para quem escreve:** um documento que **não** cabe em nenhum dos quatro prefixos
provavelmente é referência viva, e o lugar dele é dentro de um `auli_*` — não um arquivo novo na
pasta.
