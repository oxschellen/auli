# Implementação: anonimização de perguntas do Chat (`auli-anon`)

**Projeto:** Auli — workspace Rust em `~/Desktop/auli_new` (monorepo `oxschellen/auli`)
**Objetivo:** anonimizar dados pessoais e identificadores presentes nas perguntas dos analistas antes que elas (a) sejam persistidas no log de pergunta/resposta e (b) saiam do processo rumo ao LLM externo.
**Estilo de trabalho:** implementar em etapas pequenas e incrementais, na ordem das fases abaixo. Concluir e validar cada fase antes de iniciar a próxima. Commits pequenos por fase.

---

## 1. Contexto e decisão técnica

A Auli registra apenas pergunta e resposta, sem rastreamento de usuário. Porém a pergunta em si frequentemente contém PII do contribuinte: CPF, CNPJ, IE, e-mail, telefone, protocolo, número de GA, RENAVAM, placa, endereço. Hoje esse conteúdo (1) fica gravado nos logs e (2) é enviado ao LLM externo dentro do prompt.

Foi feita uma avaliação empírica (harness `auli-anon-eval`, incluído como referência) comparando crates de anonimização em Rust. **Decisão: usar `cloakrs` (crates.io, v0.3.0) como framework base**, pelos motivos:

- Reconhecedores BR de CPF e CNPJ com **validação de dígito verificador mod-11** (não regex cega), boundary checks e boost de confiança por palavras de contexto. Zero falsos positivos no teste de controle.
- `PromptSanitizer` pronto com ciclo `sanitize → mapping → restore`: substitui PII por placeholders numerados (`[CNPJ_1]`) e restaura os valores originais na resposta do LLM. Testado ponta a ponta no harness.
- Trait `Recognizer` minimalista (`id`, `entity_type`, `supported_locales`, `scan`, `validate`) e `EntityType::Custom(String)` — extensível para as entidades do nosso domínio.
- Workspace modular com dependências unidirecionais, mesmo espírito do monorepo da Auli.

**Resultado da avaliação out of the box: recall 32% (7/22 segredos).** O cloakrs pega CPF, CNPJ e e-mail; vaza telefone BR, CNPJ alfanumérico (formato 2026), IE, protocolo, GA, RENAVAM, placa, CEP, data de nascimento, nomes e razão social. Portanto o grosso deste trabalho é **escrever reconhecedores customizados** para as entidades vazadas.

**Fora de escopo (por ora):** nome de pessoa e razão social. Exigem NER ou heurísticas dedicadas — ficam para a Fase 4, opcional. Documentar essa limitação no código e no README.

---

## 2. Arquitetura

### 2.1 Novo crate no workspace

Criar o crate `auli-anon` no workspace `auli`, respeitando o layering estrito existente:

```
vector-store  (genérico, sem domínio)
auli-anon     (novo — anonimização; depende só de cloakrs + cpf_cnpj + regex)
auli-core     (embed, corpus, manifest — passa a depender de auli-anon)
auli-cli      (binário: server, update)
```

`auli-anon` NÃO depende de `vector-store` nem de `auli-core`. Ele expõe uma API pequena e é consumido por `auli-core` (ou pelo handler do server em `auli-cli`, conforme onde hoje vive a montagem do prompt — inspecionar o código e escolher o ponto natural; documentar a escolha no PR).

### 2.2 Dependências (Cargo.toml de `auli-anon`)

```toml
[dependencies]
cloakrs-core    = "=0.3.0"   # pinar versão exata — crate jovem, API pode quebrar
cloakrs-locales = "=0.3.0"
cpf_cnpj        = "0.3"      # validação DV, inclusive CNPJ alfanumérico (IN RFB 2.229/2024)
regex           = "1"
once_cell       = "1"        # (ou std::sync::LazyLock se a MSRV do workspace permitir) para regexes compiladas uma vez
```

**Tarefa preliminar obrigatória:** verificar a licença do `cloakrs` (repo `kadir/cloakrs` no GitHub) antes de adicionar. Se não for MIT/Apache-2.0 compatível com o projeto (MIT), parar e reportar. Compilado e testado com rustc 1.75; confirmar compatibilidade com a toolchain do workspace.

### 2.3 API pública de `auli-anon`

