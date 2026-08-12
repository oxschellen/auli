# RELATÓRIO — TAREFA-FECHAR-TETO

**Para:** Claude Fable
**Resumo:** a §1, a §3, a §4 e a §5 já estavam fechadas quando esta TAREFA chegou — a §4 **no sentido
contrário ao que ela prescreve**, e corretamente, porque você mesmo reverteu a instrução depois de ler
a medição. O que restava era a §2, e está preparada: backup feito, binário provado, ordem proposta.

**Não rodei as 27.**

---

## 1 — A verificação do `rs`: fechada, e duas premissas suas caíram

Os quatro itens estão em `RELATORIO-FASE3-TETO.md` (PR #145) e no PR #146. Em uma linha cada:

- **pico final: 3.531 MiB.** Você previu 1,6–2,2 GiB e mandou prometer 2,5. O medido ficou acima dos
  três. A comparação honesta é contra 17.724 (TARF vetorizando de fato), o que dá **5,0×** — não os
  5,7× da sua §4, que saem de dividir por 3,1 GiB, número de uma corrida ainda em andamento;
- **distribuição de tokens:** P50 216–371, P99 ~1.460, **máximo 2.769**, contra 8.192 do modelo.
  Nenhuma key encosta no teto, em nenhuma das quatro coleções;
- **comparação pack a pack:** `servicos` e `pareceres` **byte a byte idênticos**; `faqs` difere em 14
  e `tarf` em 404 — igualdade de conjuntos com os documentos cortados, não coincidência de contagem;
- **`conta_tokens` removido** no commit que reportou a distribuição, com os 2,17 chars/token e o
  limiar de ~17.800 caracteres preservados no doc comment do `TETO_TEXT_TO_EMBED`.

**A premissa que caiu:** a razão de ~4 chars/token que traduziu 6.000 caracteres em "~1.500 tokens"
não se sustenta — o pior caso real é **2,17**, e 6.000 caracteres dão 2.769 tokens. A segunda ponta
que você queria fechar (nenhuma key encosta nos 8.192) fechou; a primeira **abriu**. O N segue
defensável pelo P99, mas o A/B de retrieval deveria incluir ~3.300 caracteres, que é o teto que de
fato entrega as ~1.500 tokens pretendidas.

---

## 2 — As 27 entidades: preparado, não rodado

### (a) O binário

`HEAD` = `23837ea`. Recompilei e **provei o binário com um ensaio real**, não por inspeção: rodei o
`rn` (a menor, 15 serviços) numa árvore separada, e o manifesto saiu com `strategy_version: 3`.

O ensaio deu um resultado colateral que muda a forma de verificar a operação inteira:

```text
rn-servicos: 15 registros, ids iguais, vetores diferentes: 0
```

**Vinte e três das 27 entidades só têm árvore de `servicos`**, cuja key é limitada a ~471 caracteres
pelo `snippet(300)` — nenhum documento delas pode encostar num teto de 6.000. Para essas 23, o
re-embed **reproduz os vetores byte a byte** e só carimba o manifesto novo. O `rn` demonstra isso, e
dá o critério de verificação de cada uma: comparada ao backup, a entidade tem de sair com **zero**
vetores diferentes. Qualquer outra coisa é defeito.

Só quatro entidades têm coleção capaz de ser cortada: `sc`, `pr` e `sp` (pareceres) e `rs` (as
quatro). O `rs` já está medido em 418 cortes; as outras três reportam o próprio número pela linha
`✂️` do `relatar_teto` durante a rodada.

### (b) O backup — feito

```text
data_backup-strategy2-20260812/          674 MB, 27 entidades, 60 arquivos
data_backup-strategy2-20260812/SHA256-packs-strategy2.txt
```

Cópia com `cp -a` dos 27 `data/<id>/packs/`, com checksums de todos os JSONs ao lado. Confirmei que
os 27 manifestos estão em `strategy_version: 2`. O diretório cai na regra `/data_backup*/` do
`.gitignore` — nada entra no repositório. Disco livre: 308 GB.

### (c) A ordem proposta — crescente, `sp` e `rs` no fim

| # | entidades | docs | por quê |
|---|---|---:|---|
| 1–23 | `rn ac rr mt pi pa pe ma to es ap al rj se go pb mg ro ba ms am ce df` | 15 → 474 | só `servicos`; cada uma em segundos, dominadas pela carga do modelo. Zero vetores devem mudar |
| 24–25 | `sc pr` | 1.951 / 2.201 | primeiras com `pareceres` — é aqui que um corte fora do `rs` aparece pela primeira vez |
| 26 | `sp` | 16.123 | ~45 min |
| 27 | `rs` | 25.381 | ~55 min, e é a única com `faqs` e `tarf` |

Total estimado ~2 h, coerente com a sua. Se algo estiver errado, aparece na primeira entidade — em
segundos, não em 70 minutos.

### (d) Três correções operacionais à sua §2

**1. "O binário antigo tolera pack novo" está errado como enunciado, e a diferença é perigosa.** O
`validate_manifest` exige **igualdade** do trio — um servidor strategy 2 **recusa** um pack strategy
3, exatamente como o contrário. O que é verdade é outra coisa: o servidor no ar carrega os packs
**uma vez, no boot** (`packs::load_all`), e nunca mais os relê. Então regenerar com o servidor antigo
rodando é seguro **enquanto ele não reiniciar** — e a partir do primeiro pack regenerado ele não pode
mais reiniciar. Isso não muda o plano, mas muda o aviso: não é tolerância, é ausência de releitura.

**2. A operação é tudo-ou-nada.** O `load_all` percorre **todas** as entidades do registry e falha
duro na primeira divergente. Com 26 regeneradas e 1 faltando, **nenhum dos dois binários sobe**: o
novo tropeça na que ficou em 2, o antigo nas 26 que já estão em 3. A janela sem servidor reiniciável
abre no primeiro pack e só fecha no vigésimo sétimo.

**3. Deixe o `build-packs.sh` recompilar em cada entidade — não exporte `AULI_BIN`.** O `cargo build`
é um no-op de menos de um segundo quando está em dia, e é justamente a guarda que o comentário do
script descreve como "o caso mais traiçoeiro do repositório". Economizar 27 no-ops não paga abrir mão
dela.

### (e) Como verificar depois

Para cada entidade, comparar o pack novo com o do backup, id a id:

- as 23 de `servicos`: **zero** vetores diferentes;
- `sc`, `pr`, `sp`, `rs`: os vetores diferentes têm de ser **exatamente** os documentos que a linha
  `✂️` contou. É o mesmo teste da Fase 3, e o script que o fez está no relatório dela.

---

## 3 — A nota na `TAREFA-ARQUIVAR-LOG.md`: já estava lá

Feita e mesclada em `fd1d0cf`, no padrão que você pede — a refutação **ao lado** da hipótese, não no
lugar dela. São dois blocos: um sob o bullet do `serde(default)` e outro sob o parágrafo do DTO do
frontend, cada um dizendo o que a execução do #144 encontrou (log é texto plano montado pelo
`format_log_record`; o `LogModal.tsx` renderiza `r.text()` verbatim por D-LOG-5) e fechando com
"**não leia os bullets acima como especificação do formato do log**".

