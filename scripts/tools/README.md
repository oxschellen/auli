# `scripts/tools/`

O que **não** entra na operação do dia a dia. Para subir o site e a API, tudo o que se usa está um
nível acima, em `scripts/` — e o critério deste diretório é exatamente esse: se não é preciso para
publicar ou servir, mora aqui.

| script | quando se usa |
| ------ | ------------- |
| `atualizar-servicos.sh <id>...` | re-raspar os serviços de uma entidade quando o portal muda |
| `update-rs-faqs.sh` | re-raspar as FAQs do RS de ponta a ponta |
| `gen-frontend-entities.mjs` | depois de editar `config/registry.toml` |
| `mcp-smoke.sh` | verificar o protocolo MCP (handshake + as 4 ferramentas) |
| `parity-replay.py` | provar que uma mudança na recuperação não alterou o contexto RAG |
| `check-registry-sync.sh` | guarda de CI — registry × `entities.ts` |
| `check-scraper-boundary.sh` | guarda de CI — nada fora de `crates/scrapers/` toca o kit |

Os dois `check-*` são chamados pelos workflows em `.github/workflows/`; os outros cinco são rodados
à mão, quando a ocasião pede.

**Ao mover um script para cá ou daqui:** os que resolvem caminhos calculam a raiz do repositório
como `dirname($0)/../..` — de `scripts/` seria `/..`. Errar isso não dá erro de sintaxe nem falha
no ato; o script simplesmente procura `config/` e `data/` no lugar errado.
