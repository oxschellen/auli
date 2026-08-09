# TAREFA-CONFIG — `config/` na raiz: o ponto de extensão ganha endereço próprio

> **CONCLUÍDA em 08/08/2026** — PR #135.
>
> **Onde a implementação divergiu do plano, e por quê:**
>
> - **C1 e C2 num commit só.** O `git mv` e a fiação dos consumidores são atômicos: entre um e
>   outro o repo não roda. Separá-los produziria um commit quebrado, que é pior que uma fase a
>   menos.
> - **C3 partida ao meio.** O cabeçalho do registry (D-CFG-5) foi com o commit do movimento — é
>   parte do arquivo movido. A placa do README (D-CFG-4) ficou em commit próprio, porque é a parte
>   que se lê.
> - **A guarda erra mesmo se os DOIS existirem.** A D-CFG-6 dizia "se o caminho velho existir"; o
>   critério de aceite dizia "se só o layout antigo existir". Ficou a leitura estrita: ler o novo e
>   ignorar o velho em silêncio é o modo de falha caro — alguém edita o arquivo errado e passa uma
>   tarde sem entender por que a mudança não pega.
> - **`AULI_CONFIG_DIR` vazia conta como ausente**, o que o plano não especificava. Um
>   `export AULI_CONFIG_DIR=` num script não pode mandar o servidor procurar o catálogo na raiz do
>   sistema de arquivos.
> - **Fora do plano, e a migração expôs:** `sinopse.rs` e `extracao.rs` subiam dois níveis do
>   `data_dir` da entidade para achar o prompt do passo, com um comentário em um deles dizendo
>   "mesmo cálculo de caminho do sinopse". Virou `EntityConfig::prompt_de_passo`, num lugar só.

**Motivação (Carlos):** o Auli é open-source; quem lê o repo querendo adaptá-lo para outra
organização ("outras tabelas", outra secretaria) precisa achar o ponto de extensão sem cavar.
O registry é **catálogo/configuração**, não dado — morar em `data/` comunicava o contrário.

**Escopo:** mover `data/registry.toml` e `data/prompts/` para `config/` na raiz do repo,
ajustar todos os consumidores, e instalar a "placa" de adoção no README. Nada de
comportamento muda — é mudança de endereço com melhoria de documentação.

**Fora do escopo:** qualquer mudança no CONTEÚDO do registry ou dos prompts; a seção
`[[colecoes]]` dirigida por registry (isso é pós-TAREFA-DOCUMENTO, item 4 do
ESTUDO-pos-documento); os dados vivos (`data/<id>/...` não se move).

---

## Decisões

- **D-CFG-1 (o que muda de lugar):** `git mv data/registry.toml config/registry.toml` e
  `git mv data/prompts config/prompts` — um commit, história preservada. Os caminhos
  INTERNOS do registry (`prompt = "prompts/rs.txt"`) permanecem intocados: eram relativos à
  pasta do registry e continuam sendo.
- **D-CFG-2 (resolução de caminho, por convenção):** `config/` é resolvida como IRMÃ da
  raiz de dados — `<AULI_DATA_DIR>/../config` por default — com a env var opcional
  `AULI_CONFIG_DIR` para sobrepor (mesma família do AULI_DATA_DIR; obrigatória ausente NÃO
  é: tem default). Um helper único no lugar onde o registry é carregado hoje; proibido
  espalhar o cálculo do caminho.
- **D-CFG-3 (consumidores, a varredura completa):** ajustar TODOS os leitores:
  (a) servidor/auli-cli — carga do registry e dos prompts de entidade;
  (b) auli-collections — validação de entidades pelo registry, `data/prompts/sinopse.txt` e
  prompts de extração passam a `config/prompts/...`;
  (c) `scripts/gen-frontend-entities.mjs` e `scripts/check-registry-sync.sh` — novo caminho;
  (d) `start_server.sh`, `scripts/build-packs.sh` e demais scripts que referenciem
  `data/registry.toml` ou `data/prompts` (fazer `grep -rn "data/registry\|data/prompts"` na
  raiz e zerar as ocorrências — inclusive em docs e no `.env.example`);
  (e) CLAUDE.md e docs que descrevem o layout.
- **D-CFG-4 (a placa open-source):** seção nova no README — "Adaptando o Auli para a sua
  organização": comece por `config/registry.toml` (entidade nova = uma linha + pasta
  `data/<id>/` + prompt, zero recompilação); e a frase honesta sobre tabelas: "coleção nova
  hoje exige código — roteiro em docs/ESTUDO-colecoes-trait.md; a família jurisprudência
  ficará configurável pós-TAREFA-DOCUMENTO". A mesma orientação, resumida, no cabeçalho do
  registry (que já é didático — manter o estilo).
- **D-CFG-5 (registro conceitual):** o cabeçalho do registry ganha a frase da discussão:
  "Este arquivo é CATÁLOGO, não dado: o ponto onde o vocabulário do código encontra o
  inventário de `data/`. Mora em `config/` para o adotante achá-lo."
- **D-CFG-6 (deploy e compatibilidade):** o Desktop Server é o único ambiente de execução —
  atualizar o `.env`/lançadores locais na mesma janela do deploy do binário; nenhum shim de
  compatibilidade lendo do caminho antigo (ambiente único, migração atômica; se o caminho
  velho existir, o boot deve FALHAR com mensagem apontando o novo — nunca ler dele em
  silêncio).

## Fases

**C1** — Helper de resolução + carga do servidor e do auli-collections (D-CFG-2/3a/3b), com
teste: registry ausente em `config/` ⇒ erro claro citando o caminho esperado; presença de
`data/registry.toml` legado ⇒ erro orientando a migração (D-CFG-6).
**C2** — `git mv` (D-CFG-1) + scripts e varredura grep-zero (D-CFG-3c/d/e).
**C3** — README + cabeçalho do registry (D-CFG-4/5).
**C4** — Verificação (Carlos): subir o servidor local, uma consulta por coleção no rs,
`check-registry-sync.sh` verde, `gen-frontend-entities.mjs` regenerado sem diff no
`entities.ts`.

## Critérios de aceite

- `grep -rn "data/registry\|data/prompts"` na raiz: zero ocorrências em código, scripts,
  .env.example e docs vivos (menções históricas em TAREFAs antigas de `docs/` podem ficar).
- Servidor sobe e serve normalmente com `config/` no lugar; boot FALHA com mensagem clara se
  só o layout antigo existir.
- `cargo test` e `clippy -D warnings` verdes; frontend `entities.ts` idêntico ao anterior.
- README com a seção de adoção; registry com o cabeçalho novo.