```rust
pub struct Anonimizador { /* PromptSanitizer interno */ }

pub struct Anonimizado {
    /// Texto com placeholders, ex.: "CNPJ [CNPJ_1] pagou a GA [GA_1] em duplicidade"
    pub texto: String,
    /// Mapping para restore; NUNCA persistir este campo em disco
    pub mapping: cloakrs_core::PromptMapping,
}

impl Anonimizador {
    /// Constrói o scanner com locale BR + todos os reconhecedores customizados.
    pub fn novo() -> Result<Self, AnonError>;

    /// Anonimiza um texto. Determinístico, sem rede, in-process.
    pub fn anonimizar(&self, texto: &str) -> Result<Anonimizado, AnonError>;

    /// Restaura os placeholders de uma resposta usando o mapping da pergunta.
    pub fn restaurar(&self, texto: &str, mapping: &cloakrs_core::PromptMapping) -> String;
}
```

Regras:
- Construir o `Anonimizador` **uma vez** na inicialização do server e reutilizar (as regexes compilam no construtor).
- O `mapping` vive apenas em memória, no escopo da requisição. **Nunca** serializar/persistir o mapping — ele contém os valores originais.
- Em caso de erro interno do sanitizer, **fail closed**: logar o erro e persistir/enviar uma string fixa `"[ERRO DE ANONIMIZAÇÃO — pergunta descartada do log]"` em vez do texto cru. Nunca deixar o texto original passar por causa de uma falha.

---

## 3. Reconhecedores customizados (Fase 1 — o núcleo do trabalho)

Implementar cada um como `struct` própria implementando `cloakrs_core::Recognizer`, em módulos separados (`src/reconhecedores/telefone.rs`, etc.). Todos usam `EntityType::Custom("NOME".into())` exceto onde indicado. Todos devem respeitar boundary checks (não casar no meio de sequências maiores de dígitos) — seguir o padrão dos reconhecedores BR nativos do cloakrs (`is_boundary`, `has_separated_digit_before/after` em `cloakrs-locales/src/br_br.rs`; replicar helpers equivalentes localmente, pois são privados).

**Princípio geral anti-falso-positivo:** padrões numéricos ambíguos (sequências de dígitos sem formatação distintiva) só disparam com **palavra de contexto** a até ~30 caracteres antes do match (case-insensitive). Padrões com formatação inequívoca (e-mail, CEP com hífen, placa) dispensam contexto. O teste de controle (pergunta sem PII) deve permanecer com zero detecções.

Ordem de implementação (um commit por reconhecedor, com seus testes):

### 3.1 `TelefoneBrRecognizer` — `Custom("TELEFONE")`
- Padrão: `(?:\+55\s?)?\(?\d{2}\)?\s?9?\d{4}[-.\s]?\d{4}` com boundaries.
- Cobre: `(51) 99876-5432`, `(51) 3214-5678`, `51 99876 5432`, `+55 51 99876-5432`, `5199876543 2`? não — sem separadores exigir contexto (`telefone`, `celular`, `fone`, `whatsapp`, `contato`).
- Cuidado com colisão: um telefone sem formatação tem 10–11 dígitos, mesmo tamanho de CPF/RENAVAM. Regra de precedência: se o span já foi detectado como CPF (DV válido), CPF vence. Ver §3.9.

### 3.2 `CnpjAlfanumericoRecognizer` — reutilizar `EntityType::Cnpj`
- Padrão: `[0-9A-Z]{2}\.?[0-9A-Z]{3}\.?[0-9A-Z]{3}/?[0-9A-Z]{4}-?\d{2}` (case-insensitive, normalizar para maiúsculas na validação).
- `validate`: delegar para `cpf_cnpj::cnpj::validate`, que já suporta o formato alfanumérico da IN RFB nº 2.229/2024 (o DV alfanumérico usa valor ASCII − 48 nos pesos).
- Exigir que contenha ao menos uma letra OU os separadores `./-` OU contexto `CNPJ` — para não competir com o reconhecedor numérico nativo.

### 3.3 `CepRecognizer` — `Custom("CEP")`
- Com hífen (`\d{5}-\d{3}`): dispara sem contexto.
- Sem hífen (8 dígitos): só com contexto (`cep`, `endereço`, `código postal`).

### 3.4 `InscricaoEstadualRecognizer` — `Custom("IE")`
- RS: `\d{3}/?\d{7}` (formato `224/3210012` ou 10 dígitos corridos).
- Contexto obrigatório: `IE`, `inscrição estadual`, `insc. est`.
- Estruturar com tabela de padrões **por UF** (começar só com RS), pois cada estado tem formato próprio — preparar para SP/SC quando os estados forem indexados: `const PADROES_IE: &[(Uf, &str)]`.

