# Como consultar o acervo do Auli no GitHub Copilot

Manual para quem nunca configurou um servidor MCP. Leva uns cinco minutos.

Aqui há uma diferença importante em relação ao ChatGPT e ao Claude: o Copilot vive **dentro de um
editor de código** — quase sempre o **VS Code**. Não existe conector para o Copilot do site do
GitHub nem para o autocompletar que sugere código enquanto você digita. O que este manual configura
é o **chat do Copilot no VS Code, em modo Agente**.

> **Atenção: existem dois "Copilot", e este manual serve para um só.**
>
> Se o seu Copilot é aquele que abre **dentro do Edge**, com o login do trabalho e um aviso de dados
> protegidos da empresa, ele é o **Microsoft 365 Copilot** — outro produto, da mesma empresa, com o
> mesmo nome. Nele **não dá** para colar a URL de um servidor MCP: a configuração é feita por quem
> administra o ambiente, e está em
> [manual-auli-mcp-copilot-studio.md](manual-auli-mcp-copilot-studio.md).
>
> Este manual aqui é do **GitHub Copilot**, o do editor de código.

## O que você vai conseguir fazer

Depois de configurar, você pergunta em português normal no chat do Copilot — *"o que a SEFAZ de
Santa Catarina já respondeu sobre crédito de ICMS na energia elétrica?"* — e ele consulta o acervo
de pareceres tributários das secretarias estaduais da fazenda, com o link oficial de cada documento.

Hoje o acervo tem **19.780 pareceres** de quatro estados:

| Estado | Secretaria | Pareceres |
|---|---|---|
| São Paulo | SEFAZ-SP | 15.605 |
| Paraná | SEFA-PR | 2.060 |
| Santa Catarina | SEF-SC | 1.743 |
| Rio Grande do Sul | SEFAZ-RS | 372 |

Há também a **legislação**: o texto da lei em pares pergunta/resposta, cada resposta com o
dispositivo citado (*"Art. 18-A, § 2º"*) e o link para a norma na fonte oficial. São **509 pares**
de sete normas, hoje só do Rio Grande do Sul:

| Norma | O que cobre |
|---|---|
| Constituição Federal | A parte tributária — competência, princípios, imunidades, ICMS, ITCMD e IPVA |
| Lei 6.537/1973 | Processo tributário administrativo do RS: infração, multa, impugnação, recurso, dívida ativa |
| Lei Complementar 123/2006 | Simples Nacional e MEI |
| Lei 8.821/1989 e Decreto 33.156/1989 | ITCD — o imposto sobre herança e doação — e o regulamento dele |
| Lei 8.115/1985 e Decreto 32.144/1985 | IPVA e o regulamento dele |

A distinção entre as três frentes é o que decide qual delas responde à sua pergunta: a
**legislação** diz o que a norma determina; os **pareceres** dizem como a Receita interpreta a
norma num caso concreto; os **serviços e perguntas frequentes** dizem como fazer a coisa no portal.

## Antes de começar

Você precisa de:

- **VS Code atualizado.** Se o seu for de mais de um ano atrás, atualize antes — o suporte a MCP é
  recente e mudou de lugar algumas vezes.
- **Uma conta com Copilot ativo.** Qualquer plano serve, inclusive o gratuito; o que muda entre eles
  é quantas requisições você tem por mês.

Se sua conta do Copilot for da **empresa** (Business ou Enterprise), pode ser que um administrador
precise liberar o uso de servidores MCP na organização. Se você seguir os passos e o servidor não
subir, é a primeira coisa a perguntar para quem administra.

Não precisa instalar nada além do VS Code, nem criar login, nem senha. O acervo é público.

## Passo a passo

**1.** Abra o VS Code.

**2.** Pressione `Ctrl+Shift+P` (no Mac, `Cmd+Shift+P`). Abre uma caixa de busca no topo da tela —
é a *paleta de comandos*.

**3.** Digite `MCP: Add Server` e escolha essa opção.

**4.** Escolha o tipo **HTTP** (aparece como *HTTP (HTTP or Server-Sent Events)*).

**5.** Cole a URL do servidor:

```text
https://api.auli.com.br/mcp
```

**6.** Dê o nome `auli` ao servidor, quando ele pedir.

**7.** Escolha **Global** (ou *User*) para valer em qualquer projeto que você abrir. A outra opção,
*Workspace*, deixa a configuração dentro da pasta do projeto — útil para compartilhar com um time,
desnecessário se você só quer usar.

**8.** O VS Code pergunta se você confia no servidor. Aceite. É o padrão para qualquer servidor MCP
que não venha de dentro do próprio editor.

**9.** Abra o chat do Copilot: `Ctrl+Alt+I` (no Mac, `Cmd+Ctrl+I`), ou o ícone de balão de conversa
na barra lateral.

**10.** No **seletor de modo**, logo acima ou abaixo do campo de digitar, escolha **Agent**.

Este passo não é opcional: nos modos *Ask* e *Edit* o Copilot **não usa ferramentas**, e o Auli
simplesmente não vai ser consultado.

**11.** Clique no botão de **ferramentas** (*Configure Tools*, ícone de chave inglesa) e confirme
que as cinco ferramentas do Auli estão marcadas:

- `listar_entidades` — quais estados têm acervo
- `buscar_pareceres` — busca por assunto (parâmetro `colecao`: `pareceres`, o padrão, ou `tarf`)
- `obter_parecer` — texto integral de um documento (mesmo `colecao` usado na busca)
- `consultar_servicos_faqs` — serviços de atendimento e perguntas frequentes
- `buscar_legislacao` — o texto da lei em pares pergunta/resposta, com o dispositivo e o link

