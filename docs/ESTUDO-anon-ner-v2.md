# ESTUDO-anon-ner — Avaliação de NER para as lacunas residuais do `auli-anon` (v2)

**Repo:** `oxschellen/auli` · **Crate:** `auli-server/crates/auli-anon` · **Harness:** `auli-anon-eval`
**Status:** desenho do estudo (nada de integração ao pipeline antes do gate final)
**Consolida as conclusões de:** implementação e verificação das Fases 4–6 + medição do deploy do embedder (ago/2026)
**v2 (ago/2026):** incorpora as emendas E1–E7 da revisão — Passo 0 de prevalência,
fixtures ampliadas, split calibração×decisão, varredura do corpus real, janela
deslizante, cascata sobre texto mascarado e ajustes de gate.

---

## 1. Estado de partida — o que o determinístico já cobre

O pipeline atual é 100% determinístico, in-process, sem rede, fail-closed, com latência <5 ms:

| Fase | Cobertura | Mecanismo | Confiança |
|---|---|---|---|
| 1 | 14 identificadores estruturados (CPF, CNPJ inclusive alfanumérico, e-mail, telefone, IE, protocolo, GA, RENAVAM, placa, CEP, data nasc.) | DV/formato + contexto | recall **100%**, 0 FP |
| 4 | Razão social **com sufixo**; endereço logradouro+número; nome **após gatilho** | heurística ancorada | 0.7 |
| ~~5~~ | ~~Nome **solto** iniciado por prenome do Censo IBGE~~ | — | **não existe** |
| ~~6~~ | ~~Razão social **sem sufixo** com termo de segmento~~ | — | **não existe** |

> **Correção (2026-08-12) — as Fases 5 e 6 nunca entraram; a baseline são as Fases 1–4.**
>
> As duas linhas riscadas descreviam os **patches**, não o código. Verificado: `reconhecedores/` não
> tem `nome_dicionario.rs` nem `razao_segmento.rs`, e o registro do `lib.rs` monta doze
> reconhecedores — `CnpjAlfanumerico`, `TelefoneBr`, `InscricaoEstadual`, `Cep`, `Protocolo`, `Ga`,
> `Renavam`, `Placa`, `DataNascimento`, `RazaoSocial`, `Endereco`, `NomePessoa`. A Fase 5 foi
> **medida e reprovada** (397 falsos positivos no acervo) e a 6 caiu por empilhar sobre ela; os dois
> patches estão em `docs/`, com o diagnóstico, e o `auli-anon_pendencias` §5 e §7 são a fonte certa.
>
> **A direção do erro é contraintuitiva, e por isso vale registrar em vez de só apagar.** Uma
> baseline superestimada faz o ganho marginal do NER parecer **menor** do que é; e a lista de
> lacunas do §2, escrita como "o que sobra depois das Fases 5–6", **subestima** o que de fato sobra.
> Os dois efeitos empurram para o mesmo lado: **este estudo, como estava, subestimava o que o NER
> acrescentaria.** O gate 1 (falso positivo zero em `C-dec`) é absoluto e não é afetado.

Travas vigentes: `recall_estruturado_fase1` (100%), `recall_nao_estruturado_fase4`,
controles com **zero falso positivo** (a regra de ouro: precisão vence recall), travas de
convivência entre reconhecedores (dedup do cloakrs: span mais longo; empate → confiança).

## 2. O que ainda vaza — o território exclusivo do NER

Lacunas **nomeadas e assumidas** nos módulos (todas documentadas em código):

1. **Nome de 1 token** — `o João pediu a baixa` (mascarar prenome isolado inundaria de FP).
2. **Prenome fora do dicionário** — raro, estrangeiro, ou nas exclusões curadas
   (SANTA/SOL/NATAL…).
3. **Gatilho distante do nome** — `o contribuinte, que já esteve aqui, João Silva`.
4. **Homônimo de município composto** — pessoa chamada `Carlos Barbosa` (custo da guarda).
5. **Razão social sem sufixo E sem termo de segmento** — `a Anderle não emitiu a nota`.

> **Leia esta lista com a correção do §1.** Ela foi escrita como "o que sobra **depois** das Fases
> 5 e 6", que não existem. Sobre a baseline real (Fases 1–4), cada lacuna é **maior** do que a
> redação sugere: a 1 e a 2 são casos particulares de "**todo** nome solto vaza, com ou sem prenome
> de dicionário"; a 4 não existe como lacuna, porque a guarda de topônimo que a criava não está no
> código; e a 5 é "razão social **sem sufixo**", sem a segunda condição.
>
> Há um número medido para a 5, e é o primeiro: nas linhas rotuladas do TARF, **1.667 de 7.212
> nomes reais (23%) não têm sufixo societário nem termo de segmento** — ver a §6.1 do
> `auli-anon_pendencias`. Deixou de ser lacuna assumida.

