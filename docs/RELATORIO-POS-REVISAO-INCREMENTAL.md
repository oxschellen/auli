# RELATÓRIO — execução da TAREFA-POS-REVISAO-INCREMENTAL

**Para:** Claude Fable
**Assunto:** os cinco itens da revisão, executados; três premissas da TAREFA que não bateram com o
código ou com a medição.
**Base:** `main` em `e3fedc1`. **Nada commitado** — quatro arquivos modificados na árvore de
trabalho. **Nenhum pack de produção alterado** (verificado byte a byte; ver §7 e a ressalva da §9).

Segue a ordem que a TAREFA pediu em "O que reportar": (a) o item 1 e seus dois testes; (b) o teste
de equivalência e o caso com órfão; (c) o pico de RSS antes e depois do item 3; (d) as suposições
que não se sustentaram — consolidadas na §8 por serem o que mais muda o desenho.

---

## 1 — DEFEITO: `auli remover` apagava a proteção do `docs_hash`

Duas mudanças de comportamento em `crates/auli-cli/src/remocoes.rs`:

- **a guarda**, em `remocoes.rs:87`, logo depois de ler o manifesto e antes de qualquer escrita —
  `manifest::validate_docs_hash(&docs_dir, &manifesto)`, reaproveitada como a TAREFA sugeriu. A
  mensagem dela já traz o remédio; acrescentei **uma** linha dizendo quem recusou e em que ordem
  ("A árvore está à frente do pack: rode `auli update` antes do `remover`"), em vez de reformular o
  erro;
- **a linha que reescrevia `manifesto.docs_hash` saiu** (`remocoes.rs:162`). O valor lido é escrito
  de volta intacto.

**Testes.** `recusa_quando_a_arvore_esta_a_frente_do_pack` (`remocoes.rs:435`): pack carimbado, log
aprovado, um `.md` novo chega à árvore ⇒ erro com o remédio, **bytes do pack idênticos**, **bytes do
manifesto idênticos**, log ainda no disco. `docs_hash_e_preservado_nao_recalculado`
(`remocoes.rs:476`): caminho normal, manifesto sem `docs_hash` continua `None`, manifesto carimbado
em dia sai igual ao que entrou, e nas duas metades o pack de fato mudou.

**A asserção que a TAREFA especificou era vazia.** "O `docs_hash` depois é idêntico ao de antes" não
prova nada quando o campo é `Some(h)`: a guarda só deixa passar se a árvore hasheia exatamente `h`,
então recalcular produziria o mesmo valor e o teste passaria com a linha defeituosa intacta. O caso
que discrimina é o manifesto **sem** `docs_hash` com árvore em disco.

Isso expôs a estrutura do item, que vale registrar porque decide se os dois testes ficam: **a guarda
e a remoção da linha defendem cenários disjuntos.**

| estado do manifesto | quem pega | por quê |
|---|---|---|
| `Some(h)`, árvore divergente | **só a guarda** | o recálculo seria inofensivo: a guarda já garantiu que a árvore hasheia `h` |
| `None`, árvore em disco | **só a remoção da linha** | `validate_docs_hash` devolve `Ok` por construção quando o campo é `None` |

A partição é perfeita, e a verificação por mutação a confirma: reintroduzido o recálculo, falha
exatamente `docs_hash_e_preservado_nao_recalculado`; removida a guarda, falha exatamente
`recusa_quando_a_arvore_esta_a_frente_do_pack`. Nenhum dos dois é redundante — se alguém amanhã
reintroduzir o recálculo "porque é inofensivo", ele é inofensivo só no ramo `Some`; se alguém tirar
a guarda "porque a gente não recalcula mais", o ramo `Some` divergente volta a passar.

**O que o caso `None` significa** é pior que "carimbo inventado": ele **cimenta a divergência num
estado que passa**. Manifesto sem `docs_hash` é pack sem essa proteção; o `remover` ligava a
proteção com o hash da árvore atual, que pode estar à frente do pack, e a partir dali o boot valida
contra um valor que atesta uma sincronia que nunca existiu. Quem tem o direito de ligar a guarda é o
`auli update`, que carimba depois de fazer o pack corresponder à árvore.

**Bônus não previsto na TAREFA:** a guarda endurece também o caso da **árvore ausente**.
`validate_docs_hash` tem ramo próprio para "o manifesto exige hash e a árvore não existe" ⇒ erro.
Antes, com a árvore deletada, `preparar` devolvia vazio, tudo virava órfão e o `remover` removeria
tudo o que estivesse aprovado. A regra de interseção limitava o dano ao que o humano tinha lido;
recusar é melhor que limitar.

---

## 2 — LACUNA: a equivalência "incremental == do zero" passou a ser testável

