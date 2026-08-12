# TAREFA — fechar a operação do teto: as 27 entidades e duas notas de correção

**Contexto:** o `STRATEGY_VERSION` subiu para 3 e os vetores mudaram de verdade. Diferente da Fase 1
do incremental, **aqui não existe caminho de migração** — é re-embed mesmo. São três coisas pequenas
e uma operação longa.

---

## 1 — Antes de tudo: fechar a verificação do `rs`

Termine e reporte o que a Fase 3 pede, na ordem:

- **pico final** contra o previsto (eu previ 1,6–2,2 GiB e mandei prometer 2,5; a rodada estava em
  3,1 GiB no TARF). **A previsão errou, e a diferença importa** — ver §4;
- **distribuição de tokens do acervo cortado** (P50, P99, máximo), com o `Embedder::conta_tokens`.
  Ela fecha duas pontas: confirma a razão de ~4 chars/token que usei para traduzir 6.000 caracteres em
  ~1.500 tokens ao escolher o N, e prova que **nenhuma key encosta mais nos 8.192 do modelo** — o que
  é a justificativa retroativa de ter removido aquela checagem;
- **comparação pack a pack**: os registros **não cortados** saem byte a byte idênticos aos de
  produção. Só os cortados podem diferir. É o teste que prova que o corte fez exatamente o que devia e
  nada além;
- **remova o `Embedder::conta_tokens` no mesmo commit que reporta a distribuição.** Ele morre tendo
  servido. O argumento: `auli-core` não é publicado, então `pub` ali significa "visível ao workspace",
  e o workspace é o mundo inteiro — método público sem uso é código morto com um passo a mais. O
  `git log` guarda e o tokenizer continua lá, então readicionar é trivial se aparecer necessidade.

---

## 2 — As 27 entidades

**Não rode.** A execução em produção é do Carlos, como nas outras vezes. O que você faz aqui é
**preparar e reportar**:

- confirme que o binário do `HEAD` é o que vai rodar (o `scripts/build-packs.sh` recompila — foi
  exatamente essa armadilha que fez a regeneração anterior dar certo);
- **faça o backup dos 27 packs antes de qualquer coisa** e diga onde ficou. É a única parte
  irreversível: sem cópia fresca, não há como comparar os vetores depois nem voltar atrás no meio das
  ~2h. O `data_backup/` que existe é de uma migração anterior, não destes;
- proponha a **ordem crescente de tamanho**, com `rs` e `sp` no fim. Se algo estiver errado, aparece
  em minutos e não depois de 70;
- lembre que, com o binário novo implantado, **todo pack em versão 2 faz o servidor recusar subir** —
  é o trio de identidade validado no boot, funcionando. Isso não impede regenerar com o servidor
  antigo no ar: o binário antigo tolera pack novo (verificado na operação anterior).

---

## 3 — Nota de correção na `TAREFA-ARQUIVAR-LOG.md`

Ela está versionada em `docs/` com **duas instruções que a execução refutou**: o `serde(default)` e a
checagem do DTO no frontend. Ambas partiam de uma suposição minha errada — que o log fosse serde. Ele
é **texto plano** montado pelo `format_log_record`, nada o desserializa, e o modal exibe verbatim
(`r.text()`, D-LOG-5), então não havia DTO para quebrar.

Acrescente a nota no padrão que o repo já pratica: a refutação fica **ao lado** da hipótese, não a
apaga. Quem ler `docs/` daqui a três meses não pode tomar aquelas duas linhas como especificação do
formato.

---

## 4 — Nota de correção no `RELATORIO-MEMORIA-UPDATE.md` — e é a mais substantiva

O relatório afirma que o `FATIA` **não é o termo dominante**. Isso era verdade quando medido, e
provavelmente **deixou de ser** depois do corte.

A conta: o colbert de uma fatia é `[256, seq_max_da_fatia, 1024]` em f32. Com a key cortada,
`seq_max` ≈ 1.500 tokens ⇒ 256 × 1.500 × 4 KiB ≈ **1,5 GiB**. Somando à base (~1,0) e ao termo
quadrático de 6.000 chars (~0,5), dá ~3,0 GiB — que é o que a rodada está medindo. Ou seja: o `FATIA`
não era irrelevante, era **dominado**. Enquanto um documento de 8.192 tokens produzia um termo
quadrático de ~8,5 GiB, o termo do `FATIA` (~8 GiB — que é justamente o "pior caso documentado" de
8,6 GB que o relatório tratou como coincidência aritmética) sumia no ruído da soma. Cortada a cauda,
os dois ficaram da mesma ordem, e agora o `FATIA` é o maior.

**Não meça isto: não rode o TARF com `FATIA = 128` nem com nenhum outro valor.** Os 3,1 GiB já
resolvem o problema que motivou a investigação — a queda de 17,7 para 3,1 é 5,7×, e uma máquina
modesta dá conta. Uma rodada de 66 minutos para refinar um número que não muda decisão nenhuma não se
paga.

**A nota, porém, entra** — e é justamente por não haver medição que ela precisa ser escrita com
cuidado. Registre a conta acima **como hipótese não verificada**, com o número medido ao lado
(3,1 GiB observados contra 1,6–2,2 previstos por mim, ver §1) e o teste que a decidiria, para quem
um dia precisar. O que o relatório **não** pode continuar afirmando é que o `FATIA` é irrelevante:
essa era uma conclusão do regime antigo, e o regime mudou. "Provavelmente dominante, não medido"
é honesto; "irrelevante" passou a ser falso.

## 5 — Duas migalhas, se couberem no mesmo commit de docs

- **Comentário no `anexar_anomalias`** explicando por que aquele append **não** é atômico como os
  outros: o append existe de propósito, para que uma rodada interrompida ainda deixe registro, e
  proteger só o `reiniciar_anomalias` daria falsa sensação de segurança sobre um arquivo cuja outra
  metade continua exposta. Sem esse comentário, alguém "conserta" isso daqui a seis meses.
- **Confira se o comentário da doutrina K8** sobreviveu à consolidação do `escrever_atomico`. Ele
  estava só na cópia do `canonizar.rs` e era a única divergência entre as quatro. Se a K8 explica por
  que a escrita é atômica, o lugar dele é no helper; se é específica do `canonizar`, é na chamada.
  Reporte qual dos dois.

---

## O que NÃO fazer

- Não rode as 27 entidades.
- **Não mude o `FATIA` e não meça o `FATIA`.** A §4 é uma nota em documento, não um experimento.
- Não mexa no `batch_size = 1` nem no `EMBED_MAX_TOKENS`.
- Não toque no `dedup_por_slug` — segue fora por decisão, e a TAREFA do scraper que o envolve é outra.
- Não implemente chunking: se a comparação mostrar que os 23 documentos maiores pioraram, é conversa
  de plataforma, não desta TAREFA.

## Reportar

(a) o fechamento da §1, com os quatro itens; (b) o backup dos 27 e a ordem proposta; (c) as duas notas
de correção; (d) as migalhas da §5; (e) qualquer suposição minha que não bateu. A conta do `FATIA` da
§4 é raciocínio meu e fica **explicitamente não verificada** no documento — não a promova a fato, e
não a teste.