**A pergunta do estudo:** o volume real dessas sobras nas perguntas de atendimento justifica um
modelo, dado o custo permanente que ele introduz? Isso é uma pergunta de medição, não de
opinião — e a primeira medição não exige modelo nenhum (§2.1).

### 2.1 Passo 0 — medição de prevalência (decide se o estudo continua)

O eval do §6 mede **capacidade** (recall sobre fixtures); esta etapa mede **prevalência**
(frequência das lacunas no tráfego real). A prevalência decide o estudo antes de
qualquer download.

- **Fonte:** o log de auditoria (pares original/sanitizada), sob o controle de acesso já
  vigente. Nenhum dado sai do Desktop Server.
- **Amostra:** todas as perguntas logadas até a data de corte; se o volume exceder ~500,
  amostra aleatória de 500.
- **Método:** revisão manual da coluna *sanitizada*, contando vazamentos residuais **por
  lacuna** (1 a 5). Registrar também vazamentos fora das 5 lacunas, se existirem —
  lacuna não nomeada é achado, não ruído.
- **Critério de continuação, definido antes de contar:** se menos de **1%** das
  perguntas contêm vazamento residual, o estudo encerra aqui e as lacunas permanecem
  como limitação documentada. Arquivar com os números, como a TAREFA-GPU.
- **Subproduto:** cada ocorrência real vira fixture (segredos substituídos por
  equivalentes fictícios), complementando ou substituindo as sintéticas do §6 — fixture
  colhida do tráfego vale mais que fixture inventada.

## 3. Régua de custo — referência medida no Desktop Server

O deploy real do embedder é a régua para dimensionar qualquer modelo ao lado dele:

- **BGE-M3 = BGEM3Q do fastembed** (`gpahal/bge-m3-onnx-int8`, rev `2b34e84`):
  **560 MB em disco** (544 MB `model_quantized.onnx` INT8 + 17 MB `tokenizer.json`).
- O peso vem do XLM-RoBERTa por baixo: **vocabulário de 250 mil tokens** — a matriz de
  embeddings é a maior fatia. É a "taxa multilíngue".
- **Disco ≠ RAM:** o ORT mapeia pesos e aloca ativações que escalam com `max_length` — e o
  embed roda no teto de **8192 tokens** (`embed.rs:31`). O residente em serviço é bem maior
  que 544 MB; o número que vale é o **RSS do processo** com o modelo carregado.
- Cache em `./models` (`EMBED_CACHE_DIR`, `config.rs:42`), layout HF (`blobs/` +
  `snapshots/` em symlink — `du` não conta em dobro).

Consequências para o NER:

- **Alvo de disco:** int8 entre ~70 e ~150 MB (⅕ a ⅛ do embedder).
- **RSS favorece o NER:** a pergunta do analista é curta — processamento em **janelas de
  256–512 tokens** deixa as ativações pequenas e o residente perto do disco (estimativa:
  +150–250 MB de RSS; **medir**, não estimar, no gate). Perguntas maiores que a janela
  **não são truncadas** — ver janela deslizante no §8: o argumento de RSS vale porque as
  ativações são limitadas pelo tamanho da *janela*, não da pergunta.
- **Critério de seleção derivado:** preferir modelo com **vocabulário português nativo**
  (BERTimbau, ~30 mil tokens) a um XLM-R-based — mesma qualidade em pt-BR por fração do
  peso, sem a taxa multilíngue que o próprio embedder paga.

## 4. Hardware — papel de cada peça

- **Caminho crítico = CPU** (Ryzen 9, 32 threads). Um NER int8 responde em ~10–40 ms por
  pergunta — imperceptível no fluxo do Chat. Motivo decisivo: o pipeline é **fail-closed**;
  pendurar a anonimização no driver NVIDIA/CUDA (Ubuntu 25.10, não-LTS) criaria um modo de
  falha novo em que um driver quebrado derruba o fluxo de perguntas inteiro. Regex e ONNX-CPU
  não têm esse modo de falha.
- **GPU (RTX 3060 Ti, 8 GB) = bancada.** Todos os candidatos cabem com folga sobrando mais
  da metade da VRAM. Usos: (a) rodar o eval em lote (centenas de fixtures × N candidatos);
  (b) **fine-tuning** de um BERTimbau/DistilBERT com exemplos do domínio de atendimento, se nenhum
  checkpoint pronto servir — treina na GPU, publica o ONNX int8, serve na CPU. Se o volume
  de perguntas um dia justificar, o CUDA EP do `ort` fica a uma flag de distância.

## 5. Candidatos (lista curta a confirmar)

