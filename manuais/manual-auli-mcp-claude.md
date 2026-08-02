# Como consultar o acervo do Auli no Claude (web)

Manual para quem nunca configurou um conector. Leva uns três minutos — menos que no ChatGPT,
porque aqui não é preciso ligar modo desenvolvedor nenhum.

## O que você vai conseguir fazer

Depois de configurar, você pergunta em português normal — *"o que a SEFAZ de Santa Catarina já
respondeu sobre crédito de ICMS na energia elétrica?"* — e o Claude consulta o acervo de pareceres
tributários das secretarias estaduais da fazenda, com o link oficial de cada documento.

Hoje o acervo tem **19.780 pareceres** de quatro estados:

| Estado | Secretaria | Pareceres |
|---|---|---|
| São Paulo | SEFAZ-SP | 15.605 |
| Paraná | SEFA-PR | 2.060 |
| Santa Catarina | SEF-SC | 1.743 |
| Rio Grande do Sul | SEFAZ-RS | 372 |

## Antes de começar

Você precisa de um plano **Pro** ou **Max**. No plano gratuito não é possível adicionar conectores
personalizados.

**Se sua conta for Team ou Enterprise**, você não faz isso sozinho: um *Owner* da organização
precisa cadastrar o conector primeiro, em Configurações da Organização → Conectores → Adicionar →
Personalizado → Web. Depois disso cada pessoa conecta a sua conta.

Não precisa instalar nada, nem criar login, nem senha. O acervo é público.

## Passo a passo

**1.** Abra <https://claude.ai> no navegador do computador.

**2.** Vá em **Customize** (ou *Personalizar*, se a interface estiver em português) e escolha
**Connectors** / **Conectores**.

**3.** Clique no botão **+** e escolha **Add custom connector** / *Adicionar conector personalizado*.

**4.** Cole a URL do servidor:

```text
https://api.auli.com.br/mcp
```

**5.** **Não preencha as configurações avançadas.** Existe ali um campo de *OAuth Client ID* e
*Client Secret* — o Auli não usa autenticação, então deixe em branco.

**6.** Salve. O Claude vai mostrar um aviso de que conectores personalizados não são verificados
pela Anthropic. Isso é o padrão para qualquer servidor que não venha do diretório oficial dele.

**7.** Abra uma conversa nova. No canto inferior esquerdo, clique no **+**, escolha **Connectors**
e **ligue a chave do Auli**.

**8.** No seletor de modelo, escolha **Claude Fable 5**.

Pronto. Vale saber que o modelo e o conector são independentes: o conector funciona em qualquer
modelo, e trocar de modelo não desliga o conector.

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

- **Diga o estado.** É um parâmetro obrigatório da busca; sem ele o Claude tem de escolher um por
  conta ou perguntar qual.
- **Peça o texto integral** quando quiser ler o documento. A busca devolve só o resumo; o texto
  completo vem por outra ferramenta, e o Claude só a usa se você pedir.

## O que ele não faz

**Pareceres só dos quatro estados da tabela.** Se você pedir um parecer de Minas Gerais, não há.

**Serviços e perguntas frequentes cobrem estados diferentes.** Os serviços de atendimento existem
em quase todas as secretarias; as perguntas frequentes, hoje, só no Rio Grande do Sul. Pergunte
*"quais estados têm acervo no Auli, e o que cada um tem?"* — o Claude consulta isso na hora, em vez
de você conferir numa lista que envelhece.

**Não é consultoria.** São documentos públicos, com o link oficial. Confira sempre no link antes
de usar em algo que importa — e lembre que parecer tem data: regra citada em 2014 pode ter mudado.

## Quando a resposta parecer estranha

**Vieram pareceres de assunto totalmente diferente.** Isso normalmente significa que **não existe
parecer sobre o que você perguntou**. A busca é por proximidade de sentido e devolve sempre os mais
próximos que encontrar — mesmo quando nenhum é bom. Se os resultados fogem do tema, desconfie que o
acervo não cobre aquilo, em vez de insistir na mesma pergunta.

**O conector não aparece na conversa.** Ele precisa ser ligado em cada conversa nova, no **+** ao
lado do campo de digitar.

**Deu erro de conexão.** O Claude acessa o servidor **pela nuvem da Anthropic**, não pelo seu
computador — então não adianta estar na mesma rede. Se o serviço estiver fora do ar, espere e
tente de novo; se persistir, avise quem passou este manual.

**Não encontro o menu Conectores.** Os nomes mudam de tempo em tempo e variam por plano e idioma.
Procure por *Connectors*, *Conectores* ou *MCP* nas configurações; o que você precisa é do campo
onde se cola a URL do servidor. Em conta Team ou Enterprise, o menu só aparece para o Owner.

## Resumo para colar

- URL: `https://api.auli.com.br/mcp`
- Autenticação: nenhuma (deixe as configurações avançadas em branco)
- Onde: claude.ai → Customize → Connectors → + → Add custom connector
- Ligar: **+** na conversa → Connectors → chave do Auli
