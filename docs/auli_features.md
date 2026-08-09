# Auli — Propósito e Principais Funcionalidades

Visão de produto do Projeto Auli: para que serve, quem usa e o que ele faz hoje. As
funcionalidades abaixo refletem o que está implementado no código (ver
[docs/auli_code.md](docs/auli_code.md) para o detalhamento técnico e a distinção entre o que está
ativo e o que está apenas modelado).

---

## 1. Propósito

A Auli é um **assistente virtual (RAG) para impostos estaduais** que ajuda o **servidor
tributário no atendimento ao cidadão**. Em vez de o atendente procurar manualmente serviços,
perguntas frequentes, pareceres e notas espalhados pelo portal da Secretaria da Fazenda, ele
faz uma pergunta em linguagem natural e recebe uma resposta fundamentada nos conteúdos
oficiais daquele estado, com os links para aprofundamento.

Princípios do projeto:

- **Foco no atendimento público.** O destinatário primário é o servidor que atende o cidadão;
  o conteúdo e o tom das respostas são orientados a esse uso.
- **Privacidade por desenho.** A geração de embeddings (a etapa que processa o texto das
  perguntas e dos documentos) roda **localmente e no próprio processo** (fastembed/BGE-M3, sem
  serviço externo); apenas a etapa final de redação da resposta usa um LLM externo.
- **Multi-tenant por estado.** Uma mesma base de código atende várias secretarias estaduais,
  cada uma com seus próprios dados e instruções de resposta, isoladas entre si.
- **Aberto e cooperativo.** Projeto piloto open-source (licença MIT), iniciado com a SEFAZ-RS.

---

## 2. Como funciona (em uma frase)

> Pergunta em linguagem natural → embedding local in-process (fastembed/BGE-M3) → busca
> semântica nos conteúdos do estado (vector store in-process) → um LLM externo redige a resposta
> com base nos trechos recuperados.

Os conteúdos oficiais são previamente **coletados do portal** da secretaria (scrapers),
**transformados em texto estruturado** e **vetorizados em pacotes por estado** (o comando
`auli update`), que o servidor carrega prontos.

---

## 3. Funcionalidades principais

### 3.1 Perguntas e respostas com RAG (núcleo do produto)

- Resposta a perguntas em **português, em linguagem natural**, fundamentada nos conteúdos
  oficiais do estado selecionado.
- **Quatro fontes, e o usuário escolhe qual consultar** no seletor da caixa de mensagem:
  **Serviços+FAQs** juntos (o atendimento — "como faço para…"), **Pareceres** (a interpretação da
  legislação pela Receita Estadual) e **Acórdãos TARF** (as decisões do tribunal administrativo em
  recursos de contribuintes). Cada fonte tem prompt próprio; o seletor só mostra o que aquela
  entidade tem.
- Respostas com **links oficiais** para o serviço/página de origem.
- Cada estado tem seu **prompt de sistema próprio**, que define o comportamento e o tom do
  assistente para aquela secretaria.
- Comportamento robusto: estado desconhecido retorna uma mensagem amigável (não quebra), e há
  **registro das interações** para auditoria/melhoria.

### 3.2 Multi-tenant por estado (entidades)

- Um único servidor e uma única interface atendem **múltiplas secretarias estaduais**.
- Cada estado (entidade) tem **dados e configuração isolados**; as coleções de busca são
  separadas por estado, de modo que uma pergunta nunca mistura conteúdo de estados diferentes.
- **Adicionar um novo estado** é uma operação de configuração + dados (cadastrar a entidade e
  ingerir seus conteúdos), sem reescrever a aplicação.
- Estados ativos hoje: **27 secretarias**, todas com Serviços. Além disso: **Pareceres** em RS, SC,
  SP e PR; **FAQs** e **Acórdãos TARF** no RS. Todas respondem no chat com contexto do próprio
  catálogo e citam os links oficiais.

### 3.3 Privacidade: embeddings locais

- O processamento que "entende" o texto (embeddings) é feito **no próprio processo do servidor**
  (fastembed/BGE-M3, ONNX), sem qualquer serviço externo de embeddings nem envio do conteúdo das
  perguntas/documentos a terceiros.
