# ESTUDO-byok — Trazer o próprio modelo (BYOK) para o Auli

**Data:** 04/10/2026 · **Status:** estudo conceitual, sem decisão de implementação.
**Leitura prévia:** [auli_code.md](auli_code.md) §3.4.1 (as decisões D-SF-\*, em especial a
triagem em duas chamadas) e [REGISTRO-triagem.md](REGISTRO-triagem.md) (o que a fumaça contra o LLM
real mostrou sobre um modelo só).

**Convenção:** como no REGISTRO-triagem, distingo **medido** (saiu de execução real), **deduzido**
(inferência com a evidência ao lado) e **não verificado** (fica para a §13).

---

## 1. A pergunta e a resposta curta

**Pergunta:** como um usuário do Auli pode usar o **seu próprio** modelo de linguagem, com a sua
chave de API, em vez do modelo que o servidor paga?

**Resposta curta:** o que decide tudo é **por onde a chave passa**. Há quatro caminhos; dois já
existem e só precisam ser assumidos como recurso. Dos dois novos, este estudo recomenda o
**navegador chamar o LLM** — a chave nunca sai do dispositivo do usuário — com um protocolo em que
**o servidor pensa e o navegador disca**: o servidor continua fazendo anonimização, busca, triagem e
composição dos prompts, e só a chamada ao modelo muda de lugar. O caminho em que a chave viaja até o
servidor (proxy) é **descartado** (§5).

## 2. Por que considerar

- **Custo e cota.** Hoje toda resposta do chat sai da chave do servidor, num provedor com cota
  diária. Com BYOK, quem traz a chave paga o próprio uso, e o modelo padrão do servidor continua para
  quem não tem.
- **Qualidade.** Os prompts foram calibrados num modelo de pesos abertos. Quem quiser um modelo de
  fronteira — que tende a seguir melhor regras negativas como "nunca combine documentos" — pode
  trazê-lo. É a hipótese do item 2 da lista de alternativas que originou a série D-SF, agora
  disponível ao usuário sem trocar o padrão de todos.
- **Independência.** O Auli se afirma como **infraestrutura de busca**; o modelo vira escolha de quem
  usa. Coerente com a filosofia do projeto: o núcleo é achar a informação, e o texto do LLM é
  complemento.
- **Hospedagem gerenciada.** Uma instância pública atende muita gente sem pagar o LLM de todos —
  pré-condição prática do modelo de hospedagem que já foi explorado.
- **Privacidade máxima, por escolha.** Com modelo local (§9), nem a pergunta anonimizada sai da
  máquina de quem pergunta.

## 3. O que já existe

### 3.1 Instância própria

O Auli é MIT, e o `.env` já é exatamente o trio de um BYOK: `LLM_API_URL`, `LLM_API_KEY`,
`LLM_API_MODEL` (mais o `LLM_REASONING_EFFORT` da D-SF-6). Quem sobe a sua instância já traz a sua
chave. **Lacuna:** a documentação trata isso como detalhe de instalação, não como recurso.

### 3.2 "Conectar sua IA" (MCP)

É o BYOK mais radical: o assistente do usuário (o cliente MCP) chama as ferramentas de busca do
Auli, e o Auli **nunca toca em chave nenhuma**. Duas lacunas:

1. **O assistente externo não recebe o prompt do Auli.** Ele recebe o contexto, mas não as regras
   do `rs.txt` v2 (não combinar documentos, irmãos como alternativas, citação por afirmação) nem a
   triagem. **Deduzido:** com o contexto integral e sem as regras, ele está sujeito ao mesmo defeito
   de fusão que motivou a série D-SF. **Remédio barato:** o MCP tem a primitiva de **prompts**, além
   da de ferramentas; hoje o servidor anuncia só ferramentas (`enable_tools()` em `mcp.rs`).
   Publicar o `rs.txt` como prompt MCP entrega as mesmas regras ao assistente externo.
2. **A pergunta não passa pela anonimização.** O usuário a digita no próprio assistente, que a
   manda ao provedor dele; o Auli recebe só a consulta de busca que o assistente decidir fazer. Isso
   é inerente ao modo e precisa estar **dito** no manual, não resolvido.

## 4. O critério: por onde passa a chave

