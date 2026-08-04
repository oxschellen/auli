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
que segue **fora de escopo** — e o dicionário já foi tentado e medido: ver §5.

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
`PersonNameRecognizer` do cloakrs. O caminho 2 foi implementado e **medido** — ver §5, que é a
razão de ele continuar aberto.

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

## 5. Fase 5 (caminho 2, dicionário de prenomes) — **medida e REPROVADA como está** (2026-08-04)

O caminho 2 foi implementado num patch (`NomeDicionarioRecognizer`, `auli_nome_dicionario_v1`,
confiança 0.6): 18.938 prenomes do Censo 2010 do IBGE + 2.462 municípios compostos como guarda,
embutidos por `include_str!`, gerados por `tools/gera_prenomes.py`. Ele pega o **nome solto**, sem
gatilho — o buraco que a §4.3 declarou.

O patch aplica limpo, tem 80 testes unitários + 8 de fixture verdes e clippy limpo. **Mesmo assim
não deve ser mergeado**: quando medido fora das próprias fixtures, o custo supera o ganho por duas
ordens de grandeza. Registrado aqui para que a Fase 5 seja retomada pelo diagnóstico, e não do zero.

### 5.1 O que foi medido, e como

Duas medições, ambas reproduzíveis. A dos logs imprime **só agregados** — nenhum trecho de
pergunta sai do processo (doutrina §7.0 do `auli_operations.md`).

**(a) Tráfego real** — 236 perguntas extraídas de `PERGUNTA (ORIGINAL)` dos arquivos de `logs/`:

| | |
|---|---|
| Perguntas com sequência capitalizada de 2–4 tokens | 96 (41%) |
| Total dessas sequências | 161 |
| …precedidas de gatilho (a Fase 4 pega) | **1** |
| …barradas pela stoplist institucional | 45 |
| …em início de frase (maiúscula gramatical) | 22 |
| …**órfãs** (candidatas a *miss*) | **93** |

Antes do patch, um sanity check com um dicionário pequeno (138 prenomes colhidos da API pública do
Censo — o *ranking* satura rápido, os mesmos nomes de topo dominam todas as UFs e décadas) achou
**0 das 93 órfãs** começando por prenome. O dicionário de 18.938 do patch é ~140× maior e alcança
a cauda, o que **inverte a pergunta**: deixa de ser "quanto recall ganha" e passa a ser "quanto
vocabulário institucional ele marca como nome".

**(b) Prosa institucional** — 3.000 documentos públicos de `data/<id>/docs/**` (23 MB). Por
construção, quase nada ali deveria ser nome de pessoa.

### 5.2 O resultado

**Ganho:** detecções de NOME nas 236 perguntas reais foram de **1 para 2**. A detecção nova não
aparece em nenhum documento público do acervo — ou seja, é provavelmente PII de verdade. Um acerto.

**Custo:** no acervo público, **397 detecções de NOME, 137 distintas**. As mais frequentes:

| Ocorrências | Detectado como nome | O que realmente é |
|---|---|---|
| 76× | `Franca de Manaus` | **Zona Franca de Manaus** |
| 44× | `Aliomar Baleeiro` | doutrinador citado no parecer |
| 15× | `Coleta de Carga` | serviço |
| 11× | `Marinha Mercante` | termo do domínio |
| 6× | `Vida Gerador de Benefício Livre` | **VGBL** |
| 5× | `Paulo e Paraná` | de "São Paulo e Paraná" |
| 5× | `Laudo Médico` | documento |
| 3× | `Não Incidência` | conceito tributário |

Sondas diretas confirmaram: **8 de 10 frases institucionais típicas são mascaradas**.
`a Zona Franca de Manaus tem regime próprio` sai como `a Zona [NOME_1] tem regime próprio`.

### 5.3 As três falhas — estruturais, não de calibração

1. **A guarda de topônimo é contornada pelo próprio recorte.** O candidato começa no *primeiro
   prenome* da sequência, então `São Paulo e Paraná` vira candidato `Paulo e Paraná` — e a guarda,
   que compara **igualdade exata** com a lista de municípios, nunca chega a ver `São Paulo`.
   `SAO PAULO` **está** no arquivo e mesmo assim vaza. A guarda só funciona quando o topônimo
   *começa* com prenome (`Bento Gonçalves`, `Vera Cruz`, `Carlos Barbosa`) — exatamente o conjunto
   que os testes do patch exercitam. **Os testes passam por acidente de seleção.**
