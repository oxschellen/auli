# ~~TAREFA — Fase 2: migração dos packs para o formato 2~~ — **CANCELADA E ENCERRADA (10/08/2026)**

> ## O que aconteceu
>
> **O Passo 0 cancelou a própria TAREFA.** O pré-flight foi escrito e rodado como manda o roteiro
> abaixo, e o resultado tirou a migração de cena: com todas as entidades em sincronia, migrar deixou
> de comprar coisa alguma sobre re-embeddar. A decisão foi do Carlos, com o número na mão.
>
> **As duas pontas medidas pelo pré-flight** — a mesma ferramenta, antes e depois:
>
> | momento | resultado |
> | --- | --- |
> | **antes** da operação | 27 entidades · formato 1: 27 · `docs_hash` **OK: 27**, divergente 0, ausente 0 |
> | **depois** da operação | 27 entidades · formato 1: **0** · `docs_hash` **OK: 27**, divergente 0, ausente 0 |
>
> O suspeito nomeado no Passo 0 (o `rs`, cuja árvore do TARF cresce por append) estava em sincronia:
> o último `auli update` recarimbou o `docs_hash` depois da última coleta.
>
> A ferramenta **não foi commitada**. O valor dela era o número, não o código — e `tools/` é
> declaradamente o bairro do que existe para uma janela e some depois. Se a pergunta "pack e árvore
> ainda combinam?" se mostrar recorrente, o lugar certo é um subcomando do `auli`, e isso é outra
> TAREFA.
>
> ## O que foi feito no lugar
>
> `scripts/build-packs.sh` nas 27, **em ordem crescente de tamanho**, com backup do formato 1 antes
> e conferência de vetores logo depois de cada entidade:
>
> - **48.408 vetores conferidos um a um contra o backup — zero divergências.** É o total exato de
>   documentos da árvore: nenhum registro ficou de fora. Cada entidade também foi conferida quanto a
>   `pack_format=2`, ids em `doc_path`, ordem canônica e `key_hash` presente;
> - **128 minutos** no total. SP: 46 min, pico de RSS **3,1 GB** — não os 24,7 GB da dívida
>   conhecida, que o fatiamento da entrada (`4027074`) havia acabado de pagar. RS: 72 min, pico
>   **13,1 GB**, quatro vezes o SP com contagem parecida, porque o `## resumo` do TARF é a
>   fundamentação inteira do acórdão (até 27 mil caracteres) e não uma sinopse curta. **A dívida de
>   memória que sobra é do RS, não do SP**;
> - backup em `data_backup_packs_fmt1_2026-08-10/` (671 MB, 60 arquivos, md5 idêntico ao original).
>   O `data_backup/` de julho é de outra migração e não servia para isto.
>
> ## A descoberta que soltou a ordem da operação
>
> **O binário ANTIGO aceita packs formato 2** — provado por execução, não por leitura: um worktree
> em `615f789` (sem `pack_format`, sem `key_hash`) rodou `packs::load_all` sobre um pack formato 2,
> validou o manifesto, conferiu o `docs_hash` e carregou os 17 registros. Serde ignora campo
> desconhecido, nada fora da store lê `Record.id`, e o trio de identidade não mudou.
>
> Com isso não houve janela de indisponibilidade: as 27 foram regeneradas com o servidor de produção
> no ar, e o binário novo subiu depois, no tempo do operador. A escrita atômica (`.tmp` + `rename`)
> da Fase 1 é a outra metade dessa segurança — sem ela haveria o instante em que o servidor lê um
> arquivo pela metade.
>
> ## Estado final
>
> Servidor no ar com o binário da Fase 1 (conferido por `/proc/<pid>/exe`), recuperação exercitada
> no TARF do RS e devolvendo o acórdão certo. **A próxima fase é a 3** — o caminho incremental.

---