### 3.5 `ProtocoloRecognizer` — `Custom("PROTOCOLO")`
- Padrão: `\d{4}/\d{6,12}` OU sequência de 9–15 dígitos com contexto.
- Contexto obrigatório: `protocolo`, `proc.`, `processo`.

### 3.6 `GaRecognizer` — `Custom("GA")`
- Sequência de 10–17 dígitos com contexto obrigatório: `GA`, `guia de arrecadação`, `GNRE`.
- Atenção: "GA 1118" é **código de receita**, não identificador — não anonimizar números de 3–4 dígitos após "GA"/"cod". Mínimo de 10 dígitos no match.

### 3.7 `RenavamRecognizer` — `Custom("RENAVAM")`
- 9–11 dígitos com contexto `renavam`.
- Implementar a validação de dígito verificador do RENAVAM (mod-11 sobre os 10 primeiros dígitos com pesos 3,2,9,8,7,6,5,4,3,2; DV = 11 − resto, com 10/11 → 0). Se o DV não bater mas o contexto for explícito (`renavam` imediatamente antes), anonimizar mesmo assim com confiança menor.

### 3.8 `PlacaRecognizer` — `Custom("PLACA")`
- Padrão antigo `[A-Z]{3}-?\d{4}` e Mercosul `[A-Z]{3}\d[A-Z]\d{2}` (case-insensitive).
- Mercosul sem contexto (formato é inequívoco); padrão antigo com contexto (`placa`, `veículo`, `carro`) para evitar colidir com siglas + números.

### 3.9 `DataNascimentoRecognizer` — `Custom("DATA_NASC")`
- `\d{2}/\d{2}/\d{4}` **somente** com contexto (`nasc`, `nascimento`, `nascida?o`, `data de nasc`). Datas genéricas são comuns em perguntas tributárias (vencimentos, fatos geradores) e NÃO devem ser anonimizadas.

### 3.10 Resolução de sobreposição
Verificar como o `Scanner` do cloakrs resolve spans sobrepostos (inspecionar `scanner.rs`). Se não houver resolução por confiança, implementar pós-processamento no `Anonimizador`: em spans sobrepostos, vence (1) o de maior confiança, (2) em empate, o mais longo. Casos-teste obrigatórios: CPF × telefone sem formatação; CNPJ numérico × GA; RENAVAM × CPF.

---

## 4. Pontos de integração

### Fase 2 — log de pergunta/resposta (obrigatório)
No caminho onde a pergunta e a resposta são persistidas hoje:
1. `let anon = anonimizador.anonimizar(&pergunta)?;`
2. Persistir `anon.texto` no lugar da pergunta crua.
3. A resposta gerada também pode ecoar PII que veio da pergunta (o LLM repete o CNPJ, por exemplo). Antes de persistir a resposta, rodar `anonimizar` sobre ela também.
4. O embedding da pergunta continua sendo calculado sobre o **texto original** (é local, in-process — não há vazamento) para não degradar a busca vetorial. Documentar isso em comentário no código.

### Fase 3 — fronteira do LLM (atrás de feature flag)
No ponto onde o prompt é montado para o LLM externo:
1. Anonimizar a pergunta do usuário **antes** de inseri-la no prompt; guardar o `mapping` no escopo da requisição.
2. Enviar o prompt com placeholders.
3. Ao receber a resposta, aplicar `restaurar(resposta, mapping)` antes de devolver ao analista.
4. Flag de configuração (`anonimizar_llm = true/false`, default `true`) — permite desligar se degradar a qualidade das respostas em produção.
5. Atenção: os documentos recuperados pela busca vetorial são conteúdo público (serviços/FAQs) e NÃO passam pela anonimização — apenas o texto do usuário.

---

## 5. Testes e critérios de aceite

### 5.1 Testes
- Portar as 20 fixtures do harness `auli-anon-eval` (fornecido junto deste documento) como testes de integração de `auli-anon` (`tests/fixtures.rs`). Manter os dados fictícios com DV válido; usar `cpf_cnpj::{cpf,cnpj}::generate()` para casos dinâmicos.
- Testes unitários por reconhecedor: casos positivos, negativos (número parecido sem contexto), e boundary (dígito colado antes/depois).
- Teste do ciclo restore: pergunta → sanitize → resposta simulada contendo os placeholders → restore → valores originais presentes.
- Teste fail-closed: forçar erro e verificar que o texto cru não passa.
- Teste UTF-8: perguntas com acentos ao redor dos matches (ç, ã, é) — offsets de byte não podem quebrar.

