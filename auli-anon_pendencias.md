# auli-anon — pendências (Fase 4 e afins)

Estado atual do crate `auli-anon` e do que falta. Complementa o plano em
[`IMPLEMENTACAO_auli-anon.md`](IMPLEMENTACAO_auli-anon.md).

## O que já está pronto (Fases 0–3)

- **Fase 0/1** (PR #54): crate leaf `auli-anon` + 8 reconhecedores customizados.
  Cobertos **14 identificadores estruturados**: CPF, CNPJ numérico, e-mail (nativos do
  cloakrs) + CNPJ alfanumérico, telefone, IE (RS), protocolo, GA/GNRE, RENAVAM, placa, CEP,
  data de nascimento. Recall 100% sobre os estruturados, 0 falsos positivos no controle.
- **Fase 2** (PR #55): anonimização no log de auditoria (`./logs`) e no stdout, sem IP.
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
3. **NER leve (ONNX)**. cloakrs tem `EntityType::PersonName`, mas o registry BR default
   **não pega** nomes pt-BR (a fixture 14 vaza hoje). Avaliar: (a) configurar/ativar o
   recognizer de PersonName do cloakrs; (b) um modelo NER pt-BR via ONNX. **Custo alto** —
   reintroduz um modelo/processo pesado ao lado do BGE-M3. Só se (1) e (2) forem insuficientes.

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

- **Aba "Sobre" (frontend).** Não prometer "dados 100% anônimos". Texto sugerido:
  *"identificadores estruturados (CPF, CNPJ, telefone, e-mail, …) são mascarados
  automaticamente antes de sair do processo"*. Ver risco §7 do plano.
- **`cloakrs` é jovem (v0.3.0, pinado `=`).** Se abandonado, o custo de troca é baixo (toda
  a lógica de domínio está nos nossos reconhecedores); só usamos `PromptSanitizer`/`Scanner`/
  `Recognizer`. Reavaliar em cada bump.
- **Árvore de deps de cripto** (`aes-gcm`/`sha2`/`aead`) vem via o masker do cloakrs, que
  **não usamos**. Se o cloakrs modularizar features no futuro, desabilitar o masker enxuga a
  árvore.
- **IE por UF.** `InscricaoEstadualRecognizer` já tem tabela por UF com só o RS. Ao indexar
  SP/SC (formatos e DVs próprios), adicionar as entradas — o esqueleto está pronto.