2. **O conector `e` encadeia itens de lista:** `Laudo Médico e Laudo Técnico` vira uma detecção só;
   `Santa Catarina e São Paulo` vira `Catarina e São Paulo`. Faz sentido no reconhecedor ancorado,
   onde o gatilho segura o contexto; sem gatilho, funde coisas não relacionadas.
3. **Prenome + qualquer Titlecase é porteiro fraco.** `Franca`, `Coleta`, `Vida`, `Laudo` e
   `Marinha` são prenomes reais do Censo. O que os salvaria é exigir que o **segundo** token
   também seja plausível — sobrenome de dicionário, não qualquer palavra capitalizada.

### 5.4 Por que isso importa em produção

O texto anonimizado é o que vai ao LLM (Fase 3, `AULI_ANONIMIZAR_LLM` ligado por padrão): uma
pergunta com `Zona Franca de Manaus` chega ao modelo **sem o sujeito**. A recuperação não sofre —
o embedding usa o texto original —, o dano é no prompt e, por consequência, na resposta.

A exposição hoje ainda é baixa: só **1 das 236** perguntas contém alguma das armadilhas testadas
(`Não Incidência`). Mas é uma pergunta sobre Zona Franca de distância, e AM está no acervo.

### 5.5 Se a Fase 5 for retomada

- Guarda de topônimo por **contenção sobre a sequência inteira**, não igualdade sobre o candidato
  recortado.
- `e` **fora** dos conectores deste reconhecedor (mantido no ancorado).
- Testar **exigir sobrenome de dicionário no 2º token**. É a mudança de maior impacto e a que
  decide a viabilidade: mataria `Franca de Manaus`, `Coleta de Óleo Usado` e `Laudo Médico`,
  preservando `João da Silva Pereira`. Exige uma fonte de sobrenomes — a do patch só tem prenomes.
- **Gate obrigatório:** repetir a medição (b) sobre o acervo antes de aprovar qualquer versão.
  Contagem de NOME em texto institucional é o número que decide, não o resultado das fixtures.
- Pendências menores do patch: `cargo fmt --check` reprova em 6 pontos (gate do CI; o commit cita
  clippy, não fmt) e nenhuma atualização de documentação, ao contrário das fases anteriores.

### 5.6 Achado lateral — os reconhecedores nativos em texto longo

Na varredura do acervo apareceram também `URL` 3447×, `IP_ADDRESS` 58× e `CREDIT_CARD` 1× — todos
de reconhecedores **nativos** do cloakrs. Não é problema em produção (documento público não passa
pelo anonimizador; só a pergunta passa), mas mostra que numeração legal e endereço de portal
disparam esses padrões. Se algum dia o acervo passar a ser anonimizado, começar por aqui.

## 6. Anonimização **estrutural** de pareceres (aberta — próximo passo, 2026-08-04)

Direção nova, e a mais promissora depois da §5. O Carlos vai **buscar o texto de pareceres novos**
para analisarmos a estrutura do documento; a hipótese é **anonimizar apenas a parte do documento
que identifica o contribuinte**, em vez de varrer o texto inteiro com reconhecedores de entidade.

**Por que isto ataca exatamente onde a §5 quebrou.** A Fase 5 falhou porque um reconhecedor de
nome em texto livre não distingue `Aliomar Baleeiro` — doutrinador citado, que **deve** permanecer
— do nome do consulente, que **deve** sair. Os dois são nomes de pessoa; o que os separa não é a
forma, é **onde estão no documento**. Estrutura resolve o que a heurística lexical não resolve: em
vez de perguntar "isto parece nome?", passa a perguntar "isto está no bloco de identificação?".

Muda também o **ponto do pipeline**: tudo no `auli-anon` hoje roda sobre a **pergunta**, em
`auli-cli`. Anonimizar documento é na **ingestão** (scraper/`auli-collections`), antes de o texto
virar `docs/` e pack. Vale decidir cedo se o crate ganha uma API para isso ou se a lógica mora na
ingestão — a §2.1 do plano avisa que essa escolha já foi feita errado uma vez.

### 6.1 O acervo atual **já vem saneado da fonte**

Ponto de partida, confirmado pelo Carlos e corroborado pela contagem: **os pareceres públicos já
são saneados pela própria SEFAZ antes da publicação.** Não é efeito colateral do nosso pipeline
nem sorte de amostragem — é etapa editorial do publicador. A medição nas quatro entidades com
pareceres indexados:

