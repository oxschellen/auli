# TAREFA-FAQ-PR — Embedar Pergunta+Resposta nos itens de FAQ (revisada contra o repo)

## Objetivo

Mudar o texto que gera o vetor dos itens de FAQ: hoje embedamos breadcrumb +
pergunta; passaremos a embedar **assunto + pergunta + resposta** juntos. Nada
muda no que é servido ao usuário — a resposta continua vindo do mesmo lugar de
hoje (`stored_repr` intocado). Só o texto de embed muda.

Motivação: um vetor por item contendo P+R melhora recall (a resposta carrega
vocabulário que a pergunta não tem) sem perder granularidade; o assunto no
cabeçalho desambigua perguntas parecidas entre temas ("qual o prazo?" existe em
vários assuntos).

Esta versão foi revisada contra o estado real do repo (pós PRs #107/#108): as
"verificações iniciais" da spec original estão RESPONDIDAS abaixo, a fase P1 já
está feita, e um item que faltava na spec — a mudança de `STRATEGY_VERSION`
(**reset para 1**, decisão do Carlos: rescrape completo, nenhum pack anterior
sobrevive em lugar nenhum) — foi incorporado (D-FAQPR-7) com a correção
correspondente do rollback.

## Verificações iniciais — RESPONDIDAS

1. **Campos do item de FAQ** (`auli-contract`, struct `Faq`): `pergunta`,
   `resposta`, `origin`, `url`. **Não existe campo `assunto`/`categoria`** — o
   papel do `{assunto}` do template é do `origin`, o breadcrumb da página
   (ex.: `"Inicial | Perguntas Frequentes | IPVA"`), que pode ser vazio.
2. **Onde o texto de embed é montado hoje:** ponto único
   `auli_contract::compose_faq_text_to_embed(origin, pergunta)` (lib.rs), usado
   pelo produtor (`derive_faqs::faq_from_raw`) e pelo `update`
   (`preparar_faqs`, que rematerializa ao ler a árvore `docs/faqs/*.md`, a
   FONTE desde a TAREFA-FAQS-MD). Não há outra montagem.
3. **Backup do pack para o A/B (P5):** copiar `data/packs/<id>-faqs.json` e o
   `<id>.manifest.json` para um diretório de backup ANTES do update da P4.
   Atenção D-FAQPR-7: o backup não volta a subir num binário novo (ver P6).

## Decisões fechadas (D-FAQPR-*)

**D-FAQPR-1 — Template do texto de embed (exato, nesta ordem):**

```
{assunto}
P: {pergunta}
R: {resposta}
```

`{assunto}` = `origin`. Se vazio, omitir a primeira linha — nunca emitir linha
vazia ou placeholder. **Extensão pelo mesmo princípio:** resposta vazia (corpo
vazio é estado legal da árvore) ⇒ omitir a linha `R:` — nunca `R:` pendurado.

**D-FAQPR-2** — Modelo mantido: BGE-M3 via fastembed-rs (1024 dims, 8192
tokens). Esta TAREFA não troca modelo nem mexe em GPU.

**D-FAQPR-3** — Serving inalterado: chat, `/v1/retrieve`, MCP e frontend não
mudam. A resposta servida é a mesma de hoje (`stored_repr` intocado).

**D-FAQPR-4** — Estimador de tokens: chars/4, o mesmo critério do `--dry-run`
da sinopse. Sem tokenizer real nesta fase.

**D-FAQPR-5** — Sem truncamento silencioso: item acima do limite gera erro
explícito agregado no relatório do update — nunca embed truncado por baixo dos
panos.

**D-FAQPR-6** — Escopo: só a coleção de FAQs. Pareceres, serviços e conteúdos
intocados. Leis fora (não existem no corpus).

**D-FAQPR-7 (novo) — `STRATEGY_VERSION: 4 → 1` (RESET, não bump).** O
doc-comment da constante (`auli-core/src/manifest.rs`) manda mudar o número
sempre que o que é embedado muda; a validação do boot é por IGUALDADE estrita,
então qualquer valor ≠ 4 fecha a porta para os packs atuais. Decisão do Carlos:
a numeração RECOMEÇA em 1, casada com o rescrape completo das FAQs — recomeço
limpo de código e de dados juntos. **Pré-condição declarada e assumida: nenhum
pack da era v1 ORIGINAL sobrevive em lugar nenhum** (servidor, backups, outras
máquinas) — é ela que torna o reset seguro, porque um pack antigo carimbado
"1" validaria silencioso num binário "1 novo". Consequências: (a) TODO pack
v4, de TODAS as entidades, é recusado no boot do binário novo ⇒ re-`update`
geral obrigatório na promoção; (b) o rollback NÃO é só trocar o diretório do
pack — ver P6; (c) bumps futuros seguem do 1 em diante (2, 3, ...) — a
narrativa v2–v4 antiga vira nota histórica no doc-comment.

**D-FAQPR-8 (novo) — Normalização pelo utilitário da casa:** o colapso de
espaços/quebras é o `colapsa_linha` já existente em `mddoc.rs` (`pub(crate)`,
mesmo crate) — não reimplementar.

## Fases

Cada fase é um incremento pequeno, com `cargo test` verde e critério de aceite
próprio. Não avançar sem o anterior cumprido.

### P1 — Localizar e congelar o comportamento atual — **JÁ FEITA**

Cumprida pelos PRs #107/#108: a montagem É uma função pura no contrato
(`compose_faq_text_to_embed`) com o teste `compose_faq_congela_os_dois_ramos`
capturando o comportamento atual (breadcrumb + pergunta). Nada a fazer.

### P2 — Novo builder do texto de embed

Substituir o corpo (e a assinatura) da função no `auli-contract`:

```rust
/// Key de embedding de uma FAQ (estratégia 1 — numeração reiniciada; D-FAQPR-1):
/// assunto (= breadcrumb `origin`)
/// + `P:` pergunta + `R:` resposta, um por linha; linha de campo vazio é
/// OMITIDA (nunca linha vazia, placeholder ou `R:` pendurado). Campos
/// normalizados por `colapsa_linha` — o texto final tem no máximo 3 linhas, e o
/// determinismo byte a byte é pré-condição de hash estável.
///
/// Na numeração aposentada (até a antiga v4) a key era só breadcrumb + pergunta
/// (`QuestionKey`) — cega para a resposta. Vive no contrato porque produtor (`derive_faqs`) e `auli update`
/// (rematerialização da árvore `docs/faqs/*.md`) usam o MESMO ponto.
pub fn compose_faq_text_to_embed(origin: &str, pergunta: &str, resposta: &str) -> String {
    let assunto = mddoc::colapsa_linha(origin);
    let pergunta = mddoc::colapsa_linha(pergunta);
    let resposta = mddoc::colapsa_linha(resposta);
    let mut out = String::new();
    if !assunto.is_empty() {
        out.push_str(&assunto);
        out.push('\n');
    }
    out.push_str("P: ");
    out.push_str(&pergunta);
    if !resposta.is_empty() {
        out.push_str("\nR: ");
        out.push_str(&resposta);
    }
    out
}
```

A mudança de assinatura é proposital: o compilador arrasta os DOIS call sites
(`derive_faqs::faq_from_raw` → `&raw.resposta`; `preparar_faqs` → `&corpo`,
computado antes do move) — conferir que não aparece um terceiro.

Testes golden no contrato (substituem o `congela_os_dois_ramos`), byte a byte:
(a) item completo; (b) sem assunto; (c) sem resposta; (d) campos com espaços
múltiplos e quebras sujas na resposta → uma linha só. Aceite: `cargo test`
verde; a fórmula nova ainda não gera pack (isso é P4).

### P3 — Guardrail de tamanho

No `preparar_faqs` (o caminho do update — é ele que alimenta o embedder):
estimar `text_to_embed.chars().count() / 4`; estimativa > 7000 ⇒ o item entra
numa lista de violações `(caminho do .md, chars, tokens estimados)`. Ao fim da
leitura, lista não-vazia ⇒ erro alto agregado (padrão da casa, como
`recusar_pareceres_sem_sinopse`: amostra de até 5 + reticências), a vetorização
é recusada. Teste com `.md` sintético gigante em tempdir confirmando o erro e o
caminho no texto.

Aceite: `cargo test` verde; nenhum caminho de truncamento silencioso possível.

### P4 — Integração no pipeline (o "ligar a chave")

1. Reset em `auli-core/src/manifest.rs` (edição atômica, 1 ocorrência):
   `pub const STRATEGY_VERSION: u32 = 4;` → `= 1;`. O doc-comment é REESCRITO:
   abre registrando o reset — "numeração reiniciada em jul/2026
   (TAREFA-FAQ-PR): a key de `faqs` passou a incluir a resposta (D-FAQPR-1),
   junto com rescrape completo; nenhum pack da numeração anterior sobrevive,
   pré-condição do reset" — e a narrativa v2–v4 antiga é preservada abaixo,
   condensada, como **nota histórica da numeração aposentada** (continua sendo
   o registro canônico do formato; só deixa de ser a linha viva).
2. **Backup primeiro** (verificação inicial 3), depois `auli-collections rs
   process` (o `rs-faqs.json` muda — o campo `text_to_embed` agora carrega a
   resposta; o golden é o próprio `raw/`, se auto-atualiza) e `auli update` no
   RS.

Aceite: update roda limpo; contagem de vetores == contagem de itens; manifesto
com `strategy_version: 1` escrito; boot com o pack novo passa; boot apontado
para o backup v4 é RECUSADO — 4 ≠ 1, a recusa é o comportamento correto, prova
de que o carimbo fecha a porta.

### P5 — A/B de retrieval (gate de promoção)

Insumo do Carlos: ~10 consultas reais estilo contribuinte em
`data/eval/faq_pr_consultas.txt`. **Formato:** `consulta` sozinha na linha, ou
`consulta<TAB>fragmento-esperado` — o fragmento (substring da resposta do item
alvo) é o que permite hit@1/hit@5 automático; linha sem fragmento entra só na
impressão lado a lado para julgamento manual.

Harness: teste `#[ignore]` (ou exemplo) que carrega os DOIS packs direto pelo
`vector-store` (leitura crua, SEM passar pela validação de manifesto do boot —
o pack v4 de backup seria recusado por ela, e aqui a comparação é o objetivo),
embeda cada consulta com o mesmo embedder e imprime top-5 de cada pack +
resumo hit@1/hit@5 das linhas com fragmento.

Aceite (gate): novo ≥ antigo no conjunto. Resultado colado nesta TAREFA antes
de promover. Empatou ou piorou ⇒ parar e discutir (candidatos: ajuste da
normalização ou do cabeçalho de assunto). **A TAREFA-FALLBACKS só entra na fila
depois deste gate aprovado.**

### P6 — Promoção e limpeza

1. Promover: re-`process` + re-`update` de TODAS as entidades (D-FAQPR-7 — o
   boot novo (estratégia 1) recusa qualquer pack v4, inclusive de entidades sem
   faqs; enquanto os fallbacks JSON existirem, entidade ainda sem árvore cai
   neles com aviso).
2. Verificação barata da pré-condição do reset (um grep, executado pelo
   Carlos): nenhum `*.manifest.json` fora de `data/packs/` ativo — em backups
   ou cópias — pode carregar `"strategy_version": 1` de era antiga. Declarado
   inexistente; o grep só torna o declarado verificado.
3. Backup do pack v4 mantido por um ciclo de update. **Rollback corrigido
   (D-FAQPR-7): restaurar o pack NÃO basta — o binário novo (estratégia 1)
   recusa manifesto v4 no boot. Rollback real = `git revert` do commit (o
   binário volta à constante 4) + restaurar o backup + reiniciar.** Só trocar
   diretório de pack era o plano da spec original e não funciona com a mudança
   de versão.
4. Registrar a mudança onde o projeto documenta o formato do pack: o
   doc-comment reescrito do `STRATEGY_VERSION` (reset + nota histórica da
   numeração aposentada) é o registro canônico; replicar a nota em
   `auli_operations.md` se ele descrever o embed.

Aceite: servidor rodando com packs da estratégia 1 em todas as entidades;
smoke manual no
chat com 2–3 perguntas de FAQ conhecidas; e as consultas do P5 cuja RESPOSTA
contém os termos que a pergunta não tem — o caso que esta TAREFA existe para
resolver — respondendo melhor que antes.

## Riscos e observações

- Re-embed completo das coleções de FAQ (e re-update geral pela mudança de
  versão). Barato:
  as medições da TAREFA-TEMPOS deram tempo de embed muito baixo na CPU.
- Caches/hashes derivados do texto de embed invalidados — comportamento
  correto, não bug.
- O guardrail P3 mora no update (não no produtor): o `rs-faqs.json` de `raw/` é
  artefato de frontend e pode carregar o que for; quem não pode estourar é o
  que vai ao embedder.