- Apenas a **redação final** da resposta usa um LLM externo (compatível com API estilo Groq),
  configurável por variável de ambiente.

### 3.4 Interface web (frontend)

- **Tela inicial de seleção de estado** com **mapa do Brasil interativo** (estados disponíveis
  em destaque) e cards das secretarias; a escolha é **persistida** no navegador e pode ser
  trocada pelo cabeçalho.
- **Chat conversacional** com balões diferenciados para usuário e assistente, mensagem de
  "pensando", **botão de copiar** mensagens e **timeout amigável** (25s) quando a API demora.
- **Navegação por abas** (sem recarga de página), em três grupos: **Acervo** (Serviços, FAQs,
  Pareceres, Acórdãos TARF, Notas, Conteúdos), **Integrações** (Conectar sua IA, Downloads) e o
  Chat, mais o Sobre no rodapé — com o estado de cada aba preservado ao alternar. Aba de coleção que
  a entidade não tem some.
- **Páginas de referência** por estado, com **busca no cliente**:
  - **Serviços** — agrupados por classe, em acordeões, com abas por público.
  - **FAQs** — árvore navegável de perguntas e respostas.
  - **Pareceres** e **Acórdãos TARF** — lista pesquisável por número, ementa e resumo, do mais
    recente ao mais antigo, com link para a fonte oficial.
  - **Notas** e **Conteúdos** — materiais de referência.
  - **Downloads** — o acervo de cada estado em `.md`, um arquivo por documento, para apontar a
    própria IA para a pasta.
- Seções **sem dados** para o estado mostram um aviso amigável de "em breve".
- **Modo claro/escuro** que respeita a preferência do sistema e persiste a escolha.
- Layout **mobile-first**, com ajuste ao teclado virtual em celulares.

### 3.5 Ingestão e gestão de conteúdo (backend)

- A **vetorização** dos conteúdos (Serviços, FAQs, Pareceres, Acórdãos TARF) é feita por um comando
  dedicado — `auli update` —, que lê os arquivos do estado e gera **pacotes de vetores +
  manifesto** prontos para o servidor. O servidor é **somente leitura**: carrega os pacotes,
  valida o manifesto e atende; ele não escreve dados (apenas expõe a **listagem** do que está
  indexado).
- Tipos de conteúdo descritos de forma **uniforme** (um único caminho de processamento por
  tipo), o que facilita acrescentar novos tipos.

### 3.6 Acervo como serviço (retrieve + MCP) — v1

- Além do chat, o acervo pode ser consultado **como serviço**: uma rota HTTP de **recuperação pura**
  (`/v1/retrieve`) devolve os documentos com a distância de proximidade, **sem** redigir resposta e
  **sem** chamar LLM externo.
- A mesma busca é exposta por **MCP** (Model Context Protocol) em `/mcp`, com **quatro
  ferramentas**: listar as UFs e seus acervos, buscar na jurisprudência por tema (com o parâmetro
  `colecao`: pareceres ou acórdãos do TARF), ler um documento integral pelo número, e consultar
  serviços+FAQs de uma UF — esta última devolvendo o MESMO bloco de contexto que o chat usa.
- Como não há LLM nesses caminhos, a pergunta **não sai do processo** — e o registro guarda só
  metadados (UF, tipo, nº de resultados), nunca o texto perguntado.
- **Limites conhecidos:** uma UF por chamada e sem autenticação — o conteúdo é público e
  somente-leitura, protegido por limite de requisições.

### 3.7 Coleta de conteúdo (scrapers por estado)

- Um **scraper por secretaria** (um binário por estado), sobre um **kit compartilhado** (cache,
  agente HTTP, agregação), gravando todos a mesma fronteira: o **snapshot** tipado que o passo de
  derivação transforma nos arquivos padronizados (JSON estruturado + texto pronto para indexação).
