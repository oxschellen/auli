# TAREFA — a memória do `auli update`: medir antes de consertar

**Natureza:** esta é uma TAREFA de **medição**, não de correção. Não há defeito confirmado, não há
alvo identificado, e a única tentativa de correção até agora (o item 3 da revisão anterior) mirou a
causa errada. **O produto desta TAREFA é um relatório**, não um patch. Só depois dele se decide se
há o que consertar.

**Por que existe:** o `auli update` do `rs` pica em ~10,7 GiB (medido) e picou em 13,1 GiB na
regeneração. Passa com folga nos 60 GB desta máquina, mas é o número que decide se o Auli roda numa
máquina modesta — e replicabilidade barata é doutrina do projeto, não conforto.

---

## 1 — O que já se sabe (e o que já foi refutado)

**Medido, e confiável:**

- pico do `rs`: 10,72 GiB, idêntico antes e depois do item 3 (diferença de 2,3 MB = ruído);
- a curva de `VmRSS` a cada 200 ms mostra **10,36 GiB de pé aos 21 segundos**, durante a vetorização
  das **FAQs** (1.947 textos), e o platô **não cede** até o fim;
- todo o pipeline do TARF — três leituras de um arquivo de 311 MB, o `apply`, a serialização —
  acrescenta apenas **~620 MiB** sobre esse platô;
- o **SP**, com número parecido de documentos (16.123), fica em **3,1 GB**.

**Refutado, com argumento estrutural (não só empírico):** a desserialização do payload não é a causa.
Numa regeneração o `vetores_reaproveitaveis` retorna na primeira linha — cache desligado, arquivo
nunca aberto —, logo os 13,1 GB não podiam vir dali.

**Também caiu, por consequência:** a explicação de que o RS pica por causa do `## resumo` do TARF, com
até 27 mil caracteres. Se o platô se estabelece **antes** do TARF, essa história não se sustenta. Foi
uma atribuição minha, plausível e não medida. **Não construa sobre ela.**

**Portanto: não há causa identificada.** Nenhuma hipótese conhecida explica os três fatos juntos —
platô precoce nas FAQs, TARF acrescentando pouco, e o SP em um terço do valor com carga parecida.

---

## 2 — A medição que discrimina

Rodar cada coleção do `rs` **isoladamente**, com a mesma amostragem de `VmRSS` a cada 200 ms.

**A correção que torna a medição válida** (achado do relatório anterior): as FAQs são a **segunda**
coleção — `Kind::TODOS` é `[Servicos, Faqs, Pareceres, Tarf]`, e `servicos` roda antes. Então o platô
observado "durante as FAQs" pode ter sido estabelecido por `servicos`. Cada coleção precisa rodar
**sozinha e em primeiro lugar**; caso contrário a hipótese "acumulação ao longo da rodada" sobrevive a
qualquer resultado e a medição não decide nada.

**Registre, para cada rodada:** o pico, o instante do platô, a curva, e — importante — o **RSS logo
após a carga do modelo, antes do primeiro texto** (a medição anterior mostra ~1.011 MiB; confirme que
é estável entre rodadas, porque é a linha de base de tudo).

**Como ler:**

- `rs-faqs` sozinha atinge ~10,36 GiB ⇒ a causa está no caminho ou nos **dados** das FAQs; o TARF é
  inocente e a investigação estreita muito;
- não atinge ⇒ o platô depende de algo **acumulado** ao longo da rodada, e a ordem/sequência entra
  como suspeita principal;
- todas as quatro atingem valores parecidos ⇒ o custo é do embedder por si, quase independente da
  carga — e aí a pergunta vira por que o SP não atinge.

---

## 3 — O SP é o controle, e ele é metade da evidência

Não meça só o `rs`. **Meça o `sp` do mesmo jeito**, coleção a coleção. Ele tem carga comparável e
pica em um terço — é o contraste que qualquer hipótese tem de explicar, e é mais barato do que
teorizar.

Duas diferenças candidatas entre os dois, para você conferir **nos dados** e não por raciocínio:

- **comprimento dos textos** vetorizados (o `text_to_embed`): distribuição, não média — o que importa
  para um teto de memória é a **cauda**. Compare o P50, o P99 e o máximo entre `rs-faqs`, `rs-tarf`,
  `sp-pareceres`. Um único texto que bata no teto de 8.192 tokens custa caro numa fatia de 256;
- **quantos textos batem no teto de truncamento** em cada coleção (o `avisar_truncados` já sabe
  contar isso).

O `FATIA = 256` do `embed.rs` documenta o pior caso teórico como ~8,6 GB, "que o corpus real não
tem". **Essa afirmação merece ser verificada**, e é a hipótese mais concreta que temos: se as FAQs do
RS tiverem textos muito longos — e elas têm a resposta inteira na key desde a TAREFA-FAQ-PR —, uma
fatia de 256 delas pode custar muito mais que uma fatia de 256 pareceres curtos do SP. Isso explicaria
os três fatos de uma vez.

**Se essa hipótese se confirmar, não implemente a correção nesta TAREFA.** Reporte, e discutimos —
mexer no `FATIA` é trocar memória por chamadas, e o número certo depende do que a medição mostrar.

---

## 4 — O que NÃO fazer

- **Não altere o `batch_size = 1`**, nem nada que o afete. Ele não é escolha de performance: o código
  registra que, com lote automático, o mesmo texto saía com cosseno 0,978 entre lotes diferentes.
  Qualquer mudança ali invalida o acervo inteiro.
- **Não "otimize" antes de medir.** Foi exatamente o que eu fiz no item 3, e custou uma medição para
  descobrir que a causa era outra.
- Não toque no caminho do pack. O invariante segue: `auli update` de qualquer entidade produz pack
  byte a byte idêntico.
- Não mexa no `dedup_por_slug`, que segue fora por decisão.

## 5 — O que reportar

(a) as curvas por coleção, `rs` e `sp`, cada coleção sozinha e em primeiro lugar; (b) a linha de base
depois da carga do modelo; (c) a distribuição de comprimento do `text_to_embed` por coleção (P50, P99,
máximo) e a contagem de truncados; (d) qual das três leituras da §2 o resultado sustenta; (e) se
alguma hipótese explica os **três** fatos juntos — e, se nenhuma explicar, diga isso em vez de
escolher a menos ruim.

**Um "não sei, mas agora sei medir" é um resultado aceitável desta TAREFA.** Uma explicação plausível
e não verificada, não.
