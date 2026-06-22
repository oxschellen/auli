# auli-server

> **Refatorado.** Este monólito foi reorganizado no workspace Cargo único `../auli/`
> (`vector-store` ← `auli-core` ← `auli-cli`; binário `auli` com os modos `server` / `update`),
> com a lógica preservada verbatim. Este diretório é mantido como **baseline de referência**.
> Backend atual: `../auli/README.md` e `../auli_code.md` §9.

Um servidor REST API em Rust que alimenta um aplicativo (projeto piloto) de assistente virtual ([auli.com.br](https://auli.com.br)). Ele responde perguntas sobre impostos estaduais, FAQs, pareceres jurídicos e notas administrativas usando um pipeline de RAG (Geração Aumentada por Recuperação).

É **multi-tenant**: um único servidor atende várias entidades (estados), cada uma com seus próprios dados e prompt de sistema. Veja [Adicionando uma Nova Entidade](#adicionando-uma-nova-entidade-estado).

> Documentação técnica completa (arquitetura, módulos, fluxo, ressalvas): [CLAUDE.md](CLAUDE.md).

---

## Visão Geral da Arquitetura

```text
Usuário / Web UI  →  Axum HTTP API  →  Vetor in-process (busca vetorial)  →  LLM API (geração de respostas)
                           │              coleções por entidade: <id>-services, <id>-faqs, …
                     PostgreSQL (autenticação)
                           │
                      fastembed (embeddings in-process)
```

O servidor é totalmente assíncrono, construído com **Tokio** e **Axum**, e expõe uma API REST versionada sob `/v1/`.

### Estrutura de Entidades

Cada entidade (estado) fica em `./entities/<id>/`. As coleções vetoriais são isoladas por entidade no formato `<id>-<tipo>` (ex.: `rs-faqs`, `rs-services`), persistidas como `<id>-<tipo>.json` sob `./vectors`.

```text
entities/
└── rs/
    ├── entity.json          { "id": "rs", "name": "SEFAZ-RS" }
    ├── prompt.txt           instruções de sistema do LLM desta entidade
    ├── portal-servicos.txt  dados de origem (serviços)
    ├── portal-faqs.txt      dados de origem (faqs)
    ├── portal-pareceres.txt dados de origem (pareceres)
    └── portal-notas.txt     dados de origem (notas)
```

---

## Funcionalidades Principais

| Funcionalidade | Descrição |
| --- | --- |
| **Multi-tenant (entidades)** | Um único servidor atende várias entidades (estados). Cada entidade fica em `entities/<id>/` com seus próprios dados e prompt; as coleções vetoriais são isoladas por entidade (`<id>-faqs`, `<id>-services`, …) |
| **RAG Q&A** | Busca semântica nas coleções vetoriais in-process, seguida de geração de resposta via LLM |
| **Tipos de Conteúdo** | Serviços · FAQs · Pareceres · Notas (a consulta unificada combina Serviços + FAQs) |
| **Embeddings Locais** | `fastembed` (BGE-M3, ONNX INT8) roda **in-process**, sem serviço externo |
| **LLM Externo** | Configurável via `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` (ex: Groq) |
| **Autenticação JWT** | Tokens de acesso assinados com RSA (expiração configurável). Usuários precisam existir no PostgreSQL e estar verificados |
| **Gestão de Usuários** | Cadastro cria usuário pendente de verificação; login gera JWT apenas para usuários verificados. Refresh tokens e redefinição de senha estão modelados, mas ainda inativos |
| **Ingestão de Dados** | Carrega conteúdo de arquivos locais ou via web scraping para as coleções vetoriais in-process |
| **CORS** | Pré-configurado para `auli.com.br` e origens comuns de desenvolvimento local |

---

## Endpoints da API

### Públicos

| Método | Caminho | Descrição |
| --- | --- | --- |
| `GET` | `/v1/health` | Verificação de saúde |
| `POST` | `/v1/question` | Enviar uma pergunta em linguagem natural |
| `POST` | `/v1/signin` | Autenticar usuário verificado e receber JWT |
| `POST` | `/register` | Cadastrar usuário pendente de verificação |
| `GET` | `/v1/gen_rsa_keypair` | Gerar um par de chaves RSA |

### Protegidos (JWT obrigatório)

| Método | Caminho | Descrição |
| --- | --- | --- |
| `GET` | `/v1/protected` | Obter informações do usuário autenticado |
| `POST` | `/v1/protected_question` | Enviar uma pergunta como usuário autenticado |

### Gerenciamento de Dados

Rotas genéricas por tipo de conteúdo, onde `{kind}` ∈ `services | faqs | pareceres | notas`. Todas aceitam o parâmetro opcional `?entity=<id>` (padrão: `rs`).
Essas rotas exigem `Authorization: Bearer <token>` de um usuário verificado.

| Método | Caminho | Descrição |
| --- | --- | --- |
| `GET` | `/v1/{kind}/list` | Listar os registros armazenados de um tipo |
| `GET` | `/v1/{kind}/load_from_file` | Ingerir um tipo a partir do arquivo da entidade |
| `POST` | `/v1/{kind}/load_from_web` | Ingerir um tipo a partir de texto enviado no corpo |

Exemplos: `GET /v1/faqs/list?entity=rs`, `GET /v1/services/load_from_file?entity=rs`.

### Corpo da Requisição de Pergunta

```json
{ "question": "Como faço para obter certidão negativa?", "entity": "rs" }
```

O campo `entity` é opcional (padrão: `rs`). Ele seleciona a entidade (estado) a ser
consultada. Nas rotas GET de gestão de dados, use o parâmetro `?entity=<id>`.

---

## Stack Tecnológica

| Componente | Tecnologia |
| --- | --- |
| Linguagem | Rust (edição 2021) |
| Framework Web | Axum 0.8 |
| Runtime Assíncrono | Tokio |
| Banco de Dados | PostgreSQL via SQLx |
| Armazenamento Vetorial | In-process (Rust, índice plano por cosseno, persistido em JSON sob `./vectors`) |
| Embeddings Locais | `fastembed` (BGE-M3 ONNX, in-process) |
| LLM | API HTTP externa (compatível com Groq) |
| Autenticação | JWT (RSA) + hash de senha Argon2 / bcrypt |

---

## Variáveis de Ambiente

| Variável | Descrição |
| --- | --- |
| `DATABASE_URL` | String de conexão do PostgreSQL |
| `JWT_SECRET` | Segredo HMAC para assinatura JWT |
| `JWT_RSA_PRIVATE_KEY` | Chave privada RSA (PEM) |
| `JWT_RSA_PUBLIC_KEY` | Chave pública RSA (PEM) |
| `JWT_ACCESS_EXPIRATION_MINUTES` | TTL do token de acesso (padrão: 15) |
| `JWT_REFRESH_EXPIRATION_DAYS` | TTL do refresh token (padrão: 7) |
| `LLM_API_URL` | Endpoint de inferência do LLM |
| `LLM_API_KEY` | Chave de API do LLM |
| `LLM_API_MODEL` | Nome do modelo LLM |
| `EMBED_CACHE_DIR` | Diretório de cache do modelo ONNX (padrão `./models`) |
| `EMBED_THREADS` | Threads intra-op do ONNX Runtime (padrão 16) |
| `VECTOR_DB_PATH` | Diretório do vector store in-process (padrão `./vectors`) |

---

## Executando Localmente

**Pré-requisitos:** Rust, PostgreSQL, (opcionalmente) ngrok. O armazenamento vetorial **e** os
embeddings (`fastembed`/BGE-M3) são in-process — não há serviço separado a iniciar.

```bash
# Iniciar tudo de uma vez (servidor + túnel ngrok)
./scripts/start_all.sh
```

Este script abre dois terminais:

- **Terminal 1** — compila e executa o servidor Rust (o vetor in-process sobe junto)
- **Terminal 2** — inicia um túnel ngrok para `api.auli.com.br:3000`

Para um ambiente headless/WSL (sem ngrok, no próprio terminal), use:

```bash
# Derruba a instância antiga, recompila (--release) e sobe em :3000
./scripts/start_local.sh
```

**Passos manuais:**

```bash
# Migrações do banco de dados
sqlx migrate run

# Compilar e executar
cargo build --release
./target/release/auli-server
```

O servidor escuta na porta **3000** por padrão.

---

## Migrações do Banco de Dados

Localizadas em `migrations/`, aplicadas em ordem:

1. `create_users` — tabela de contas de usuário
2. `create_refresh_tokens` — armazenamento de refresh tokens
3. `create_verification_tokens` — verificação de e-mail
4. `create_password_reset_tokens` — fluxo de redefinição de senha

## Adicionando uma Nova Entidade (Estado)

1. Crie o diretório `entities/<id>/` (ex.: `entities/sc/`).
2. Adicione `entity.json`: `{ "id": "sc", "name": "SEFAZ-SC" }`.
3. Adicione `prompt.txt` com as instruções de sistema do LLM para a entidade.
4. Adicione os arquivos de dados: `portal-servicos.txt`, `portal-faqs.txt`,
   `portal-pareceres.txt`, `portal-notas.txt`.
5. Reinicie o servidor (a lista de entidades é carregada no startup).
6. Ingira os dados: `GET /v1/faqs/load_from_file?entity=sc`,
   `GET /v1/services/load_from_file?entity=sc`, etc.
7. Consulte: `POST /v1/question` com `{ "entity": "sc", "question": "..." }`.

As coleções vetoriais ficam isoladas por entidade (`sc-faqs`, `sc-services`, …) sob `./vectors`.

---

## Licença

Este projeto está licenciado sob a **licença MIT**.  
Veja o arquivo [LICENSE](LICENSE) para mais detalhes.

---