**Pré-requisito:** Fase 1 concluída (formato 2 gerado pelo `auli update`, 560 testes, escrita atômica,
`upsert` recebendo `Vec<Record<P>>`, `PACK_FORMAT = 2` recusado no boot). Esta fase é a única coisa que
separa `main` de voltar a ser implantável — hoje os 27 manifestos de produção são formato 1 e o
servidor recusa subir, corretamente, por D-INC-7.

**Especificação-mãe:** `docs/TAREFA-UPDATE-INCREMENTAL.md`, decisões D-INC-1, D-INC-2, D-INC-4,
D-INC-7 e **D-INC-13** (a guarda que governa esta fase).

**O que esta fase NÃO faz:** não re-embeda nada, não toca o caminho incremental (Fase 3), não mexe em
remoções (Fase 4), não altera comportamento do servidor além do que a Fase 1 já alterou.

---

## Passo 0 (antes de escrever código) — pré-flight do `docs_hash` nas 27 entidades

A migração recusa entidade cuja árvore divergiu do pack (D-INC-13). Precisamos saber **quantas
recusariam** antes de escrever a ferramenta, porque cada recusa significa `auli update` completo
daquela entidade — e isso dimensiona o trabalho real desta fase.

Escreva um comando de diagnóstico (pode ser um subcomando `--dry-run` da própria ferramenta, ou um
script; escolha o mais simples) que, para cada manifesto em `data/*/packs/`:

1. leia o manifesto (`manifest::read_manifest`);
2. reporte o `pack_format` corrente (ausente ⇒ 1);
3. compare `manifest.docs_hash` com `manifest::hash_docs_tree(docs_dir)` — **sem migrar nada**;
4. imprima uma linha por entidade: `entidade | pack_format | docs_hash OK|DIVERGENTE|AUSENTE`.

Rode e **pare aqui**. Reporte o resultado antes de seguir: se muitas entidades divergirem, a conversa
sobre a ordem de execução muda (talvez `auli update` completo seja mais simples que migrar), e essa é
uma decisão minha, não sua. Suspeito conhecido: `rs`, cuja árvore do TARF pode ter crescido depois do
último pack.

Reaproveite `validate_docs_hash` se ele servir; se a mensagem dele não couber num relatório de 27
linhas, compare os hashes direto — mas **não duplique** a lógica de hash da árvore.

---

## Passo 1 — a ferramenta `tools/migrar-pack-fmt2`

Vive em `tools/`, ao lado do `converter-fmt-md`, e pelo mesmo motivo declarado no `Cargo.toml` dele:
não é código de produção; existe para uma janela de migração e sai do caminho depois. Herda o
`Cargo.lock` do workspace (`tools/*` já é membro).

Para cada entidade pedida (aceite uma entidade por invocação, no padrão dos comandos do repo):

1. **Guarda de `pack_format`.** Manifesto já em 2 ⇒ nada a fazer, informe e saia com sucesso
   (idempotência: rodar duas vezes não é erro).
2. **Guarda de `docs_hash` (D-INC-13).** Divergente ou ausente com manifesto exigindo ⇒ **recuse a
   entidade inteira**, com mensagem que mande rodar `auli update`. Este é o guard-rail que impede o
   cache de nascer mentindo: se a árvore está à frente do pack, carimbaríamos o `key_hash` do conteúdo
   novo ao lado do vetor do conteúdo velho, e o defeito seria silencioso e de nascença.
3. Para cada `CollectionEntry` do manifesto:
   - leia o arquivo da coleção (`read_collection_file`);
   - para cada `Record`: extraia o `doc_path` **de dentro do payload** e escreva-o no campo `id`
     (é o que torna a migração possível sem reembed — D-INC-13);
   - recomponha o `text_to_embed` **pelo mesmo ponto único do contrato** que o `update.rs` usa, a
     partir do `.md` da árvore, e carimbe o `key_hash`. Não reimplemente a fórmula: se ela não estiver
     acessível, extraia a função em vez de copiá-la;
   - **os `embedding` são copiados verbatim.** Nenhuma chamada ao embedder nesta ferramenta — se você
     precisar do `Embedder`, algo está errado no desenho e é hora de parar e perguntar;
   - grave com a escrita ordenada e atômica da Fase 1 (D-INC-4, D-INC-5).