| Modo | Onde mora a chave | Quem chama o LLM | Anonimização | Log da consulta | Esforço |
| --- | --- | --- | --- | --- | --- |
| Instância própria (§3.1) | `.env` de quem hospeda | o servidor dele | igual a hoje | igual a hoje | nenhum (documentar) |
| MCP (§3.2) | no assistente do usuário | o assistente | **não se aplica** | só a busca | baixo (prompt MCP) |
| **Proxy** (§5) | viaja a cada requisição até o servidor | o servidor | igual a hoje | igual a hoje | baixo — **descartado** |
| **Navegador** (§6) | só no navegador do usuário | o navegador | igual a hoje | igual a hoje (com retorno) | médio — **recomendado** |

## 5. Por que descartar o proxy

O proxy parece o mais simples: o usuário manda a chave num cabeçalho, o servidor chama o provedor
com ela e não grava nada. São quatro objeções, e qualquer uma bastaria:

1. **Custódia.** Credenciais de terceiros passam a transitar pela máquina de quem mantém o Auli.
   Mesmo sem gravá-las, a responsabilidade por elas existe — e é desproporcional ao ganho.
2. **O túnel termina o TLS na borda.** A API é exposta por túnel da Cloudflare, que termina o TLS na
   borda dela. **Deduzido** da natureza do túnel: o cabeçalho com a chave fica legível nessa
   infraestrutura antes de chegar ao servidor.
3. **SSRF.** Se o usuário escolhe a URL do provedor, o servidor passa a fazer requisições para
   endereços arbitrários — inclusive da rede onde ele roda. Fechar isso exige lista fixa de
   provedores, e com lista fixa já não é "traga o seu".
4. **Vazamento por log.** Basta um `debug!` de cabeçalho, hoje ou num refactor futuro, para a chave
   ir ao disco. A única forma de não vazar uma chave é ela não estar lá.

## 6. O desenho recomendado: o servidor pensa, o navegador disca

### 6.1 O problema que o desenho resolve

A resposta de Serviços + FAQs no `rs` tem **duas chamadas** (D-SF-9), e a segunda depende da
primeira: o servidor interpreta o JSON da triagem, filtra os descartados preservando a numeração e
compõe o segundo prompt com o marcador `{{CONTEXTO}}` (D-SF-8). Pôr o navegador para conduzir isso
**duplicaria no frontend** `compor_system_prompt`, o parse da triagem e o filtro — duas fontes de
verdade para a mesma lógica, que divergiriam no primeiro ajuste.

### 6.2 O laço

O servidor conduz e o navegador executa. A conversa tem a forma de um laço:

1. O navegador envia a pergunta (como hoje, com a entidade e o tipo) e avisa que vai trazer o
   próprio modelo. O servidor anonimiza, busca e devolve **não a resposta, mas a próxima chamada a
   fazer**: o prompt de sistema e a mensagem do usuário, já prontos, mais o `log_id` e as `fontes`.
2. O navegador chama o LLM do usuário, com a chave dele, e devolve o texto ao servidor.
3. O servidor interpreta: se era a triagem, devolve a próxima chamada (a redação, com o contexto
   filtrado); se era a redação, devolve **fim** com o texto final, e o navegador o mostra.

O contrato, em forma ilustrativa (nomes a fixar na implementação):

```text
→ POST pergunta { question, entity, type, modelo_proprio: true }
← { log_id, fontes, chamar: { system, user, parametros } }

→ POST continuar { log_id, saida: "<texto devolvido pelo LLM do usuário>" }
← { chamar: { ... } }            // ainda há etapa (ex.: depois da triagem)
← { fim: { answer } }            // acabou; answer já restaurado
```

O laço vale para **qualquer** fluxo futuro de várias etapas, e toda a lógica fica num lugar só — o
Rust. Os tipos de uma chamada só (pareceres, TARF, legislação) são o caso degenerado: uma volta.

### 6.3 Estado entre as voltas

Entre uma volta e outra, o servidor guarda o estado da consulta **em memória**, chaveado pelo
`log_id`: o contexto, a etapa em que está, os tempos e o `mapping` da anonimização.

O `mapping` merece atenção. Hoje ele vive **no escopo de uma requisição** e nunca é persistido
(comentário em `rag.rs`), e a decisão de manter o `auli-anon` como biblioteca registrou que restore
com estado é "PII por sessão". No laço, ele precisa sobreviver entre as voltas — então o tempo de
vida passa de uma requisição para a duração do laço. Para não mudar a natureza da garantia:

