# ESTUDO — `docs_hash` por diretório (arquivado)

**Data:** 12/08/2026 · **Decisão: não fazer agora.** Analisado até o desenho fechado; nada
implementado. Documento existe para que retomar não custe refazer a análise.

**Atualização 20/08/2026: EXECUTADO** — revisado e implantado pela
`TAREFA-DOCS-HASH-POR-DIRETORIO` (que morreu executada, como manda o ciclo de vida). Os quatro
ajustes que a revisão acrescentou vivem nas decisões `D-DH-*` do
[auli_code.md](auli_code.md); este estudo permanece como o registro do **desenho** e dos
**caminhos descartados** — que continuam descartados pelas razões daqui.

**Para o Claude Code:** commitar como está. Nenhuma mudança de código nesta leva.

---

## O problema

O manifesto guarda **um** `docs_hash`: FNV-1a agregado sobre `(caminho, bytes)` de **toda** a árvore
`data/<id>/docs/` (`manifest.rs::hash_docs_tree` — varredura recursiva, sem filtro de kind ou
extensão). Três pontos dependem dele: o boot (`packs.rs`), e duas vezes o `remover`.

Sendo um hash só, o pipeline é **tudo-ou-nada**. Um `auli update --kind servicos` teria duas saídas,
ambas ruins:

- **carimba o hash novo** — o hash é da árvore inteira, então o manifesto passa a afirmar que *todos*
  os packs vêm dela, quando o do TARF veio de outra. A guarda deixa de proteger e passa a abençoar;
- **não carimba** — o hash antigo já não bate, e o servidor não sobe até alguém rodar as quatro.

Não há terceira saída **enquanto o hash for um só**.

## A proposta

> Trocar `docs_hash: Option<String>` por um **mapa `diretório → hash`**, com uma entrada para cada
> subdiretório de primeiro nível de `docs/` — chaveado pelo **disco**, não pela lista de coleções do
> manifesto.

É o chaveamento pelo disco que preserva a **cobertura por omissão** que existe hoje: `notas` ganha
entrada por existir, um `docs/legislacao/` futuro também, e nada depende de alguém ter declarado.

**A regra que torna o `--kind` seguro** (é a peça central):

> O hash de um diretório só é recarimbado quando o pack daquela coleção é reconstruído. Os demais são
> copiados **verbatim** do manifesto anterior.

Assim o hash significa *"este pack veio deste estado da árvore"*, e não *"a árvore estava assim quando
o manifesto foi escrito"*. Mexer em `docs/faqs/` e rodar `--kind tarf` faz o hash preservado de `faqs`
divergir, e o boot recusa **nomeando `faqs`**. Recarimbar tudo a cada rodada reproduziria o defeito de
hoje, só que particionado.

**No boot**, comparar dois mapas — três erros nomeados: chave só no disco (`docs/notas/ existe e não
está no manifesto`), chave só no manifesto (`docs/tarf/ sumiu`), hash divergente (`a árvore de tarf
mudou`). Mesmo custo de hoje: os mesmos bytes, particionados.

**Detalhe de implementação que define cobertura:** particionar a **varredura** (a mesma, restrita a
`docs/<kind>/`), nunca trocá-la pela lista de documentos que o `preparar` enxerga — senão um `.tmp`
esquecido dentro de `docs/tarf/` deixa de ser detectado.

**Compatibilidade:** ausência do mapa é **erro alto**, não "nada a validar" — senão a mudança desliga a
guarda em silêncio nos 27 manifestos. Mais varredura das 27 como conveniência. Ela **não** re-vetoriza
a jurisprudência (o trio de identidade não muda, o cache segue válido): o custo é o reset total de
`servicos` e `faqs`, ~15–20 min no conjunto.

Com isso o `--kind` sai de graça: vetoriza aquele kind, substitui aquela `CollectionEntry`, substitui
aquela chave do mapa, preserva o resto.

## O que foi descartado no caminho

- **Chavear pela `CollectionEntry`** — errado. Entrada de manifesto é lista **declarada**, e lista
  declarada omite. Abre três buracos: diretório que não é um dos quatro kinds; **kind com árvore e sem
  entrada** (o grave — raspar o TARF numa entidade nova e não rodar o `update` faz o servidor subir
  servindo três coleções como se a quarta não existisse, o *inverso* do defeito que a guarda existe
  para pegar); e arquivo solto em `docs/`.
- **Tirar a pasta `notas`** — trata o sintoma. A perda não é "existe uma pasta a mais", é que deixa de
  existir cobertura por omissão; o buraco reabre sozinho na próxima pasta.
- **Invariante "`docs/` contém exatamente os kinds com entrada"** — resolve, mas gera **mais** controle:
  duas listas a manter sincronizadas. Com o mapa não há regra a manter — a partição é o disco.
- **Hash dos hashes (raiz Merkle) na guarda** — não. Raiz paga quando permite pular a leitura das
  folhas, e o boot lê todos os bytes de qualquer forma; cria segunda fonte da verdade, vira cache que
  envelhece sob `--kind`, e **piora** o diagnóstico (volta a "algo mudou"). Uso legítimo só **fora** da
  guarda: identidade estável de release — conferir produção contra backup, ou publicar o dataset — e
  aí sempre calculada a partir do mapa na hora, **nunca armazenada e confiada**. (Hashear o
  `manifest.json` não serve: o `built_at` muda a cada execução. O `bundle.rs` já usa esse raciocínio
  com o `conteudo_sha256`.)

## Por que ficou arquivada

**O ganho é raio de alcance e diagnóstico, não tempo.** Com o incremental do PR #140, a rodada
completa do `rs` já não re-vetoriza a jurisprudência — a última imprimiu `pareceres: 372
reaproveitados, 0 a vetorizar` e `tarf: 22476 reaproveitados, 0 a vetorizar`. O que sobra é I/O de pack
(o do TARF é 311 MB de 344 MB da entidade): poupar isso são minutos.

Não há defeito ativo, e o preço é varredura nas 27 mais a guarda de boot reescrita — dentro da janela
em que se quer máxima consistência de saída.

**Gatilhos para retomar:** uma coleção nova; uma entidade com árvore parcial; ou um susto de boot que o
hash agregado não saiba nomear.

**Nota:** a mudança é **neutra em saída** — não toca key, vetor nem banda. Se for retomada, cabe
inteira dentro de uma janela de congelamento.