| UF | Pareceres | Com papel (`consulente`/`consignante`/`interessado`) | Com marcador `XYZ` |
|---|---|---|---|
| rs | 372 | 182 | 23 |
| sc | 1.743 | 1.741 | 0 |
| pr | 2.060 | 2.056 | 0 |
| sp | 15.605 | 15.605 | 3 |

O RS chega a substituir o nome por um literal (`XYZ vem formular consulta de seu interesse`); SC,
PR e SP usam o papel (`A consulente indaga`, `A Consulente informa`). **Para o acervo de hoje, não
há o que anonimizar.**

Isso reposiciona a seção: o alvo não é o acervo existente, é **verificar se a mesma prática vale
para as fontes novas**. Saneamento na origem é convenção do publicador, não garantia contratual —
pode variar por UF, por época e por formato (o PDF antigo digitalizado é o suspeito natural). A
amostra nova serve para confirmar a regra e, principalmente, para achar a exceção.

### 6.2 A estrutura observada (amostra pequena, a confirmar)

Os documentos já ingeridos têm cabeçalho estável, e é nele que uma identificação apareceria:

- **RS** — `Informação n.º NNNNN` + `Assunto :` + cidade/data + a frase de abertura do relato.
- **SC** — `CONSULTA N/AAAA` + `EMENTA:` + `DA CONSULTA`.
- **PR** — `CONSULTA Nº: NN, de <data>` + `SÚMULA:` + relato.
- **SP** — `RESPOSTA À CONSULTA TRIBUTÁRIA NNNN/AAAA, de <data>` + `Ementa` + itens numerados.

No formato `.md` do acervo isso vive sob `## corpo`; acima dele há o frontmatter (`numero`,
`assunto`, `link`) e a `## sinopse` gerada por LLM — **a sinopse também precisa entrar no escopo**,
porque ela reescreve o relato e pode reintroduzir um nome que o corpo trouxesse.

### 6.3 Perguntas que a amostra nova precisa responder

1. A fonte identifica o contribuinte? O esperado é que **não** — as quatro atuais já vêm saneadas
   da origem. Se a nova também vier, o trabalho acaba aqui, e o resultado é uma linha nesta seção.
2. Se identifica, **onde** — cabeçalho, relato, assinatura, anexo? É região contígua e rotulada?
3. A estrutura é estável dentro da UF e entre UFs? O que acontece com PDF convertido, onde a
   quebra de linha destrói o rótulo?
4. Identificação estruturada (CNPJ/IE do consulente) já é coberta pelos reconhecedores das Fases
   0/1 — o que sobra de fato é **nome/razão social** dentro daquela região.
5. Se a região for identificável, o mais seguro é **suprimir a região inteira** (ou substituí-la
   por um marcador) em vez de rodar reconhecedor dentro dela: menos precisão exigida, zero risco
   de mascarar doutrina.

### 6.4 Cuidados

- **Não vale para a pergunta do usuário.** Isto é sobre documento; o texto que o analista digita
  continua sob as Fases 0–4.
- **Reprocessamento.** Anonimizar na ingestão muda o texto embarcado ⇒ re-gera pack e espaço
  vetorial da entidade. Decidir se vale reprocessar o acervo existente ou só valer daqui pra frente.
- **A §5.6 continua valendo:** rodar o anonimizador inteiro sobre documento é ruim (397 falsos
  positivos de NOME em 3.000 arquivos). A proposta desta seção é o oposto — escopo estreito,
  definido por estrutura.

## 7. Fase 6 (razão social sem sufixo, por dicionário de segmentos) — **analisada, não aplicada**

Existe um patch de Fase 6 (`RazaoSegmentoRecognizer`, `auli_razao_segmento_v1`, confiança 0.65)
que ataca outra limitação declarada da §4: **razão social sem sufixo societário** —
`Anderle Transportes` sem o `Ltda`. O porteiro deixa de ser o sufixo e passa a ser um **substantivo
de ramo de atividade** (~120 termos curados em `data/segmentos_razao.txt`: `Transportes`,
`Padaria`, `Distribuidora`, `Metalúrgica`…), exigindo além dele ao menos um token Titlecase
**distintivo** — o nome da empresa.

**Decisão de trabalho (2026-08-04): apenas analisado e registrado. Não aplicado, não medido em
execução.** Primeiro se resolve a questão dos nomes (§5 e §6); a Fase 6 entra depois.

### 7.1 O que o patch faz bem

