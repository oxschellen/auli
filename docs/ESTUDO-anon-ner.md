# ESTUDO-anon-ner — Avaliação de NER para as lacunas residuais do `auli-anon`

**Repo:** `oxschellen/auli` · **Crate:** `auli-server/crates/auli-anon` · **Harness:** `auli-anon-eval`
**Status:** desenho do estudo (nada de integração ao pipeline antes do gate final)
**Consolida as conclusões de:** implementação e verificação das Fases 4–6 + medição do deploy do embedder (ago/2026)

---

## 1. Estado de partida — o que o determinístico já cobre

O pipeline atual é 100% determinístico, in-process, sem rede, fail-closed, com latência <5 ms:

| Fase | Cobertura | Mecanismo | Confiança |
|---|---|---|---|
| 1 | 14 identificadores estruturados (CPF, CNPJ inclusive alfanumérico, e-mail, telefone, IE, protocolo, GA, RENAVAM, placa, CEP, data nasc.) | DV/formato + contexto | recall **100%**, 0 FP |
| 4 | Razão social **com sufixo**; endereço logradouro+número; nome **após gatilho** | heurística ancorada | 0.7 |
| 5 | Nome **solto** iniciado por prenome do Censo IBGE (18.938 prenomes + guarda de 2.462 municípios compostos) | dicionário | 0.6 |
| 6 | Razão social **sem sufixo** com termo de segmento (~120 substantivos de ramo) | dicionário | 0.65 |

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

**A pergunta do estudo:** o volume real dessas sobras nas perguntas do NAVI justifica um
modelo, dado o custo permanente que ele introduz? Isso é uma pergunta de medição, não de
opinião — e o instrumento (harness + fixtures + controles) já existe.

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
- **RSS favorece o NER:** a pergunta do analista é curta — capar a sequência em
  **256–512 tokens** deixa as ativações pequenas e o residente perto do disco
  (estimativa: +150–250 MB de RSS; **medir**, não estimar, no gate).
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
  (b) **fine-tuning** de um BERTimbau/DistilBERT com exemplos do domínio NAVI, se nenhum
  checkpoint pronto servir — treina na GPU, publica o ONNX int8, serve na CPU. Se o volume
  do NAVI um dia justificar, o CUDA EP do `ort` fica a uma flag de distância.

## 5. Candidatos (lista curta a confirmar)

Tamanhos aproximados de conhecimento geral; **confirmar tamanho, licença e rótulos de cada
checkpoint no Hugging Face antes de baixar** — isso muda a cada release.

| Candidato | Base / vocab | Disco fp32 → int8 (aprox.) | Observações |
|---|---|---|---|
| NER BERTimbau-base afinado no **LeNER-Br** (ex.: família `pierreguillou/ner-bert-*-lenerbr`) | BERT-base pt, ~30k vocab | ~430 MB → ~110 MB | Corpus **jurídico** brasileiro (PESSOA, ORGANIZACAO, LOCAL, LEGISLACAO…) — o domínio mais próximo do texto fiscal; primeiro da fila |
| DistilBERT pt afinado para NER | destilado, vocab pt | ~260 MB → ~65–70 MB | O mais leve da classe transformer; qualidade a medir |
| BERTimbau-base NER genérico (HAREM/notícias) | BERT-base pt | ~430 MB → ~110 MB | Baseline pt clássico |
| GLiNER small (zero-shot) | XLM-R-based | ~550–600 MB → ~150 MB | Rótulos arbitrários ("razão social") sem re-treino; paga a taxa multilíngue |
| spaCy `pt_core_news_md` | pipeline próprio | ~40–45 MB | **Piso de referência apenas** — Python no caminho, NER pt fraco; não é candidato a produção |

Descartados de antemão: qualquer NER que exija chamada de rede (contradição com o
propósito) e dicionário de razões sociais dos dados abertos do CNPJ (dezenas de milhões de
entradas; e a versão "maiores do país" cobre onde o NAVI não precisa e colide com o
português comum — Vale, Oi, Azul).

## 6. Desenho do eval

**Instrumento:** o `auli-anon-eval` existente, ampliado.

