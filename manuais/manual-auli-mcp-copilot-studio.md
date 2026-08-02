# Auli no Microsoft 365 Copilot — manual para a TI

Este manual **não é para o usuário final**. É para quem tem acesso ao Copilot Studio e ao centro de
administração do Microsoft 365, porque no Microsoft 365 Copilot não existe — e provavelmente nunca
vai existir — uma tela onde o usuário cola a URL de um servidor MCP e sai usando.

No ChatGPT e no Claude, o próprio usuário configura o conector em cinco minutos. Aqui não: a
extensão do Copilot passa obrigatoriamente por um **agente publicado no tenant**, que alguém precisa
construir, submeter e ter aprovado. Os manuais dos outros dois assistentes estão em
[manual-auli-mcp-chatgpt.md](manual-auli-mcp-chatgpt.md) e
[manual-auli-mcp-claude.md](manual-auli-mcp-claude.md).

O resultado final, para o usuário, é um agente **Auli** que ele chama por `@` dentro do mesmo chat do
Copilot que já usa no Edge.

## Ficha técnica do servidor

| | |
|---|---|
| URL | `https://api.auli.com.br/mcp` |
| Transporte | **Streamable HTTP** — é o único que o Copilot Studio aceita (SSE saiu de cena em 2025) |
| Autenticação | **Nenhuma**. Não há chave, não há OAuth, não há usuário |
| Acervo | 19.780 pareceres tributários de SEFAZ-SP (15.605), SEFA-PR (2.060), SEF-SC (1.743) e SEFAZ-RS (372) |
| Origem dos dados | Documentos públicos das secretarias estaduais da fazenda, com link oficial em cada resultado |

Três ferramentas são expostas:

| Ferramenta | O que faz |
|---|---|
| `listar_entidades` | Lista as UFs com acervo e o total de documentos de cada uma |
| `buscar_pareceres` | Busca semântica numa UF. Devolve número, ementa, sinopse, link oficial e score — **não** devolve o corpo |
| `obter_parecer` | Devolve o corpo integral de um parecer, dado a UF e o número exato |

**O que trafega para fora do tenant:** o texto da pergunta e a sigla da UF. Nada mais — não há
upload de documento, não há contexto do Microsoft 365, não há identificação do usuário. **O que o
servidor registra em log:** apenas metadados (UF, quantidade de resultados, tempo de resposta). O
texto da pergunta nunca é gravado; isso é regra do código, não configuração.

## Pré-requisitos

1. **Licença de criador do Copilot Studio** para quem vai montar o agente.
2. **Orquestração generativa ligada** no agente. Sem ela o MCP simplesmente não funciona — é
   requisito explícito da Microsoft, não recomendação.
3. **Política de DLP do Power Platform que permita o conector.** Este é o portão de verdade. O
   Copilot Studio conecta em servidores MCP **através de conectores do Power Platform**, então
   qualquer política de dados que classifique conectores também governa este acesso. Em tenant de
   órgão público, conectores para fora costumam estar no grupo bloqueado por padrão.

O item 3 é o que costuma travar tudo, e não se resolve no Copilot Studio: resolve-se no *Power
Platform admin center*, em Políticas de dados.

## Caminho A — assistente de MCP no Copilot Studio

O caminho normal, e o mais rápido.

**1.** Em <https://copilotstudio.microsoft.com>, abra o agente (ou crie um novo).

**2.** Vá em **Tools** → **Add a tool** → **New tool** → **Model Context Protocol**.

**3.** Preencha:

- **Server name:** `Auli`
- **Server URL:** `https://api.auli.com.br/mcp`
- **Server description:** este campo **não é decorativo** — é com ele que o orquestrador decide se
  chama o servidor. Sugestão:

  > Acervo de pareceres e consultas tributárias das secretarias estaduais da fazenda do Brasil
  > (SP, PR, SC e RS). Use para perguntas sobre entendimento do fisco estadual em ICMS, ITCMD e
  > demais tributos estaduais. Cada resultado traz o link do documento oficial. A busca exige a UF.

**4.** Em autenticação, escolha **None** e clique em **Create**.

**5.** Em **Add tool**, escolha **Create a new connection** e depois **Add to agent**.

**6.** Confirme que a **orquestração generativa** está ligada nas configurações do agente.

**7.** Teste no painel lateral: *"Quais estados têm pareceres disponíveis no Auli?"* — a resposta
deve listar as quatro UFs com as quantidades da ficha técnica acima.

### Publicar para a organização

**8.** **Publish** no agente (é preciso publicar ao menos uma vez antes de conectar canais).

**9.** Conecte os canais **Teams e Microsoft 365 Copilot**.

**10.** Escolha **Show to everyone in my org** e **Submit for admin approval**. O formulário que
aparece alimenta a entrada no catálogo da organização.

**11.** Um administrador aprova em **Integrated apps**, no centro de administração do Microsoft 365.
Aprovado, o agente aparece na coleção *Built by your org* do Agent Store, e o usuário passa a chamar
`@Auli` no chat do Copilot.

## Caminho B — registrar o servidor no tenant (BYO MCP, preview)