- **Lista curada e pequena** (120 substantivos), não dicionário massivo — o oposto da §5.
- **Adjetivos deliberadamente fora** (`Comercial`, `Industrial`, `Atacadista`), com o motivo
  escrito: `Código Comercial` e `Distrito Industrial` teriam densidade de armadilha maior que o
  ganho. É a lição da §5 aplicada de forma preventiva.
- **Duas condições**, não uma: termo de segmento **e** nome distintivo. `o setor de Transportes`
  não casa porque falta o distintivo.
- **Armadilhas locais** (`Federação`, `Câmara`, `Distrito`, `Terminal`…) em lista própria, não na
  stoplist global, para não mudar o comportamento das Fases 4–5 — `Câmara` é sobrenome legítimo.
- Confiança 0.65 acima do prenome (0.6) com justificativa: em `Rosa Transportes`, o termo de
  segmento é sinal mais forte de empresa do que o prenome é de pessoa, e no empate de span decide
  a confiança.

### 7.2 Dependência estrutural: o patch está empilhado sobre a §5

Ele **modifica** `nome_dicionario.rs` (promove `normaliza` para o `comum.rs`) e traz uma trava de
convivência entre segmento e prenome. Ou seja: **não aplica sobre `main`**, só sobre a Fase 5 —
que está reprovada. Herdaria os 397 falsos positivos de NOME junto.

A dependência, porém, é **de embalagem, não de conceito**: a Fase 6 não precisa dos 18.938
prenomes, só do helper `normaliza` e da própria lista de segmentos. Desempilhar é trabalho pequeno,
e é pré-condição para avaliá-la em separado.

### 7.3 O que a sonda mostrou

A lógica do porteiro foi **reimplementada em Python** e rodada sobre os mesmos corpora da §5 — o
patch **não** foi aplicado nem compilado. É aproximação do `RE_SEQ` do Rust (tokenização
equivalente, com pontuação encerrando a sequência), então os números são de ordem de grandeza, não
exatos:

| | |
|---|---|
| Acervo público (3.000 documentos) | **373 detecções, 120 distintas** |
| Perguntas reais (236) | **0 detecções, 0 perguntas afetadas** |

As mais frequentes no acervo — todas falso positivo:

| Ocorrências | Detectado como razão social | O que é |
|---|---|---|
| 99× | `Ministérios da Ciência e Tecnologia` | órgão federal |
| 23× | `Livre Comércio` | de "Área de Livre Comércio" |
| 14× | `Depósito Alfandegado Certificado` | regime aduaneiro |
| 14× | `Comércio Exterior Camex` | órgão |
| 11× | `Empresa Brasileira de Pesquisa Agropecuária` | Embrapa |
| 10× | `Zonas de Livre Comércio` | conceito legal |
| 9× | `Lei de Informática` | a lei |

### 7.4 O defeito de maior alavanca: a lista de armadilhas é **singular**

`ARMADILHAS` tem `MINISTERIO`, `ZONA`, `DISTRITO`, `CAMARA`, `FEDERACAO`, `CENTRO`, `SETOR`,
`TERMINAL` — e o texto tributário usa o **plural**. Nenhum casa:

`Ministérios`→`MINISTERIOS`, `Zonas`→`ZONAS`, `Áreas`→`AREAS` (nem singular existe na lista),
`Distritos`, `Câmaras`, `Federações`, `Centros`, `Setores`, `Terminais` — **9 de 9 escapam**.
Sozinho, isso explica os dois maiores falsos positivos (122 das 373 detecções).

É o **mesmo padrão de falha da §5.3**: guarda que compara forma exata contra texto que varia. Lá
era o topônimo composto, aqui é o plural. Vale como regra geral para a família: *guarda por
igualdade exata é frágil; guarda por normalização morfológica ou por contenção é o que resiste.*

Dois defeitos menores, herdados da §5: o conector `e` encadeia itens de lista
(`Indústria e Comércio Exterior`, `Tarifas e Comércio`) e o span emitido é a **sequência inteira**,
sem recorte.

### 7.5 Quando for retomada

- **Desempilhar da §5** primeiro (só `normaliza` + a lista de segmentos são necessários).
- Armadilhas com **plural** — ou comparação por radical, ou as duas formas na lista.
- `e` fora dos conectores, como na §5.5.
- **Mesmo gate da §5.5:** medir contagem em texto institucional antes de aprovar. O alvo declarado
  (`Anderle Transportes` sem sufixo) é legítimo e continua sendo limitação aberta da §4 — o que
  falta é um porteiro que não marque `Comércio Exterior` junto.

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