### 5.2 Critérios de aceite (Fase 1 completa)
| Critério | Meta |
|---|---|
| Recall nas fixtures, entidades estruturadas (todas exceto nome/razão social/endereço) | 100% (0 vazamentos) |
| Falsos positivos na fixture de controle sem PII | 0 |
| Falsos positivos em "GA 1118" (código de receita) e datas genéricas de vencimento | 0 |
| `cargo test` no workspace inteiro | verde |
| `cargo clippy -- -D warnings` em `auli-anon` | limpo |
| Latência de `anonimizar` para pergunta de 1 KB | < 5 ms (é regex in-process; medir com um bench simples, sem criterion) |

### 5.3 Fora do aceite (documentar como limitação conhecida)
Nome de pessoa, razão social e endereço livre continuam vazando na Fase 1–3. Registrar no README de `auli-anon` com o plano da Fase 4.

---

## 6. Fases (ordem de execução — uma por vez)

1. **Fase 0:** verificar licença do cloakrs; criar crate `auli-anon` vazio no workspace com deps pinadas; CI/`cargo test` verde. *Commit.*
2. **Fase 1:** reconhecedores customizados §3.1→§3.10, um commit por reconhecedor com testes; ao final, fixtures completas passando com os aceites §5.2. *Um commit por reconhecedor + um do aceite.*
3. **Fase 2:** integração no log (pergunta e resposta), com teste de integração no caminho de persistência. *Commit.*
4. **Fase 3:** sanitize/restore na fronteira do LLM, atrás de flag. *Commit.*
5. **Fase 4 (futura, NÃO implementar agora):** heurística de razão social (sequência capitalizada seguida de `Ltda|S\.?A\.?|EIRELI|ME|EPP`) e avaliação de NER leve para nomes. Apenas deixar um `TODO.md` no crate descrevendo.

---

## 7. Riscos e mitigação

- **cloakrs é jovem (0.3.0, primeiro release recente).** Versão pinada com `=`; a API usada é pequena (`Scanner`, `PromptSanitizer`, `Recognizer`, `EntityType`); se o crate for abandonado, o custo de trocar por implementação própria é baixo porque toda a lógica de domínio está nos nossos reconhecedores. Não usar features além das citadas.
- **Anonimização nunca é perfeita.** O texto anonimizado no log continua sendo tratado como dado sensível em termos de acesso; a anonimização reduz risco, não o elimina. Refletir isso na descrição da funcionalidade (não prometer "dados 100% anônimos" na aba Sobre — dizer "identificadores estruturados são automaticamente mascarados").
- **Regressão de qualidade das respostas (Fase 3).** O LLM perde acesso ao valor literal (ex.: não pode comentar "esse CNPJ é de fora do RS"). Mitigado pela flag e pelo fato de as respostas se basearem nos documentos recuperados, não nos identificadores.

---

## Anexo — referência rápida da API do cloakrs usada

```rust
use cloakrs_core::{Locale, PromptSanitizer, Recognizer, EntityType, PiiEntity, Span, Confidence};

// Montagem do scanner: registry BR nativo + reconhecedores próprios
let scanner = cloakrs_locales::default_registry()
    .into_scanner_builder()
    .locale(Locale::BR)
    .recognizer(TelefoneBrRecognizer)   // repetir para cada custom
    .build()?;                          // conferir assinatura exata do builder para recognizers extras

let sanitizer = PromptSanitizer::new(scanner);
let (texto_limpo, mapping) = sanitizer.sanitize(pergunta)?;   // "[CPF_1]" etc.
let restaurado = mapping.restore(&resposta_llm);              // devolve os originais

// PromptMappingEntry { placeholder, entity_type, original, span_start, span_end, confidence }
```

Trait a implementar (assinatura conforme cloakrs-core 0.3.0 — conferir no código-fonte do crate):

```rust
impl Recognizer for TelefoneBrRecognizer {
    fn id(&self) -> &str { "auli_telefone_br_v1" }
    fn entity_type(&self) -> EntityType { EntityType::Custom("TELEFONE".into()) }
    fn supported_locales(&self) -> &[Locale] { /* BR */ }
    fn scan(&self, text: &str) -> Vec<PiiEntity> { /* regex + boundaries + contexto */ }
    fn validate(&self, candidate: &str) -> bool { /* dígitos plausíveis */ }
}
```

Se `entity_type()` retornar tipo não-referenciável por `&self` devido ao `Custom(String)`, verificar a assinatura real da trait no fonte (`cloakrs-core/src/recognizer.rs`) e ajustar (pode exigir `EntityType` por valor ou `&'static`). Este é o primeiro ponto a conferir na Fase 1.