- Suporta **plataformas de portal bem diferentes** com a mesma forma de saída, e **sem navegador
  headless em nenhuma**: APIs JSON, Next.js, SharePoint, WordPress, ServiceNow, ColdFusion, Angular,
  e até catálogos embutidos no bundle JavaScript. O inventário por entidade, com as armadilhas de
  cada portal, está em
  [SCRAPERS.md](auli-server/crates/scrapers/SCRAPERS.md) e em
  [docs/REGISTRO-scrapers.md](docs/REGISTRO-scrapers.md).
- **Cache em disco** das páginas coletadas e **modo offline** (reconstrói as saídas a partir
  do cache, sem acessar a rede), para reprocessar sem sobrecarregar o portal.
- **Deduplicação** de serviços que aparecem em vários públicos, evitando conteúdo repetido na
  base de busca (quando o link identifica o serviço).

---

## 4. Tipos de conteúdo

| Tipo | O que é | Onde aparece hoje |
| --- | --- | --- |
| **Serviços** | Carta de serviços da secretaria, por público-alvo | Chat (fonte Serviços+FAQs) + aba + downloads |
| **FAQs** | Perguntas frequentes oficiais | Chat (fonte Serviços+FAQs) + aba + downloads |
| **Pareceres** | Interpretação da legislação pela Receita Estadual | Chat (fonte própria) + aba + MCP + downloads |
| **Acórdãos TARF** | Decisões do tribunal administrativo em recursos | Chat (fonte própria) + aba + MCP + downloads |
| **Notas** | Notas administrativas/tributárias | Aba Notas (referência) |
| **Conteúdos** | Materiais de referência diversos | Aba Conteúdos (referência) |

Observação de produto: **quatro dos seis tipos alimentam respostas** — serviços e FAQs juntos, no
atendimento; pareceres e acórdãos separadamente, no uso do auditor. **Notas e Conteúdos** seguem só
como navegação de referência: não têm fonte estruturada e por isso não são indexados.

---

## 5. Arquitetura em três partes

| Componente                  | Papel no produto                                                                                                                                                                                         |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **auli-server** (workspace) | Cérebro: o binário `auli` em dois modos — `auli server` recebe a pergunta, busca o contexto e gera a resposta (somente leitura); `auli update` vetoriza os conteúdos em pacotes. |
| **auli-frontend**           | Experiência do usuário: seleção de estado, chat e navegação pelos conteúdos                                                                                                                              |
| **scrapers + auli-collections** | Abastecimento: os **scrapers por estado** coletam do portal (gravam o snapshot) e o **auli-collections** deriva o contrato padronizado que alimenta a busca                                            |

> O backend é o workspace **auli-server**, em camadas estritas
> (`vector-store` ← `auli-core` ← `auli-retrieval` ← `auli-cli`), mais o `auli-contract` (a forma do
> dado, compartilhada com os scrapers); ver [docs/auli_code.md](docs/auli_code.md) §3.

---

## 6. Estado atual e direção

- **Funcionando hoje:** chat com RAG nas **27 secretarias**, com quatro fontes selecionáveis;
  interface completa (chat + abas de acervo + seleção de estado com mapa + downloads do acervo em
  `.md`); acervo como serviço por `/v1/retrieve` e por MCP; e embeddings locais. **29.732 documentos
  indexados** — 4.205 serviços, 19.780 pareceres, 1.947 FAQs e 3.800 acórdãos. (O servidor é
  público, sem auth nem banco — expõe `/v1/health`, `/v1/question`, `/v1/retrieve`,
  `/v1/{kind}/list` e `/mcp`.)
- **Em evolução:** fechar a coleta do TARF (3.800 de ~22.522); FAQs além do RS; e busca híbrida
  (densa + léxica), medida em [docs/ESTUDO-busca-hibrida.md](docs/ESTUDO-busca-hibrida.md) — hoje uma
  palavra rara solta não volta bem, e o estudo diz por quê.

Para o detalhamento técnico — rotas, o caminho RAG passo a passo, a arquitetura das coleções —
consulte [docs/auli_code.md](docs/auli_code.md). Para operar (coletar, vetorizar, publicar),
[docs/auli_operations.md](docs/auli_operations.md).