- só memória, nunca disco;
- prazo curto, da ordem do orçamento da consulta (hoje 70 s no frontend), descartado no **fim**, no
  erro ou no vencimento;
- reinício do servidor perde tudo, e a consulta em curso falha — aceitável.

A alternativa — devolver o `mapping` ao navegador para ele restaurar — tira o estado do servidor,
mas põe o `mapping` de volta na rede e cria um segundo lugar que restaura. Não recomendada.

### 6.4 O que se preserva

- **Anonimização:** o navegador só recebe a pergunta **já anonimizada** dentro do prompt. Ela
  protege o contribuinte, não o dono da chave — vale em qualquer modo.
- **Restauração:** continua no servidor, sobre o texto devolvido na última volta.
- **Log da consulta:** pergunta, contexto, triagem e resposta continuam no servidor, gravados no
  **fim** (ou no erro). A base de calibragem não perde nada — desde que a resposta volte (§12, D2).
- **Fontes e citação:** iguais às de hoje (D-SF-7, D-SF-10).
- **Fail-open da triagem:** se o texto devolvido na volta da triagem não for JSON válido, o servidor
  segue com o contexto integral, exatamente como hoje.

### 6.5 Tempo

Cada volta acrescenta uma ida e vinda ao servidor — pequena perto dos segundos do LLM (o REGISTRO
mediu 7–9 s por consulta com `medium`). Os tetos de tempo das chamadas ao LLM passam a ser do
navegador; o servidor só precisa do prazo do estado (§6.3).

## 7. A chave no navegador

### 7.1 Onde guardar

- **Padrão: só na memória da aba.** Fechou a aba, sumiu.
- **"Lembrar neste navegador"**: opção explícita, com aviso de que qualquer pessoa com acesso a este
  navegador pode usá-la.
- **Nunca** no servidor, nunca em cookie, nunca em URL.
- Configurar o próprio modelo **não exige login**: o acervo e o chat seguem públicos.

### 7.2 XSS passa a valer uma chave

Com a chave na página, um XSS na SPA deixa de ser incômodo e passa a ser roubo de credencial. O chat
renderiza markdown que vem do LLM e do portal. **Deduzido:** o `react-markdown` não renderiza HTML
cru sem um plugin como o `rehype-raw`, e o repositório não o usa — mas a superfície precisa de
revisão explícita antes de a chave entrar na página (§13).

### 7.3 Política de conexões (CSP)

**Deduzido** por busca no repositório: hoje não há `Content-Security-Policy` declarada. O BYOK é o
momento de criar uma, porque é quando ela passa a proteger algo valioso: `connect-src` limitado ao
servidor do Auli **mais** os domínios dos provedores. O meio-termo é uma lista de provedores
conhecidos mais a opção "URL personalizada", com confirmação explícita.

### 7.4 CORS dos provedores

O navegador só consegue chamar o provedor se o provedor aceitar chamadas de origem de navegador.
Uns aceitam; outros exigem um cabeçalho de consentimento explícito; modelos locais exigem liberar a
origem na configuração deles. **Não verificado** provedor a provedor — é a primeira tarefa antes de
prometer suporte a qualquer um (§13).

## 8. Compatibilidade de modelos

- **Formato comum:** compatível com OpenAI (URL base + chave + modelo) — o mesmo trio do `.env`, e o
  que a maior parte dos provedores e dos servidores locais aceita.
- **Parâmetros:** o `parametros` que o servidor devolve em cada volta (§6.2) diz o que mandar —
  `reasoning_effort` só para modelos que o aceitam. O episódio do `high` (REGISTRO §3.2) mostrou que
  o mesmo parâmetro, noutro contexto, esvazia a resposta: parâmetro de modelo não se presume.
- **Triagem exige JSON.** Modelo que não segue o formato faz a triagem cair aberta — a resposta sai
  com o contexto integral, como uma chamada só. Funciona, mas sem o benefício.
- **Modelos testados × os demais.** A interface separa "testados" — com fumaça registrada no padrão
  do REGISTRO-triagem (respostas não vazias, citação conferida, triagem com JSON válido, os dois
  regimes) — de "outros, por sua conta". A lista de testados é dado, não código.