**A costura**, em `update.rs:141`: `trait Vetorizador` com `embed_dense` e `conta_tokens` — os dois
métodos que o caminho de ingestão chama, e nenhum além. Implementado pelo `Embedder` real;
`ingest_items` e `avisar_truncados` passam a receber `&dyn Vetorizador`. Vive ao lado de quem o usa,
não em `auli-core`. Nada mais foi tocado no `ingest_items`.

**`incremental_e_reescrita_total_produzem_o_mesmo_arquivo`** (`update.rs:1214`) — **passa nos quatro
cenários**: nenhuma mudança, documento novo, documento com resumo novo, e os dois juntos. Cada
cenário monta o pack de ontem, mexe na árvore, roda a rodada incremental e roda a reescrita total do
mesmo estado num diretório separado; a asserção é igualdade **byte a byte** do arquivo de pack.

**`com_orfao_os_dois_caminhos_divergem_e_isso_e_correto`** (`update.rs:1256`) — **diverge como
esperado**, e o teste afirma os dois lados em vez de só a diferença: o incremental fica com
`[a-1, b-2]`, a reescrita total com `[a-1]`, o log de remoções existe num diretório e não existe no
outro. A divergência está nomeada como comportamento correto (D-INC-9), justamente para que ninguém
a "conserte" depois achando que o incremental deixa lixo para trás.

**Verificação por mutação** (é o que diz se o teste morde): servir um vetor errado do cache derruba
o teste de equivalência; trocar o `if !incremental { reset }` por um `reset` incondicional derruba o
do órfão. Um defeito, um teste.

Detalhe de implementação que evita um mau dia: a comparação passa por um helper `diferenca()` que
devolve tamanhos e o índice do primeiro byte divergente. Um `assert_eq!` sobre `Vec<u8>` despejaria
dezenas de milhares de bytes de JSON no console e não diria nada.

---

## 3 — MEMÓRIA: a mudança está feita; o ganho previsto não existe

`vetores_reaproveitaveis` passou a ler o pack com um payload que o serde descarta (`update.rs:400`).
`orfaos_do_pack` ficou como está — ele precisa de `titulo` e `link`.

**Pico de RSS do `rs`**, `auli update` completo, mesma árvore e mesmos packs de partida, duas
rodadas de 5 minutos, medidas com `/usr/bin/time -v`:

| | pico | wall clock |
|---|---|---|
| antes (payload `String`) | 11.236.652 KB = **10,72 GiB** | 5:01,7 |
| depois (payload descartado) | 11.238.932 KB = **10,72 GiB** | 5:02,0 |

Diferença de 2,3 MB — ruído, e no sentido contrário. Amostrei a curva de `VmRSS` a cada 200 ms para
saber por quê:

```
t =   0 s    1.011 MiB   carga do modelo
t =  21 s   10.358 MiB   ← platô, durante a vetorização das FAQS
t = 300 s   10.353 MiB   o platô não cede (serviços, faqs e pareceres terminaram)
t = 301 s   10.976 MiB   ← PICO, na ingestão do TARF
t = 304 s    9.071 MiB   fim
```

**10,36 GiB já estavam de pé aos 21 segundos**, durante a reescrita total das faqs, antes de
qualquer pack de TARF ser aberto. Todo o pipeline do TARF — três leituras do arquivo de 311 MB, o
`apply` e a serialização — acrescenta ~620 MiB sobre esse platô. A dívida de memória do `rs` é do
embedder/ONNX, não da desserialização do payload.

E a atribuição não se sustenta nem estruturalmente: **numa REGENERAÇÃO o `vetores_reaproveitaveis`
nem chega a abrir o arquivo** — com o cache desligado ele retorna na primeira linha. Os 13,1 GB da
regeneração não podem ter vindo daqui.

**Mantive a mudança** (é estritamente menos trabalho, e o payload é peso morto naquele ponto), mas
**reescrevi o comentário** que eu mesmo havia colocado atribuindo-lhe o pico. Num repo que trata
comentário como contrato, deixar a atribuição errada seria pior que não comentar: o comentário agora
diz o que foi medido, diz que numa regeneração a função sequer lê o arquivo, e manda a dívida de
memória para a outra TAREFA.

---

## 4 — Comentários que descreviam o desenho morto

Os quatro da tabela, corrigidos. No doc de `arquivos_md` (`update.rs:199`) dei o motivo **atual** em
vez de apagar a menção: o id não é mais posicional, e o que a ordenação sustenta hoje é a ordem
canônica de escrita (D-INC-4) — ler a árvore em ordem de nome é o que faz o `sort` da escrita ser um
no-op em vez de um reembaralhamento por rodada.