Tamanhos aproximados de conhecimento geral; **confirmar tamanho, licença e rótulos de cada
checkpoint no Hugging Face antes de baixar** — isso muda a cada release.

| Candidato | Base / vocab | Disco fp32 → int8 (aprox.) | Observações |
|---|---|---|---|
| NER BERTimbau-base afinado no **LeNER-Br** (ex.: família `pierreguillou/ner-bert-*-lenerbr`) | BERT-base pt, ~30k vocab | ~430 MB → ~110 MB | Corpus **jurídico** brasileiro (PESSOA, ORGANIZACAO, LOCAL, LEGISLACAO…) — o domínio mais próximo do texto fiscal; primeiro da fila. **Expectativa registrada:** LeNER-Br é texto jurídico *formal*, a pergunta de balcão é coloquial — possível queda de domínio; não é motivo para tirá-lo da frente, é motivo para não se surpreender nas fixtures |
| DistilBERT pt afinado para NER | destilado, vocab pt | ~260 MB → ~65–70 MB | O mais leve da classe transformer; **linha mais especulativa da tabela** — confirmar se existe checkpoint com qualidade publicada; se não existir, sai |
| BERTimbau-base NER genérico (HAREM/notícias) | BERT-base pt | ~430 MB → ~110 MB | Baseline pt clássico |
| GLiNER small (zero-shot) | XLM-R-based | ~550–600 MB → ~150 MB | Rótulos arbitrários ("razão social") sem re-treino; paga a taxa multilíngue. **Não é token-classification ONNX padrão** — caminho de inferência próprio, custo de integração maior; esse custo entra na comparação B−A |
| spaCy `pt_core_news_md` | pipeline próprio | ~40–45 MB | **Piso de referência apenas** — Python no caminho, NER pt fraco; não é candidato a produção |

Descartados de antemão: qualquer NER que exija chamada de rede (contradição com o
propósito) e dicionário de razões sociais dos dados abertos do CNPJ (dezenas de milhões de
entradas; e a versão "maiores do país" cobre onde o atendimento não precisa e colide com o
português comum — Vale, Oi, Azul).

## 6. Desenho do eval

**Instrumento:** o `auli-anon-eval` existente, ampliado.

**Fixtures novas (o alvo):** **8–10 fixtures por lacuna do §2 (40–50 no total)** —
prioritariamente derivadas das ocorrências reais do Passo 0, completadas com sintéticas.
O gate 2 exige recall *por lacuna*; com 3–4 exemplos por lacuna a variância seria
inaceitável para um gate. Classe nova (`Classe::ResidualNer` ou equivalente),
`coberto: false` até existir reconhecedor. **As fixtures passam pelo pipeline completo
(Fases 1–6 → NER), como em produção** — ver cascata no §8: placeholders no meio do texto
perturbam o modelo, e essa perturbação deve aparecer na medição, não na surpresa.

**Controles novos (o preço):** frases institucionais com sequências capitalizadas — o NER
erra de formas que regex não erra, e é aqui que ele será reprovado ou não. Reaproveitar os
13 controles atuais e adicionar armadilhas específicas de modelo (nomes de órgãos, leis com
epônimo — `Lei Kandir` —, servidores públicos citados em normativos).

**Split calibração × decisão:** os controles são repartidos, no momento da escrita e
antes de qualquer candidato rodar, em dois conjuntos **disjuntos**:

- **C-cal** — calibra o threshold de cada candidato; uso livre durante a bancada.
- **C-dec** — rodado **uma única vez** por candidato, com threshold já congelado; é o
  conjunto que decide o gate 1. Qualquer ajuste de threshold, pós-processamento ou
  stoplist feito após contato com C-dec invalida a rodada daquele candidato.

**Colunas de custo (por candidato, tudo medido, nada estimado):**

1. disco int8 (MB);
2. **delta de RSS** do processo com o modelo carregado, janela de 256–512;
3. latência CPU int8 (p50/p95), single-thread e com o pool do server, para a pergunta de
   1 KB **e** para a maior pergunta real do corpus (a janela deslizante escala linear com
   o número de janelas).

**Colunas de qualidade:**

1. recall sobre as fixtures residuais (por lacuna do §2, não só agregado);
2. FP em C-dec (o número que manda);
3. **varredura do corpus real:** rodar o candidato sobre todas as perguntas logadas
   (mesma fonte do §2.1), revisar manualmente **apenas as detecções exclusivas do NER**
   (a diferença B−A — dezenas, não milhares) e reportar FPs confirmados / detecções
   novas e **FPs por mil perguntas**. O gate 1 continua absoluto sobre C-dec; a
   varredura fornece o que C-dec não fornece — a expectativa de FP no tráfego real;
