# TAREFA — arquivar o log antes do bump, e tornar o log autodescritivo

**Contexto:** o teto do `text_to_embed` (N = 6.000) vai subir o `STRATEGY_VERSION` de 2 para 3. A
partir daí as distâncias gravadas no log de auditoria passam a se referir a **outro espaço vetorial**,
e misturar as duas eras corromperia qualquer calibragem futura.

São duas coisas independentes: **arquivar** (agora, antes do re-embed) e **carimbar** (para nunca mais
precisar arquivar).

---

## 1 — Arquivar, não apagar

Mova o log atual (107 arquivos desde 02/08) para **`data/eval/log-strategy-2/`** — o número é o
`STRATEGY_VERSION` que produziu aqueles registros, então a próxima pasta se nomeia sozinha.

**Por que mover e não zerar:** o log tem duas coisas de valores muito diferentes. As **distâncias**
envelhecem com o bump e são re-deriváveis (basta rodar a pergunta de novo). As **perguntas** são
perguntas reais de analista, acumuladas em dez dias de uso, e **não são reproduzíveis**. Elas são
exatamente o conjunto de teste para comparar N = 6.000 contra 4.000 ou 8.000: rodar as mesmas
perguntas contra cada pack e ver o que sobe e o que desce. Com o log zerado, esse A/B só existiria
depois de mais dez dias de uso; arquivado, existe no dia seguinte ao re-embed.

**Já verifiquei o `.gitignore`, e está seguro:** a regra `/data/*` (linha 76) cobre `data/eval/`, com
exceção apenas para `registry.toml` e `prompts/`. Confirmado por `git check-ignore -v data/eval/`.
Nada do log entra no repositório — o que importa porque esses arquivos contêm o par
original/anonimizada, isto é, a versão **não mascarada** de perguntas reais sobre casos de
contribuintes, num repositório público.

**Ainda assim, confirme antes do `mv`** (`git check-ignore -v data/eval/log-strategy-2/`) e reporte o
resultado. Estou lendo um clone; se a sua árvore divergir, é melhor descobrir antes de mover 107
arquivos do que depois.

Se o `data/eval/` já tiver conteúdo curado (por exemplo `faq_pr_consultas.txt`, do gate P5), **não o
toque** — o arquivamento cria uma subpasta ao lado, não reorganiza a pasta.

---

## 2 — Carimbar o log com a identidade do espaço vetorial

**O problema que isto resolve:** hoje, a cada mudança de fórmula, alguém precisa **lembrar** de
arquivar o log — e esquecer não dá erro, dá análise silenciosamente errada, misturando distâncias de
dois espaços. É a mesma classe de defeito que o `docs_hash` no boot existe para impedir, sem a
proteção equivalente.

**O que fazer:** gravar em cada arquivo de log o `strategy_version` — ou, melhor, o trio de identidade
inteiro (modelo, dimensão, `strategy_version`), que é o que o manifesto já usa para decidir se um pack
e um servidor combinam. O log passa a ser **autodescritivo**: separar eras vira **filtro**, não
mudança de pasta, e nenhuma análise futura mistura por engano.

Detalhes:

- campo novo no DTO que já é escrito, com `serde(default)` — logs antigos (sem o campo) continuam
  desserializando, e a ausência **significa** "anterior ao carimbo", que é informação verdadeira;
- **não** reescreva logs existentes para acrescentar o campo. Eles vão para
  `data/eval/log-strategy-2/`, e a pasta já é o carimbo deles;
- se houver uma fonte única do trio de identidade em `auli-core` (o manifesto a usa), reaproveite-a em
  vez de reescrever a montagem — três formulações de uma identidade é o tipo de coisa que só aparece
  quando duas divergem;
- um teste: o log escrito carrega o `strategy_version` corrente, e um log sem o campo desserializa
  sem erro.

**Cuidado com a rota de leitura:** a rota `GET /v1/log/{uuid}` serve o conteúdo íntegro ao modal. O
campo novo aparecerá lá. Isso é aceitável — não é dado sensível —, mas confira se o modal quebra com
um campo desconhecido no DTO do frontend, e reporte antes de mexer no frontend. Se quebrar, é
conversa antes de código.

---

## 3 — Ordem e escopo

Isto vem **antes** do re-embed da TAREFA do teto, e é independente dela: pode virar commit próprio,
sem esperar a Fase 1. O item 1 é um `mv`; o item 2 é um campo.

**Não faça:** limpeza, rotação ou poda do log; mudança no formato dos arquivos existentes; nada no
frontend sem reportar antes.

## 4 — Reportar

(a) o resultado do `git check-ignore` e quantos arquivos foram movidos; (b) o campo novo e o teste;
(c) se o modal aguenta o campo desconhecido; (d) qualquer suposição minha que não bateu — inclusive
sobre a estrutura do `data/eval/`, que estou inferindo de um clone.