No `vector-store`: o doc do módulo passou a dizer `reset`/`apply`; o doc de `reset` (`write.rs:35`)
passou a dar o motivo real (política total do D-INC-8, e por que o caminho incremental nunca o
chama — D-INC-9); o teste virou `apply_rejects_mismatched_dimension`.

**Fora da lista:** há mais dois comentários com o mesmo defeito, nos testes
`ordem_e_estavel_por_nome_de_arquivo` e `preparar_servicos_le_arvore_em_ordem`, repetindo
literalmente "os `id-N` do pack derivam dela". Corrigi junto — é a mesma frase morta que o item
existe para matar. Reverter esses dois é trivial se a preferência for ficar estrito à tabela.

---

## 5 — As duas menores

`escrever_log_remocoes` escreve em `.tsv.tmp` e faz `rename` (`update.rs:490`), na mesma disciplina
do pack (D-INC-5).

O doc de `run_remover` (`remocoes.rs:63`) ganhou o parágrafo sobre a **não-atomicidade entre pack e
manifesto**: são dois arquivos, cada um atômico, os dois juntos não; morrer na janela deixa o boot
recusando por hash de pack; a recuperação é `auli update`, as remoções já aplicadas permanecem
(os órfãos não estão na árvore de qualquer forma) e o log volta a ser gerado por lá.

---

## 6 — Estado da verificação

96 testes no `auli-cli`, 14 no `vector-store`, workspace inteiro verde.
`cargo clippy --workspace --all-targets` e `cargo fmt --check` limpos.

## 7 — O invariante que a TAREFA pediu

`auli update` do `rs` inteiro, contra uma cópia isolada da árvore e dos packs:

```
rs-pareceres: IDÊNTICO    rs-servicos: IDÊNTICO
rs-tarf:      IDÊNTICO    rs-faqs:     IDÊNTICO
```

Byte a byte, nas quatro coleções — inclusive nas duas de reescrita total, que foram re-vetorizadas
do zero e saíram iguais. Nada saiu do escopo.

---

## 8 — As três premissas da TAREFA que não bateram

1. **A asserção do teste (b) do item 1 é vazia** como especificada. Com a guarda no lugar, o ramo
   `Some(h)` não distingue preservar de recalcular; o caso que discrimina é `None` com árvore em
   disco. Ver §1 — a correção não muda o desenho do item, só onde o teste tem tração.
2. **`serde_json::value::IgnoredAny` não existe.** O tipo é `serde::de::IgnoredAny` (o `serde` já é
   dependência direta do `auli-cli`).
3. **O item 3 não reduz o pico de RSS**, e a causa apontada — o payload materializado três vezes —
   não é a causa do pico nem poderia ser a dos 13,1 GB da regeneração. Ver §3.

## 9 — Dois efeitos colaterais, declarados

**`auli-server/target/release/auli` foi recompilado** com estas mudanças ainda não commitadas —
precisei do binário para medir. Se o servidor for reiniciado agora, ele sobe com este código. (Ele
já estava desatualizado antes disto: o processo em execução roda um inode deletado, de uma build
anterior à do disco.)

**`data/rs/packs/rs.manifest.json` foi alterado sem intenção.** Isolei a rodada de medição com uma
árvore de *hardlinks*, para não copiar 700 MB. Isso é seguro para os packs, porque
`write_collection_file` escreve em `.tmp` e faz `rename` — o inode original nunca é tocado. Mas
`write_manifest` é um `fs::write` direto, sem rename: ele escreveu **através** do hardlink, no
arquivo de produção. Erro de método meu, e vale registrar a assimetria que o expôs — **a escrita do
pack é atômica por D-INC-5, a do manifesto não é**, e o boot depende das duas.

O dano é um campo: `built_at` passou de ontem para hoje. Todo o resto é idêntico e verificável —
`version` continua `"1"` (igual às outras 26 entidades), e `count`, `bytes`, `hash` de cada coleção
e o `docs_hash` foram computados sobre arquivos byte a byte iguais aos de produção, logo descrevem
corretamente o que está no disco. O manifesto está coerente com os packs e o boot passa. Não
restaurei um `built_at` anterior: o valor de hoje é verdadeiro sobre quando o manifesto foi escrito,
e um carimbo fabricado seria pior que a marca honesta.

## 10 — O que ficou fora, de propósito

O critério de desempate do `dedup_por_slug` do scraper, intocado. Nenhuma outra mudança estrutural
no `ingest_items` além da costura. E a dívida de memória do `rs`, que a §3 localiza no embedder —
assunto de outra TAREFA, com uma medição a favor agora: o platô de 10,36 GiB aparece na vetorização
das faqs, com 1.947 textos, muito antes do TARF.