Pronto.

## Se preferir editar o arquivo direto

Dá no mesmo, e é mais rápido de passar para outra pessoa. Na paleta de comandos, rode
**MCP: Open User Configuration** e cole:

```json
{
  "servers": {
    "auli": {
      "type": "http",
      "url": "https://api.auli.com.br/mcp"
    }
  }
}
```

Se o arquivo já tiver outros servidores, acrescente a entrada `"auli"` dentro do `"servers"` que já
existe — não crie um segundo bloco.

Para valer só em um projeto (e ir junto no Git, para o time todo), o mesmo conteúdo vai em
`.vscode/mcp.json`, na raiz do projeto.

## Teste se funcionou

No chat, em modo **Agent**, escreva:

> Quais estados têm pareceres disponíveis no Auli?

A resposta deve listar os quatro estados com as quantidades da tabela acima. Se listou, está
funcionando.

Na primeira vez o Copilot vai pedir sua autorização para rodar a ferramenta — clique em **Continue**
/ *Continuar*. Se você marcar a opção de permitir sempre, ele para de perguntar.

## Como perguntar bem

São três frentes — a **legislação**, os **pareceres e consultas tributárias** e os **serviços de
atendimento e perguntas frequentes** dos portais —, e a busca funciona melhor quando você diz **o
estado** e **o tema**.

Bons exemplos:

> Em Santa Catarina, o que já foi decidido sobre crédito de ICMS na energia elétrica usada na
> industrialização?

> Procure pareceres de São Paulo sobre substituição tributária em autopeças.

> No Rio Grande do Sul, há parecer sobre isenção de ICMS no transporte de cargas? Me mostre o texto
> completo do mais relevante.

> Em Minas Gerais, como faço para emitir a segunda via da guia do IPVA?

> O que a lei do Rio Grande do Sul exige de uma locadora para ela pagar a alíquota menor de IPVA?

Três dicas que mudam muito o resultado:

- **Diga o estado.** É um parâmetro obrigatório da busca; sem ele o Copilot tem de escolher um por
  conta ou perguntar qual.
- **Peça o texto integral** quando quiser ler o documento. A busca devolve só o resumo; o texto
  completo vem por outra ferramenta, e o Copilot só a usa se você pedir.
- **Diga que é para responder no chat**, se ele começar a querer criar arquivos. O Copilot em modo
  Agente é feito para mexer em código, e às vezes leva a pergunta para esse lado. *"Só me responda
  aqui, não crie arquivos"* resolve.

## O que ele não faz

**Pareceres só dos quatro estados da tabela.** Se você pedir um parecer de Minas Gerais, não há.

**Cada frente cobre estados diferentes.** Os serviços de atendimento existem em quase todas as
secretarias; as perguntas frequentes e a legislação, hoje, só no Rio Grande do Sul. Pergunte
*"quais estados têm acervo no Auli, e o que cada um tem?"* — o Copilot consulta isso na hora, em vez
de você conferir numa lista que envelhece.

**Não vale para o autocompletar.** As sugestões de código que aparecem enquanto você digita não
usam MCP. Só o chat em modo Agente usa.

**Não é consultoria.** São documentos públicos, com o link oficial. Confira sempre no link antes de
usar em algo que importa — e lembre que parecer tem data: regra citada em 2014 pode ter mudado.

## Quando a resposta parecer estranha

**Vieram pareceres de assunto totalmente diferente.** Isso normalmente significa que **não existe
parecer sobre o que você perguntou**. A busca é por proximidade de sentido e devolve sempre os mais
próximos que encontrar — mesmo quando nenhum é bom. Se os resultados fogem do tema, desconfie que o
acervo não cobre aquilo, em vez de insistir na mesma pergunta.

**Ele respondeu sem consultar nada.** Quase sempre é o modo errado: confira se está em **Agent**, e
não em *Ask*. Se estiver certo, peça explicitamente: *"use a ferramenta do Auli"*.

**As ferramentas do Auli não aparecem na lista.** O VS Code limita quantas ferramentas podem ficar
ligadas ao mesmo tempo. Se você tem vários servidores MCP instalados, desligue os que não estiver
usando naquele momento para abrir espaço.

**O servidor aparece como parado ou com erro.** Rode `MCP: List Servers` na paleta de comandos,
escolha o `auli` e peça para reiniciar; ali também dá para ver o log com o motivo do erro. Se o
serviço estiver fora do ar, espere e tente de novo; se persistir, avise quem passou este manual.

**Não encontro os comandos `MCP: ...`.** Seu VS Code está desatualizado, ou é uma variante que não
traz o Copilot (como o VSCodium). Atualize e tente de novo.

## Em outros editores

O Copilot também roda no Visual Studio, no JetBrains e no Xcode, e todos aceitam servidores MCP —
mas cada um guarda a configuração num lugar diferente. O que não muda é o conteúdo: tipo `http` e a
URL `https://api.auli.com.br/mcp`, sem autenticação. Procure por *MCP* nas configurações do editor e
preencha esses dois campos.

## Resumo para colar

- URL: `https://api.auli.com.br/mcp`
- Tipo: HTTP · Autenticação: nenhuma
- Onde: VS Code → `Ctrl+Shift+P` → **MCP: Add Server** → HTTP → URL → nome `auli` → Global
- Usar: chat do Copilot em modo **Agent** (nos modos Ask e Edit não funciona)
