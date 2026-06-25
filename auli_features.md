# Auli — Propósito e Principais Funcionalidades

Visão de produto do Projeto Auli: para que serve, quem usa e o que ele faz hoje. As
funcionalidades abaixo refletem o que está implementado no código (ver
[auli_code.md](auli_code.md) para o detalhamento técnico e a distinção entre o que está
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
- Recuperação semântica combinando, hoje, **Serviços** e **Perguntas Frequentes (FAQs)** como
  contexto da resposta.
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
- Estados atualmente presentes no projeto: **RS (SEFAZ-RS)** — conteúdo completo — e
  **SC (SEF-SC)** — em implantação, com Serviços já coletados. (O atendimento de chat por
  estado depende de o estado estar configurado também no backend; ver [auli_code.md](auli_code.md).)

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
- **Navegação por abas** (sem recarga de página): Chat, Serviços, FAQs, Pareceres, Notas,
  Conteúdos e Sobre — com o estado de cada aba preservado ao alternar.
- **Páginas de referência** por estado, com **busca no cliente**:
  - **Serviços** — agrupados por classe, em acordeões, com abas por público.
  - **FAQs** — árvore navegável de perguntas e respostas.
  - **Pareceres** e **Notas** — conteúdo textual com links.
  - **Conteúdos** — categorias de materiais de referência.
- Seções **sem dados** para o estado mostram um aviso amigável de "em breve".
- **Modo claro/escuro** que respeita a preferência do sistema e persiste a escolha.
- Layout **mobile-first**, com ajuste ao teclado virtual em celulares.

### 3.5 Ingestão e gestão de conteúdo (backend)

- A **vetorização** dos conteúdos (Serviços, FAQs, Pareceres, Notas) é feita por um comando
  dedicado — `auli update` —, que lê os arquivos do estado e gera **pacotes de vetores +
  manifesto** prontos para o servidor. O servidor é **somente leitura**: carrega os pacotes,
  valida o manifesto e atende; ele não escreve dados (apenas expõe a **listagem** do que está
  indexado).
- Tipos de conteúdo descritos de forma **uniforme** (um único caminho de processamento por
  tipo), o que facilita acrescentar novos tipos.

### 3.6 Coleta de conteúdo (scrapers — auli-collections)

- Programa de **scraping reutilizável** que coleta os conteúdos do portal de uma secretaria e
  gera arquivos padronizados (estruturados em JSON e em texto pronto para indexação).
- Suporta **plataformas de portal diferentes** por estado (ex.: o portal RS, baseado em CMS, e
  o portal SC, baseado em API JSON), com a mesma forma de saída.
- **Cache em disco** das páginas coletadas e **modo offline** (reconstrói as saídas a partir
  do cache, sem acessar a rede), para reprocessar sem sobrecarregar o portal.
- **Deduplicação** de serviços que aparecem em vários públicos, evitando conteúdo repetido na
  base de busca.

---

## 4. Tipos de conteúdo

| Tipo | O que é | Onde aparece hoje |
| --- | --- | --- |
| **Serviços** | Carta de serviços da secretaria, por público-alvo | Resposta do chat (RAG) + aba Serviços |
| **FAQs** | Perguntas frequentes oficiais | Resposta do chat (RAG) + aba FAQs |
| **Pareceres** | Pareceres jurídicos/técnicos | Aba Pareceres (referência) |
| **Notas** | Notas administrativas/tributárias | Aba Notas (referência) |
| **Conteúdos** | Materiais de referência diversos | Aba Conteúdos (referência) |

Observação de produto: hoje **Serviços e FAQs** alimentam diretamente as respostas do
assistente; **Pareceres, Notas e Conteúdos** estão disponíveis como navegação/consulta de
referência na interface.

---

## 5. Arquitetura em três partes

| Componente | Papel no produto |
| --- | --- |
| **auli-engine** (workspace) | Cérebro: o binário `auli` em dois modos — `auli server` recebe a pergunta, busca o contexto e gera a resposta (somente leitura); `auli update` vetoriza os conteúdos em pacotes. Três crates em camadas. |
| **auli-frontend** | Experiência do usuário: seleção de estado, chat e navegação pelos conteúdos |
| **auli-collections** | Abastecimento: coleta e padroniza os conteúdos oficiais que alimentam a busca |

> O backend é o workspace **auli-engine** (`vector-store` ← `auli-core` ← `auli-cli`); ver
> [auli_code.md](auli_code.md) §3.

---

## 6. Estado atual e direção

- **Funcionando hoje:** chat com RAG para o estado configurado, interface completa (chat +
  abas de referência + seleção de estado com mapa), coleta de Serviços e FAQs (RS) e Serviços
  (SC) e embeddings locais. (O servidor é público, sem auth nem banco — expõe só `/v1/health`,
  `/v1/question` e `/v1/{kind}/list`.)
- **Em evolução:** ampliação do estado SC (FAQs e demais conteúdos), coleta automatizada de
  Pareceres/Notas, e uso desses tipos também nas respostas do assistente.

Para o que é apenas modelado/planejado versus efetivamente ativo no código (rotas, fluxos de
autenticação, divergências entre os repositórios), consulte [auli_code.md](auli_code.md).