- **Resposta vazia:** a guarda do #170 (`texto_ou_aviso`) vive no servidor e se aplica ao texto
  devolvido pelo navegador do mesmo jeito — vazio vira aviso visível.

## 9. Modelo local

Ollama, LM Studio e afins expõem o formato compatível com OpenAI em `localhost`. Cabem no mesmo
desenho sem nada a mais, e têm um argumento próprio para quem lida com dado fiscal: **nada sai da
máquina de quem pergunta**, nem a pergunta anonimizada. A qualidade tende a ser menor, e a triagem
pode falhar mais (§8) — mas, para alguns usuários, a garantia vale mais que a qualidade.

## 10. Interface (conceitual)

- Um painel **"Seu modelo"** acessível do cabeçalho: provedor (lista + "personalizado"), URL base,
  modelo, chave, **"Testar conexão"** (uma chamada mínima, que também revela CORS), e "Lembrar neste
  navegador".
- Indicador discreto no chat de que a resposta virá do modelo do usuário.
- No rodapé de cada resposta: **quem respondeu** — "modelo X (sua chave)" ou "modelo do servidor".
- O log da consulta registra o **modelo** e a **origem** (servidor ou BYOK) — sem a chave, claro.
- Erro do provedor do usuário (chave inválida, cota, CORS) aparece como erro **dele**, com a
  sugestão de voltar ao modelo do servidor — nunca como falha do Auli.

## 11. Riscos e o que não fazer

- **Não** aceitar a chave no servidor em nenhuma rota, nem "só para testar conexão" — o teste é do
  navegador.
- **Não** duplicar no frontend a lógica de triagem e composição (§6.1).
- **Não** relaxar a anonimização por o usuário "ter escolhido o próprio provedor": ela protege o
  contribuinte, que não escolheu nada.
- **Não** prometer suporte a provedor sem CORS verificado e fumaça registrada.
- **Risco aceito:** a resposta gerada no navegador é devolvida pelo próprio navegador — um usuário
  pode adulterá-la antes de voltar. Ela só afeta o log **dele**, e o log nunca foi prova de nada além
  do que o próprio usuário viu. Registrar no log que a resposta veio de BYOK basta.

## 12. Decisões em aberto

| | Decisão | Opções | Inclinação deste estudo |
| --- | --- | --- | --- |
| D1 | Descartar o proxy | aceitar / rejeitar o descarte | aceitar (§5) |
| D2 | A resposta gerada no navegador volta ao log | sempre / só com consentimento | sempre — o laço já a devolve para restaurar, e sem ela o log perde a resposta |
| D3 | Onde guardar a chave por padrão | memória da aba / navegador | memória da aba, com opt-in |
| D4 | URL do provedor | livre / lista fechada / lista + personalizado com confirmação | lista + personalizado com confirmação |

## 13. Não verificado

1. CORS de cada provedor candidato (inclusive servidores locais e seus requisitos de origem).
2. Se o `rmcp` na versão em uso expõe a primitiva de prompts com o mesmo esforço das ferramentas.
3. Revisão da superfície de XSS do chat (markdown do LLM e do portal, links, a lista de fontes).
4. Comportamento dos prompts calibrados em modelos de outras famílias — só a fumaça responde.
5. Se o túnel em uso termina mesmo o TLS na borda na configuração atual (deduzido da natureza do
   serviço, §5, item 2) — irrelevante se o proxy for descartado, mas vale para outras decisões.

## 14. Fases sugeridas

1. **Sem código novo no chat:** documentar a instância própria (§3.1) e o MCP (§3.2) como "traga
   sua IA", incluindo a nota de que no MCP a pergunta não passa pela anonimização; publicar o
   `rs.txt` como prompt MCP.
2. **O laço** (§6), com o painel mínimo (§10), primeiro nos tipos de uma chamada só, depois na
   triagem. Pré-requisitos: CORS verificado para ao menos um provedor, revisão de XSS e a CSP.
3. **Modelos locais, perfis de modelos testados** e a lista "testados × por sua conta" (§8, §9).

Nada aqui deve passar à frente das pendências da §40 da [auli_pendencias.md](auli_pendencias.md):
a validação v1 × v2 e a medida da triagem dizem se o desenho atual responde bem — e o BYOK só
multiplica esse desenho por mais modelos.