**Fixtures novas (o alvo):** ~15–20 perguntas estilo NAVI cobrindo exatamente as 5 lacunas
do §2, cada uma com seus `segredos`. Classe nova (`Classe::ResidualNer` ou equivalente),
`coberto: false` até existir reconhecedor.

**Controles novos (o preço):** frases institucionais com sequências capitalizadas — o NER
erra de formas que regex não erra, e é aqui que ele será reprovado ou não. Reaproveitar os
13 controles atuais e adicionar armadilhas específicas de modelo (nomes de órgãos, leis com
epônimo — `Lei Kandir` —, servidores públicos citados em normativos).

**Colunas de custo (por candidato, tudo medido, nada estimado):**

1. disco int8 (MB);
2. **delta de RSS** do processo com o modelo carregado, sequência capada em 256–512;
3. latência CPU int8 por pergunta de 1 KB (p50/p95, single-thread e com o pool do server).

**Colunas de qualidade:**

1. recall sobre as fixtures residuais (por lacuna do §2, não só agregado);
2. FP nos controles (o número que manda);
3. regressão zero nas fixtures das Fases 1–6 (o NER entra **em cima**, nunca no lugar).

**Cenários comparados:** (A) status quo Fases 1–6 — o baseline já implantado;
(B) A + cada candidato NER. A diferença B−A, coluna a coluna, é o resultado do estudo.

## 7. Gate de decisão (go/no-go)

O NER só sai da bancada se, simultaneamente:

1. **FP:** zero detecções novas nos controles — a regra de ouro não relaxa para modelo.
   (Mitigação admissível: threshold de confiança do modelo calibrado no eval; se só passa
   com threshold que mata o recall, é reprovação.)
2. **Recall:** ganho material sobre o baseline nas lacunas do §2 — "material" definido
   antes de rodar (proposta: ≥50% dos segredos residuais capturados).
3. **Custo:** delta de RSS ≤ ~250 MB e p95 ≤ ~50 ms na CPU.
4. **Licença:** compatível com MIT (modelo E dataset de fine-tuning).

Se reprovado: as lacunas do §2 permanecem como limitação documentada — que é a situação
atual, honesta e conhecida — e o estudo fica arquivado com os números, como a TAREFA-GPU.

## 8. Integração (só após o gate — desenho antecipado)

- O NER vira **mais um `Recognizer`** (`auli_ner_v1`), emitindo `PiiEntity` com os mesmos
  `Custom("nome")`/`Custom("razao_social")` — placeholders e restore inalterados.
- Confiança **abaixo de 0.6** (proposta: 0.5): em qualquer empate de span, o determinístico
  vence o probabilístico. As heurísticas continuam como primeira linha; o NER pega a sobra.
- Runtime: ONNX int8 via `ort`, CPU, sequência capada, modelo carregado no `novo()` junto
  com as regexes. Flag própria (`AULI_ANON_NER`, default **off** no primeiro deploy) —
  independente da `AULI_ANONIMIZAR_LLM`.
- Fail-closed intacto: erro do NER degrada para o pipeline determinístico (que já é
  aprovado sozinho), nunca para texto cru.
- Artefato do modelo versionado fora do git (mesmo esquema de cache do embedder), com
  hash/revisão pinados como o `gpahal` está.

## 9. Riscos nomeados

- **FP imprevisível** é o risco central: regex erra de forma enumerável, modelo não. Por
  isso o gate 1 é absoluto e os controles são a peça mais importante do eval.
- **Dependência nova de peso** (`ort` já existe via fastembed; o risco é o artefato do
  modelo, não o runtime).
- **Deriva de checkpoint:** tamanhos/licenças/qualidade mudam por release — pinar revisão
  do HF como já se faz com o embedder e com o cloakrs (`=0.3.0`).
- **Manutenção de longo prazo:** um modelo afinado próprio (plano B do §4) cria obrigação
  de re-treino; só assumir se os checkpoints prontos reprovarem por pouco.

## 10. Próximos passos, em ordem

1. Confirmar no HF os checkpoints da lista curta (tamanho int8 real, licença, rótulos).
2. Escrever as fixtures residuais + controles novos no harness (sem tocar no crate).
3. Rodar o eval A/B na bancada (GPU acelera o lote; medição de custo é em CPU).
4. Decidir pelo gate do §7 — e só então, se aprovado, TAREFA de integração pelo §8.
