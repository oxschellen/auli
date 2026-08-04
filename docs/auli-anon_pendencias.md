# auli-anon — estado e pendências

Estado atual do crate `auli-anon` e do que falta. Complementa o plano em
[`docs/IMPLEMENTACAO_auli-anon.md`](docs/IMPLEMENTACAO_auli-anon.md).

## O que já está pronto (Fases 0–3)

- **Fase 0/1** (PR #54): crate leaf `auli-anon` + **9** reconhecedores customizados.
  Cobertos **12 tipos de identificador estruturado**: CPF, CNPJ numérico, e-mail (nativos do
  cloakrs) + CNPJ alfanumérico, telefone, IE (RS), protocolo, GA/GNRE, RENAVAM, placa, CEP,
  data de nascimento. Recall 100% sobre os estruturados, 0 falsos positivos no controle.
  *(Corrigido em 2026-08-04: dizia "8 reconhecedores" e "14 identificadores" — a contagem não
  fechava com a própria lista da frase nem com `reconhecedores/mod.rs`, que exporta 9.)*
- **Fase 2** (PR #55): anonimização no **stdout** (o `info!` publica só a pergunta anonimizada) e
  omissão de IP. **⚠️ Corrigido em 2026-07-25:** esta linha dizia "anonimização no log de auditoria
  (`./logs`) e no stdout" — o log em disco **nunca foi anonimizado**. Ele grava a pergunta crua em
  `PERGUNTA (ORIGINAL)` e a resposta já restaurada em `RESPOSTA`, divergindo do §4 do plano, que
  marcava a persistência anonimizada como *obrigatória*. Auditoria de 118 arquivos: 3 com PII do
  usuário na pergunta, 1 na resposta (os 26 do `CONTEXTO RAG` são dado institucional de documento
  público, outra categoria). **Requisito revisto, não corrigido:** o log é deliberadamente íntegro,
  porque sem o par original/anonimizada lado a lado não há como auditar o que o anonimizador
  deixou passar. A proteção passou a ser de acesso e retenção — ver `format_log_record` em
  [rag.rs](auli-server/crates/auli-cli/src/rag.rs) e `docs/auli_operations.md` §7.
- **Fase 3** (PR #56): anonimização na fronteira do LLM (sanitize→restore), atrás do flag
  `AULI_ANONIMIZAR_LLM` (default on).
- **Fase 4** (2026-08-04): as três entidades sem forma canônica — razão social, endereço e nome
  de pessoa — por **heurística determinística**, sem dicionário de prenomes e sem NER. Detalhe
  abaixo.

## Fase 4 — entidades não estruturadas ✅ **entregue (2026-08-04)**

As três entidades que vaziam por não terem forma canônica passaram a ser cobertas por
heurística determinística. Elas continuam marcadas `Classe::NomeRazaoEndereco` nas fixtures,
mas agora com `coberto: true` e sob trava própria:

| Fixture | Entidade | Reconhecedor | Placeholder | Porteiro |
|---|---|---|---|---|
| 15 | Razão social | `auli_razao_social_v1` | `[RAZAO_SOCIAL_n]` | sufixo societário |
| 17 | Endereço | `auli_endereco_v1` | `[ENDERECO_n]` | logradouro **e** número |
| 14 | Nome de pessoa | `auli_nome_pessoa_v1` | `[NOME_n]` | gatilho de contexto |

Todos com confiança **0.7** — o menor nível do crate, o mesmo do RENAVAM com DV inválido mas
contexto explícito. São heurísticas, não validações por DV, e a confiança diz isso.

**O que continua fora (recall parcial assumido).** A regra da fase foi *precisão vence recall*,
e o preço está declarado:

- **nome solto**, sem gatilho antes (`João da Silva Pereira solicitou a baixa`);
- **nome de um token só** (`Sr. João`) — o mínimo é 2 tokens (D-ANON-4.5);
- **gatilho distante** do nome (`o contribuinte, que já esteve aqui, João Silva`);
- **razão social sem sufixo** societário e **endereço sem número**;
- qualquer candidato com token da **stoplist institucional** — inclusive um legítimo, como uma
  `Transportadora Rio Grande Ltda` real, que não será mascarada porque `Rio`/`Grande` estão lá.

Esses casos são o insumo da fase seguinte, de pesquisa (dicionário de prenomes BR e NER pt-BR),
que segue **fora de escopo**.

**Falso positivo residual conhecido:** `Estrada de Ferro 2` casa como endereço. Documentado e
aceito — texto tributário raramente traz a construção, e apertar o padrão custaria endereço real.

---

### 4.0 O que o cloakrs nativo NÃO resolve (verificado no fonte 0.3.0, 2026-08-04)

Antes de escrever qualquer reconhecedor da Fase 4: o registry universal (`cloakrs_patterns`)
**já registra** `PersonNameRecognizer`, `PhysicalAddressRecognizer` e `DateOfBirthRecognizer`, e
os três **estão ativos no nosso scanner** — `supported_locales()` devolve `&[]`, e
`supports_locale` trata slice vazio como universal, então o `Locale::BR` não os filtra. Eles
rodam a cada pergunta e **nunca disparam em pt-BR**:

| Reconhecedor nativo | Por que é inerte aqui |
|---|---|
| `person_name_dictionary_v1` | `validate_parts` exige que **prenome E sobrenome** estejam em duas listas EN `const` privadas (`FIRST_NAMES`/`LAST_NAMES`, ~50 nomes cada, sem nome brasileiro). Gate duro, não boost. |
| `physical_address_us_v1` | Padrão US: **número antes** do logradouro + sufixo EN (`St`, `Ave`, `Blvd`…). `Av. Mauá, 1155` é a ordem inversa e sem sufixo EN. |
| `date_of_birth` | O padrão `\d{1,2}[/-]\d{1,2}[/-]\d{4}` casa, mas os termos de contexto são EN (`dob`, `born`, `date of birth`) — daí o nosso `DataNascimentoRecognizer`. |

Duas consequências para o plano abaixo:

- **A opção (a) do §4.3 está fechada.** Não há "configurar/ativar" o `PersonName` do cloakrs: as
  listas são `const` privadas do crate. As saídas reais são reconhecedor próprio, `deny_list` do
  `ScannerBuilder` (literais conhecidos) ou NER.
- **Reutilizar o `EntityType` nativo é possível sem reutilizar o reconhecedor** — `PersonName` →
  `[PERSON_n]`, `PhysicalAddress` → `[ADDRESS_n]` —, como o `CnpjAlfanumericoRecognizer` já faz
  com `EntityType::Cnpj`. Um `EntityType::Custom("nome")` daria `[NOME_n]` (o `redaction_tag` de
  `Custom` é `upper_snake` do valor). Decidir por consistência de idioma dos placeholders.

Knobs do `ScannerBuilder` ainda não usados e relevantes para a Fase 4: `min_confidence`
(hoje `Confidence::ZERO` — tudo que os reconhecedores devolvem é mascarado), `allow_list`
(literais que nunca mascaram — válvula para falso positivo de nome/endereço) e `deny_list`.

### 4.1 Razão social — `RazaoSocialRecognizer` ✅

Porteiro único: o **sufixo societário** (`Ltda`, `S.A.`, `S/A`, `EIRELI`, `EPP`, `ME`, `MEI`,
`Cia`, `Companhia`), com janela de 1+4 tokens antes. Duas travas **não previstas no plano**,
ambas motivadas por falso positivo medido:

- **Titlecase para sufixo fraco** (`ME`/`SA`/`EPP`/`MEI`): os 2 tokens anteriores precisam ter
  minúscula depois da inicial. Sem isso, `PARCELAMENTO ICMS ME` e `BAIXA DA IE ME` viravam razão
  social — a contagem de tokens capitalizados do plano não distingue sigla em caixa alta.
- **Stoplist institucional**: sem ela, `Simples Nacional ME` (Titlecase legítimo) passava.

Detalhe de regex que custou um teste: a guarda final **consome um caractere não alfanumérico** em
vez de usar `\b`. Com `\b`, sufixo terminado em ponto (`S.A.`, `Ltda.`) perdia o ponto, porque
não há fronteira de palavra entre `.` e o espaço seguinte e o motor desiste do `\.?` opcional.

### 4.2 Endereço — `EnderecoRecognizer` ✅

Dois porteiros obrigatórios: palavra de logradouro no início **e** número no fim; captura até o
número, inclusive (D-ANON-4.4 — bairro/cidade/UF ficam de fora). Entre o nome e o número exige-se
separador real, e é isso que mantém `Rodovia BR-116 km 23` fora: ali o dígito está colado dentro
do token.

Correção sobre o padrão do plano: o **conector é aceito também antes do 1º token**. Sem isso,
`Travessa dos Cataventos, nº 22` e `Rua dos Andradas, 736` — a forma mais comum de endereço
brasileiro — não casavam.

### 4.3 Nome de pessoa — `NomePessoaRecognizer` ✅ (caminho 1)

Implementado o **caminho 1** (heurística ancorada em gatilho), como o plano recomendava. Sem
gatilho não há match. O span emitido é **só o nome**: o gatilho nunca é mascarado, senão a
pergunta perderia o sentido (`o [NOME_1] solicitou` em vez de `o contribuinte [NOME_1] solicitou`).

Duas correções sobre o padrão do plano: `\b` antes do grupo de gatilhos (sem ele, `nome` casava
dentro de *sobrenome*) e conector opcional entre gatilho e nome (sem ele, `em nome de Maria
Eduarda dos Santos` escapava).

Os caminhos **2 (dicionário de prenomes BR)** e **3 (NER pt-BR via ONNX)** seguem abertos, agora
como fase de pesquisa e não mais como alternativa: a §4.0 fechou a opção de reaproveitar o
`PersonNameRecognizer` do cloakrs.

### 4.4 Base compartilhada — `reconhecedores/comum.rs`

Os nove reconhecedores das Fases 0/1 duplicam o próprio `limites_ok` porque cada um tem uma
variação da regra de "separador colado a dígito", que protege CNPJ/protocolo de serem fatiados.
Os três da Fase 4 casam **texto**: a regra não se aplica, e a fronteira precisa enxergar acento
(`char::is_alphanumeric`, não `is_ascii_alphanumeric` — `'ã'` não é ASCII alfanumérico, e um match
colado ao fim de palavra acentuada passaria). Daí um módulo, em vez de uma quarta cópia. Ele
hospeda também a `STOPLIST` e os helpers `titlecase`/`comeca_maiuscula`.

---

## Como plugar um reconhecedor novo (padrão já estabelecido)

1. `auli-server/crates/auli-anon/src/reconhecedores/<nome>.rs` — `struct` implementando
   `cloakrs_core::Recognizer` (`id`/`entity_type`/`supported_locales`/`scan`/`validate`),
   com os mesmos helpers `limites_ok` (boundaries de byte) e janela de contexto ~40 chars
   dos reconhecedores existentes. Regex compilada uma vez (no `novo()` ou `LazyLock`).
   Se o reconhecedor casa **texto** e não dígito, usar `comum::limites_ok` (§4.4) em vez de
   escrever a décima cópia.
2. Registrar em `reconhecedores/mod.rs` (`mod` + `pub use`) e em `lib.rs`
   `Anonimizador::novo()` (`.recognizer(...)`).
3. Testes unitários no módulo: positivos, negativos (sem gatilho), boundaries.
4. Fixtures (`tests/fixtures.rs`): virar `coberto: true` a fixture correspondente.
5. Trava de aceite correspondente: `recall_estruturado_fase1` (100%) para identificador
   estruturado, `recall_nao_estruturado_fase4` (recall parcial documentado) para entidade sem
   forma canônica. Se o reconhecedor novo puder gerar falso positivo em texto institucional,
   acrescentar a frase típica a `CONTROLES_FASE4`.
6. `cargo test -p auli-anon` e `cargo clippy -p auli-anon -- -D warnings` verdes.
   Lembrete: converter `map_or(true/false, …)` → `is_none_or`/`is_some_and` (o gate clippy
   `-D warnings` do workspace reprova a forma antiga).

## Critérios de aceite da Fase 4 — ✅ atendidos (2026-08-04)

- ✅ Fixtures 14/15/17 mascaradas, sob `recall_nao_estruturado_fase4`; o que a heurística não
  pega está listado acima.
- ✅ **0 falsos positivos**: fixture de controle (20) intacta e 10 frases de referência típicas
  em `CONTROLES_FASE4`, todas com zero detecção. Cada frase corresponde a um falso positivo que
  existiu durante a implementação.
- ✅ Sem regressão: `recall_estruturado_fase1` e `regressao_coberto` intocadas e verdes.
- ✅ Span da entidade apenas — `span_exclui_o_gatilho` trava que o gatilho de nome não é mascarado.
- ✅ Ciclo `anonimizar`→`restaurar` com os placeholders novos (`restore_fase4`); Fase 3 e
  `AULI_ANONIMIZAR_LLM` inalteradas.
- ✅ Sobreposição nome × razão social travada (`sobreposicao_nome_dentro_de_razao_social`) — a
  regra é do `deduplicate` do cloakrs, e o teste existe para um bump do crate não a mudar calado.
- ✅ **Latência**: mediana **0,34 ms** (p95 0,35 ms) para pergunta de 1,2 KB densa em PII, contra
  o teto de 5 ms do plano §5.2. Construção do `Anonimizador`, uma vez no boot: 3,3 ms.

## Outras pendências (fora da Fase 4)

- ~~**Aba "Sobre" (frontend).** Não prometer "dados 100% anônimos".~~ ✅ **Resolvida
  (2026-08-02, PR #116, commit `2a9a6b2`).** [about.md](auli-frontend/public/about.md) hoje diz
  o que o sistema faz: a pergunta é guardada como foi escrita (*"dados pessoais que você mesmo
  digitar ficam gravados"*) e o mascaramento é descrito como fronteira de saída — *"antes de a
  pergunta ser enviada ao modelo de linguagem … identificadores como CPF, CNPJ, telefone e
  e-mail são substituídos automaticamente por marcadores"*, com a ressalva do MCP (nenhum
  modelo externo é chamado). Alinhado com a §7.0 do `auli_operations.md` e com a §29 do
  `auli_pendencias.md`.
- **`cloakrs` é jovem (v0.3.0, pinado `=`).** Se abandonado, o custo de troca é baixo (toda
  a lógica de domínio está nos nossos reconhecedores); só usamos `PromptSanitizer`/`Scanner`/
  `Recognizer`. Reavaliar em cada bump.
- **Árvore de deps de cripto** (`aes-gcm`/`sha2`/`aead`) vem via o masker do cloakrs, que
  **não usamos**. Se o cloakrs modularizar features no futuro, desabilitar o masker enxuga a
  árvore.
- **IE por UF.** `InscricaoEstadualRecognizer` já tem tabela por UF com só o RS. Ao indexar
  SP/SC (formatos e DVs próprios), adicionar as entradas — o esqueleto está pronto.