4. regressão zero nas fixtures das Fases 1–6 (o NER entra **em cima**, nunca no lugar),
   **incluindo o round-trip** `anonimizar → restaurar == original` com o NER ligado.

**Cenários comparados:** (A) status quo Fases 1–6 — o baseline já implantado;
(B) A + cada candidato NER. A diferença B−A, coluna a coluna, é o resultado do estudo.

## 7. Gate de decisão (go/no-go)

O NER só sai da bancada se, simultaneamente:

1. **FP:** zero detecções novas em **C-dec** — a regra de ouro não relaxa para modelo.
   (Mitigação admissível: threshold calibrado em **C-cal**, nunca em C-dec; se só passa
   com threshold que mata o recall, é reprovação.)
2. **Recall:** ganho material sobre o baseline nas lacunas do §2 — "material" definido
   antes de rodar (proposta: ≥50% dos segredos residuais capturados, por lacuna
   ponderada pela prevalência do §2.1).
3. **Custo:** delta de RSS ≤ ~250 MB e **p95 single-thread ≤ ~50 ms** (pior caso por
   pergunta; a medição com o pool é reportada como informativa, não decide).
4. **Licença:** compatível com MIT (modelo E dataset de fine-tuning).

Se reprovado: as lacunas do §2 permanecem como limitação documentada — que é a situação
atual, honesta e conhecida — e o estudo fica arquivado com os números, como a TAREFA-GPU.

## 8. Integração (só após o gate — desenho antecipado)

- **Cascata:** o NER roda **sobre o texto já mascarado pelas Fases 1–6**, nunca em
  paralelo com elas. Os placeholders protegem os spans determinísticos por construção —
  a regra span-mais-longo do dedup deixa de poder atropelar um match estruturado (um
  span PESSOA de confiança 0.5 englobando `João Silva, CPF 123...` alteraria placeholder
  e restore). A confiança **0.5** fica reservada a empates dentro da própria etapa NER.
- **Trava de placeholder no harness:** nenhum placeholder `[..._N]` pode ser alterado,
  englobado ou re-detectado pela etapa NER — span do NER que intersecta placeholder é
  descartado.
- O NER vira **mais um `Recognizer`** (`auli_ner_v1`), emitindo `PiiEntity` com os mesmos
  `Custom("nome")`/`Custom("razao_social")` — placeholders e restore inalterados.
- **Janela deslizante, não truncamento:** janelas de 256–512 tokens com stride de ~50%
  (128–256), spans mapeados ao offset global e merge por união entre janelas. Truncar
  criaria uma lacuna nova e silenciosa — PII após o corte passaria sem o NER olhar, o
  modo de falha que o pipeline inteiro existe para não ter.
- Runtime: ONNX int8 via `ort`, CPU, modelo carregado no `novo()` junto com as regexes.
  Flag própria (`AULI_ANON_NER`, default **off** no primeiro deploy) — independente da
  `AULI_ANONIMIZAR_LLM`.
- Fail-closed intacto: erro do NER degrada para o pipeline determinístico (que já é
  aprovado sozinho), nunca para texto cru.
- Artefato do modelo versionado fora do git (mesmo esquema de cache do embedder), com
  hash/revisão pinados como o `gpahal` está.

## 9. Riscos nomeados

- **FP imprevisível** é o risco central: regex erra de forma enumerável, modelo não. Por
  isso o gate 1 é absoluto, os controles são a peça mais importante do eval, e a
  varredura do corpus real (§6) existe para estimar o que os controles não estimam.
- **Dependência nova de peso** (`ort` já existe via fastembed; o risco é o artefato do
  modelo, não o runtime).
- **Deriva de checkpoint:** tamanhos/licenças/qualidade mudam por release — pinar revisão
  do HF como já se faz com o embedder e com o cloakrs (`=0.3.0`).
- **Manutenção de longo prazo:** um modelo afinado próprio (plano B do §4) cria obrigação
  de re-treino; só assumir se os checkpoints prontos reprovarem por pouco.

## 10. Próximos passos, em ordem

0. **Passo 0 — prevalência no log real (§2.1). Pode encerrar o estudo aqui.**
1. Confirmar no HF os checkpoints da lista curta (tamanho int8 real, licença, rótulos;
   filtro do §5 para o DistilBERT).
2. Escrever as fixtures residuais (8–10 por lacuna, priorizando as ocorrências do
   Passo 0) + controles repartidos em C-cal/C-dec + trava de placeholder — sem tocar no
   crate.
3. Rodar o eval A/B em cascata, com janela deslizante, na bancada (GPU acelera o lote;
   medição de custo é em CPU) + varredura do corpus real.
4. Decidir pelo gate do §7 — e só então, se aprovado, TAREFA de integração pelo §8.
