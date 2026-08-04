# auli-anon — pendências (Fase 4 e afins)

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

## O que falta: Fase 4 — entidades não estruturadas

Três entidades **ainda vazam** por não terem forma canônica (exigem NER ou heurística).
Nas fixtures elas estão marcadas `Classe::NomeRazaoEndereco` e ficam **fora** da trava
`recall_estruturado_fase1`:

| Fixture | Categoria | Segredo que vaza | Dificuldade |
|---|---|---|---|
| 15 | Razão social | `Anderle Transportes` | média |
| 17 | Endereço | `Av. Mauá, 1155` | média–alta |
| 14 | Nome de pessoa (pt-BR) | `João da Silva Pereira` | alta |

Ordem de implementação sugerida: **razão social → endereço → nome** (do mais tratável ao
mais ambíguo).

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

### 4.1 Razão social — `RazaoSocialRecognizer` (heurística de sufixo societário)

**A mais tratável.** Razão social quase sempre termina num sufixo societário canônico.

- **Padrão:** sequência capitalizada (com conectores `e`/`&`/`da`/`de`/`do`) seguida de
  sufixo: `Ltda`, `S/A`, `S.A.`, `SA`, `EIRELI`, `ME`, `EPP`, `MEI`, `Cia`, `Companhia`.
  Capturar do início da sequência capitalizada até o sufixo (inclusive).
- **Anti-falso-positivo:** exigir o sufixo (é o porteiro); o sufixo sozinho é inequívoco,
  não precisa de contexto. Cuidar de `ME`/`SA` isolados (siglas) — só casar quando
  precedidos de uma sequência capitalizada plausível.
- **Placeholder:** `EntityType::Custom("razao_social")` → `[RAZAO_SOCIAL_n]`.
- **Fixture 15** deve mascarar `Anderle Transportes Ltda` (ou ao menos `Anderle Transportes`).
- **Risco:** nomes de órgãos/produtos capitalizados sem sufixo NÃO casam (bom); o risco real
  é capturar demais (uma frase capitalizada longa antes do `Ltda`) — limitar a janela a
  ~5 tokens antes do sufixo.

### 4.2 Endereço — `EnderecoRecognizer` (heurística de logradouro)

- **Padrão:** palavra de logradouro (`Rua`, `Av\.?`, `Avenida`, `Travessa`, `Praça`,
  `Rodovia`, `Estrada`, `Alameda`, `Largo`) + nome do logradouro (capitalizado) + número.
  Ex.: `Av. Mauá, 1155`.
- **Escopo:** capturar até o número (o `, 1155`); bairro/cidade que seguem são mais ambíguos
  — decidir se entram (a fixture 17 espera só `Av. Mauá, 1155`).
- **Placeholder:** `EntityType::Custom("endereco")` → `[ENDERECO_n]`, ou reutilizar o nativo
  `EntityType::PhysicalAddress` (placeholder `[ADDRESS]`) — decidir por consistência PT.
- **Fixture 17** deve mascarar `Av. Mauá, 1155`.
- **Risco:** "Rodovia BR-116 km 23" e nomes de logradouro em texto de referência; a palavra
  de logradouro como porteiro reduz bastante o ruído.

### 4.3 Nome de pessoa — `NomePessoaRecognizer` (a mais difícil)

Sem forma canônica; qualquer sequência capitalizada pode ser nome. Três caminhos, do mais
leve ao mais pesado:

1. **Heurística ancorada em contexto (recomendado começar por aqui).** Só capturar sequência
   capitalizada (2–4 tokens, conectores `da`/`de`/`do`/`dos`/`das`/`e`) **após** um gatilho:
   `Sr\.?`, `Sra\.?`, `contribuinte`, `requerente`, `produtor(a)? rural`, `sócio`, `nome`,
   `titular`, `responsável`, `contador(a)?`. Recall parcial, mas falso-positivo baixo.
2. **Dicionário de prenomes BR** (lista de nomes comuns) + heurística de sobrenome
   (conectores + capitalização). Melhora recall sem gatilho, ao custo de manter a lista.
3. **NER leve (ONNX)**. cloakrs tem `EntityType::PersonName`, mas o `PersonNameRecognizer` que
   o acompanha é dicionário EN com gate duro e **não pega** nomes pt-BR (a fixture 14 vaza
   hoje) — ver §4.0: não há o que configurar, a opção sobrou como modelo NER pt-BR via ONNX.
   **Custo alto** — reintroduz um modelo/processo pesado ao lado do BGE-M3. Só se (1) e (2)
   forem insuficientes.

- **Placeholder:** `EntityType::Custom("nome")` → `[NOME_n]` (ou o nativo `PersonName` →
  `[PERSON]`).
- **Fixture 14** deve mascarar `João da Silva Pereira`.
- **Precedência:** cuidar para não colidir com razão social (um nome pode preceder `Ltda`);
  o `deduplicate` do cloakrs resolve sobreposição pelo span mais longo.

---

## Como plugar um reconhecedor novo (padrão já estabelecido)

1. `auli-server/crates/auli-anon/src/reconhecedores/<nome>.rs` — `struct` implementando
   `cloakrs_core::Recognizer` (`id`/`entity_type`/`supported_locales`/`scan`/`validate`),
   com os mesmos helpers `limites_ok` (boundaries de byte) e janela de contexto ~40 chars
   dos reconhecedores existentes. Regex compilada uma vez (no `novo()` ou `LazyLock`).
2. Registrar em `reconhecedores/mod.rs` (`mod` + `pub use`) e em `lib.rs`
   `Anonimizador::novo()` (`.recognizer(...)`).
3. Testes unitários no módulo: positivos, negativos (sem gatilho), boundaries.
4. Fixtures (`tests/fixtures.rs`): virar `coberto: true` a fixture correspondente.
5. **Nova trava de aceite** para a Fase 4 (as fixtures 14/15/17 são `NomeRazaoEndereco`,
   hoje fora do `recall_estruturado_fase1`): criar `recall_nao_estruturado_fase4` (ou
   reclassificar) exigindo recall sobre nome/razão/endereço, aceitando **recall parcial**
   documentado — diferente da meta de 100% dos estruturados.
6. `cargo test -p auli-anon` e `cargo clippy -p auli-anon -- -D warnings` verdes.
   Lembrete: converter `map_or(true/false, …)` → `is_none_or`/`is_some_and` (o gate clippy
   `-D warnings` do workspace reprova a forma antiga).

## Critérios de aceite da Fase 4

- Fixtures 14/15/17 mascaradas (recall parcial aceitável para nome; documentar o que não
  pega).
- **0 falsos positivos** na fixture de controle (20) e em texto de referência típico —
  nomes/endereços são a maior fonte de falso-positivo; o controle não pode regredir.
- Sem regressão nos 14 estruturados (a trava `recall_estruturado_fase1` continua verde).

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
