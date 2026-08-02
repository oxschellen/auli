# Como consultar o acervo do Auli dentro do ChatGPT

Manual para quem nunca configurou um conector. Leva uns cinco minutos.

## O que você vai conseguir fazer

Depois de configurar, você pergunta em português normal no ChatGPT — *"o que a SEFAZ de Santa
Catarina já respondeu sobre crédito de ICMS na energia elétrica?"* — e ele consulta o acervo de
pareceres tributários das secretarias estaduais da fazenda, com o link oficial de cada documento.

Hoje o acervo tem **19.780 pareceres** de quatro estados:

| Estado | Secretaria | Pareceres |
|---|---|---|
| São Paulo | SEFAZ-SP | 15.605 |
| Paraná | SEFA-PR | 2.060 |
| Santa Catarina | SEF-SC | 1.743 |
| Rio Grande do Sul | SEFAZ-RS | 372 |

## Antes de começar

Você precisa de uma **conta paga do ChatGPT** (Plus, Pro, Business ou Enterprise). No plano
gratuito não é possível adicionar conectores.

Não precisa instalar nada, nem criar login, nem senha. O acervo é público.

## Passo a passo

**1.** Abra o ChatGPT no navegador do computador: <https://chatgpt.com>

O celular não serve para *configurar* — mas depois de configurado, funciona no celular também.

**2.** Clique na sua foto/inicial no canto superior direito e escolha **Configurações**.

**3.** No menu lateral, procure **Conectores** (em algumas versões aparece como *Aplicativos e
conectores*).

**4.** Procure a opção de **modo desenvolvedor** — costuma estar em *Avançado* ou como uma chave
no fim da página — e **ligue**. É o que libera adicionar um conector próprio.

**5.** Volte para Conectores e clique em **Criar** (ou *Adicionar conector personalizado*).

**6.** Preencha assim:

- **Nome:** `Auli`
- **Descrição:** `Pareceres tributários das secretarias estaduais da fazenda`
- **URL do servidor MCP:**

  ```text
  https://api.auli.com.br/mcp
  ```

- **Autenticação:** escolha **Nenhuma** (ou *No authentication*). Não há usuário nem senha.

**7.** Salve. Se aparecer um aviso pedindo para você confirmar que confia no servidor, aceite — é
o padrão do ChatGPT para qualquer conector que não seja da própria OpenAI.

**8.** Abra uma conversa nova. Perto do campo de digitar, no botão de **+** ou de ferramentas,
ative o conector **Auli**.

Pronto.

## Teste se funcionou

Numa conversa nova, com o conector ligado, escreva:

> Quais estados têm pareceres disponíveis no Auli?

A resposta deve listar os quatro estados com as quantidades da tabela acima. Se listou, está
funcionando.

## Como perguntar bem

O acervo tem duas frentes: **pareceres e consultas tributárias** — respostas que as secretarias
deram a perguntas de contribuintes — e os **serviços de atendimento e perguntas frequentes** dos
portais, para dúvidas de "como fazer". Funciona melhor quando você diz **o estado** e **o tema**.

Bons exemplos:

> Em Santa Catarina, o que já foi decidido sobre crédito de ICMS na energia elétrica usada na
> industrialização?

> Procure pareceres de São Paulo sobre substituição tributária em autopeças.

> No Rio Grande do Sul, há parecer sobre isenção de ICMS no transporte de cargas? Me mostre o
> texto completo do mais relevante.

> Em Minas Gerais, como faço para emitir a segunda via da guia do IPVA?

Duas dicas que mudam muito o resultado:

- **Diga o estado.** Sem isso o ChatGPT pode escolher um por conta ou perguntar qual.
- **Peça o texto integral** quando quiser ler o documento. A busca devolve só o resumo; o texto
  completo vem por outra ferramenta, e o ChatGPT só a usa se você pedir.

## O que ele não faz

**Pareceres só dos quatro estados da tabela.** Se você pedir um parecer de Minas Gerais, não há.

**Serviços e perguntas frequentes cobrem estados diferentes.** Os serviços de atendimento existem
em quase todas as secretarias; as perguntas frequentes, hoje, só no Rio Grande do Sul. Pergunte
*"quais estados têm acervo no Auli, e o que cada um tem?"* — o ChatGPT consulta isso na hora, em vez
de você conferir numa lista que envelhece.

**Não é consultoria.** São documentos públicos, com o link oficial. Confira sempre no link antes
de usar em algo que importa — e lembre que parecer tem data: regra citada em 2014 pode ter mudado.

## Quando a resposta parecer estranha

**Vieram pareceres de assunto totalmente diferente.** Isso normalmente significa que **não existe
parecer sobre o que você perguntou**. A busca é por proximidade de sentido e devolve sempre os
mais próximos que encontrar — mesmo quando nenhum é bom. Se os resultados fogem do tema,
desconfie que o acervo não cobre aquilo, em vez de insistir na mesma pergunta.

**O conector não aparece na conversa.** Ele precisa ser ativado em cada conversa nova, no botão de
ferramentas ao lado do campo de digitar.

**Deu erro de conexão.** O serviço pode estar fora do ar no momento. Espere e tente de novo; se
persistir, avise quem passou este manual.

**Não encontro "modo desenvolvedor".** Os nomes dos menus do ChatGPT mudam de tempo em tempo e
variam por plano. Procure por *Conectores*, *Conectores personalizados* ou *MCP* nas Configurações;
o que você precisa é do campo onde se cola a URL do servidor.

## Resumo para colar

- URL: `https://api.auli.com.br/mcp`
- Autenticação: nenhuma
- Onde: ChatGPT → Configurações → Conectores → modo desenvolvedor → Criar