Se a TI preferir governar o servidor centralmente — com aprovação formal, bloqueio a qualquer
momento e telemetria no Defender — existe o **Bring Your Own MCP server** do Agent 365. Duas
ressalvas antes: está **em preview**, e **não é suportado em agentes declarativos** do Microsoft 365
Copilot (só Copilot Studio, VS Code, Claude Code e GitHub Copilot CLI). Ou seja: mesmo por este
caminho, quem entrega a experiência ao usuário final continua sendo um agente do Copilot Studio.

O registro é feito por um desenvolvedor, com a CLI do Agent 365:

```bash
a365 develop-mcp register-external-mcp-server \
  --server-name "Auli" \
  --server-url "https://api.auli.com.br/mcp" \
  --publisher "Auli" \
  --description "Pareceres tributários das secretarias estaduais da fazenda (SP, PR, SC, RS)" \
  --auth-type "NoAuth" \
  --tools "listar_entidades,buscar_pareceres,obter_parecer"
```

Depois disso, um **AI admin** ou **Global admin** revisa em **Agents → Tools → Requests** no centro
de administração, aprova, e concede o consentimento do Entra. Só então o servidor aparece no
registro — e pode levar até 30 minutos para propagar em todos os ambientes do Copilot Studio.

Vale saber que a Microsoft **não suporta excluir** um servidor BYO depois de registrado; o que existe
é bloquear.

## Caminho C — conector personalizado, se a DLP exigir

Alguns tenants exigem que todo acesso externo passe por um conector personalizado revisado, em vez
do assistente. Nesse caso, importe este OpenAPI no Power Apps (**Custom connectors** → *New custom
connector* → *Import OpenAPI file*):

```yaml
swagger: '2.0'
info:
  title: Auli
  description: Pareceres tributários das secretarias estaduais da fazenda
  version: 1.0.0
host: api.auli.com.br
basePath: /
schemes:
  - https
paths:
  /mcp:
    post:
      summary: Auli MCP Server
      x-ms-agentic-protocol: mcp-streamable-1.0
      operationId: InvokeMCP
      responses:
        '200':
          description: Success
```

A linha que importa é `x-ms-agentic-protocol: mcp-streamable-1.0` — é ela que diz ao Power Platform
que o endpoint fala MCP por transporte streamable.

## Texto sugerido para o pedido à TI

> Solicito a liberação de um servidor MCP externo para uso no Microsoft 365 Copilot.
>
> **Endereço:** `https://api.auli.com.br/mcp` — **Autenticação:** nenhuma — **Transporte:**
> streamable HTTP.
>
> **O que é:** acervo de consulta a 19.780 pareceres e consultas tributárias publicados pelas
> secretarias estaduais da fazenda de SP, PR, SC e RS. Todo o conteúdo é público e cada resultado
> devolve o link do documento na fonte oficial.
>
> **Sentido do fluxo:** apenas leitura. O servidor não recebe documento, não acessa dados do
> Microsoft 365 e não identifica o usuário. O que sai do tenant é o texto da pergunta e a sigla da
> UF consultada; do lado do servidor, apenas metadados de uso são registrados — o texto da pergunta
> não é gravado.
>
> **O que peço:** (a) liberar o conector correspondente na política de dados do Power Platform e
> (b) aprovar, em Integrated apps, o agente que será submetido pelo Copilot Studio.

## Quando não funcionar

**O agente responde sem consultar nada.** Quase sempre é a orquestração generativa desligada, ou a
descrição do servidor genérica demais para o orquestrador escolhê-lo. Melhore a descrição antes de
mexer em qualquer outra coisa.

**Erro de conexão ou o conector nem aparece.** Suspeite da política de DLP antes de suspeitar do
servidor. Para separar as duas coisas, teste o mesmo endereço no ChatGPT ou no Claude, seguindo os
manuais ao lado: se lá funciona, o servidor está de pé e o problema é de política.

**Aprovado, mas não aparece no Copilot Studio.** Propagação leva até 30 minutos.

**Vieram pareceres de assunto totalmente diferente.** Não é defeito de configuração: normalmente
significa que não existe parecer sobre aquilo. A busca é por proximidade de sentido e sempre devolve
os mais próximos que encontrar, mesmo quando nenhum é bom.

## Enquanto isso não anda

O usuário não precisa esperar a aprovação para usar o acervo. Duas coisas funcionam hoje, sem
depender de ninguém:

- **<https://auli.com.br>** — a consulta direto no navegador.
- **A aba Downloads** do portal — o `.zip` do estado, com um `.md` por documento, que qualquer IA
  que aceite anexo consegue ler.

## Resumo para colar

- URL: `https://api.auli.com.br/mcp` · Transporte: streamable HTTP · Autenticação: nenhuma
- Ferramentas: `listar_entidades`, `buscar_pareceres`, `obter_parecer`
- Onde: Copilot Studio → Tools → Add a tool → New tool → Model Context Protocol
- Obrigatório: orquestração generativa ligada + conector liberado na DLP do Power Platform
- Publicação: Publish → canais Teams e Microsoft 365 Copilot → Show to everyone in my org →
  Submit for admin approval → aprovação em Integrated apps