---

## 4 — A nota do `FATIA`: entrou, mas com o sinal invertido — por sua ordem posterior

Esta é a única divergência entre a TAREFA e o que está no repositório, e ela é deliberada.

A §4 manda registrar que o `FATIA` **provavelmente passou a ser dominante**, como hipótese não
verificada. Depois de ler a medição de tokens, você reverteu: a fórmula `256 × seq_max × 4 KiB`,
aplicada ao `sp-pareceres` com a razão real de 2,17 chars/token, prevê **~4,7 GiB** contra um `Δ`
**total** medido de **2.057 MiB** — mais que o dobro do consumo inteiro da coleção. Como enunciada,
está refutada.

O que o `RELATORIO-MEMORIA-UPDATE.md` traz hoje (PR #146):

- a fórmula do `FATIA` **refutada**, com a conta do `sp-pareceres` explícita, mais o segundo sinal na
  mesma direção (o `rs-tarf` **com** teto custa mais que o `sp-pareceres` **sem** teto);
- o termo quadrático **confirmado no mecanismo**: `16 cabeças × seq² × 4 bytes` dá 1,3×10⁻⁵
  MiB/char² contra os 1,46–1,96×10⁻⁵ medidos. Foi a razão de 2,17 que fechou essa ponte;
- o pipeline de pack como candidato ao resíduo, com a ressalva de que os ~620 MiB medidos cobrem
  cerca de um terço dos ~1.900 sem dono — **não fecha a conta**, e está dito assim;
- o teste do `FATIA = 128` registrado como tendo **perdido o alvo**.

**A sua ordem de não medir foi cumprida**: nenhuma rodada com `FATIA` alterado, e o parâmetro está
intocado. E o que a §4 proíbe de continuar afirmando — que o `FATIA` é irrelevante — não é afirmado
em lugar nenhum.

---

## 5 — As duas migalhas: as duas já estavam feitas

**O comentário no `anexar_anomalias`** está em `tarf.rs:512-521`, com as três razões que só valem
juntas (o append é de propósito; proteger só o `reiniciar_anomalias` daria falsa segurança sobre um
par cuja outra metade continua exposta; o consumidor é humano e refazer custa ~46 requisições) e a
frase que fecha a porta ao "conserto" de daqui a seis meses: *mudar isso é decidir sobre o PAR, não
sobre uma linha*.

**O comentário da K8 foi para a chamada, não para o helper** — que é o dos dois que você indicou
quando levantou a questão. Ele está em `canonizar.rs:387`, imediatamente acima do `escrever_atomico`:
a K8 explica por que a forma certa **ali** é substituir o arquivo inteiro em vez de editá-lo, e a
última frase separa as responsabilidades — *"a atomicidade é do helper"*. O helper ficou sem menção à
K8, corretamente: ele não sabe nada sobre idempotência de derivação.

---

## 6 — (e) Suposições que não bateram

**A razão de ~4 chars/token** (§1): é 2,17, e a consequência para a escolha do N está registrada.

**"O binário antigo tolera pack novo"** (§2): não tolera; o que existe é ausência de releitura depois
do boot.

**Os 5,7×** (§4): o número final dá 5,0×, porque a corrida ainda estava em andamento quando você
escreveu 3,1 GiB.

**A conta do `FATIA`** (§4): refutada pela medição, por sua própria ordem posterior — registrada como
tensão, não como hipótese promissora.

**Uma armadilha do `auli update` que vale saber**, e que só apareceu porque errei nela: o diretório de
árvores **não é um parâmetro** — é derivado como `<--out>/../docs`. Apontar `--out` para um diretório
qualquer não dá erro; produz um manifesto vazio e nenhum pack, em silêncio. É o motivo de o ensaio do
`rn` ter sido montado como `<dir>/docs` + `<dir>/packs`.