4. **Recarimbe o manifesto**: `count`/`bytes`/`hash` de cada coleção, `pack_format: 2`, e o
   `docs_hash`. Use a **mesma** função de carimbo que o `update.rs` usa; se ela ainda estiver embutida
   no `run_update`, **extraia-a agora** — a Fase 4 (D-INC-11) vai precisar dela pelo terceiro caminho,
   e três cópias divergentes de um carimbo de integridade é exatamente o tipo de coisa que só aparece
   quando o boot recusa e ninguém sabe por quê.
5. Não apague o pack antigo. `data_backup/` continua sendo a única cópia do formato anterior e está
   fora do git.

---

## Passo 2 — verificação (é o que decide se a fase cumpriu a promessa)

**O teste que decide:** um pack **migrado** deve sair **byte a byte idêntico** a um pack gerado por
`auli update` completo da mesma entidade, na mesma árvore. Se não sair, ou a ordenação canônica
(D-INC-4) ou o cálculo do `key_hash` está divergindo entre os dois caminhos — investigue e **pare**,
não contorne. Este teste é o motivo de a migração ter subido para antes do caminho incremental: é
muito melhor descobrir a divergência aqui do que na Fase 3, com o diff em cima.

Faça isso em **duas** entidades:

- uma pequena, para iterar rápido;
- **uma grande** (jurisprudência, milhares de registros). Não vejo mecanismo pelo qual a escala mude o
  resultado — os `id` são únicos, então não há empate de ordenação, e a serialização de float é por
  valor — mas o custo de confirmar é quase zero e fecha a dúvida de vez. Se aqui divergir e na pequena
  não, o problema é de escala e é grave: reporte antes de mexer.

Verifique também:

- **idempotência**: migrar duas vezes produz o mesmo arquivo e não é erro;
- **recusa por `docs_hash`**: com a árvore adulterada de propósito, a ferramenta recusa e não escreve
  nada (nem parcialmente — importante, porque ela grava por coleção);
- **vetores intactos**: compare os `embedding` antes e depois, byte a byte, em pelo menos uma coleção
  grande. É a promessa central: nenhum reembed;
- `cargo test` verde e `clippy` limpo, como na Fase 1.

---

## Passo 3 — o que reportar

Ao terminar, reporte nesta ordem: (a) o resultado do pré-flight do Passo 0 nas 27 entidades; (b) se o
pack migrado saiu byte-idêntico ao rebuild, nas duas entidades; (c) quantas entidades precisarão de
`auli update` completo por divergência de árvore; (d) o que você tocou fora do previsto, se algo.

**Não migre as 27 entidades de produção por conta própria.** Migre as duas de verificação e pare. A
execução em produção é minha, e o intervalo entre Fase 1 e Fase 2 é o único momento do plano em que
`main` fica inimplantável — quero decidir a ordem (migrar × `auli update`) com o resultado do
pré-flight na mão.

---

## Notas de conduta (CLAUDE.md)

- Se qualquer suposição desta TAREFA não bater com o código, **pare e diga** — não contorne em
  silêncio. Em particular: se o `doc_path` não estiver recuperável de algum payload, a premissa da
  migração-sem-reembed cai para aquela coleção, e isso muda o plano, não o código.
- Mudança cirúrgica: nada de "melhorar" o `update.rs` de passagem. A única refatoração autorizada é a
  **extração** da função de carimbo do manifesto (Passo 1.4), e ela é autorizada porque a Fase 4 já a
  exige.
- Se você escrever mais de ~200 linhas nesta ferramenta, provavelmente está reimplementando algo que
  já existe em `auli-core` ou `vector-store`. Pare e reveja.
