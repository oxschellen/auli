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

Além dos pareceres, o RS tem os **acórdãos do TARF** — o Tribunal Administrativo de Recursos
Fiscais, segunda instância do contencioso tributário estadual. Para consultá-los, peça em
português mesmo (*"o que o TARF já decidiu sobre..."*); por baixo, a busca leva `colecao: "tarf"`.
São decisões de casos concretos, não interpretação normativa. São 22.476 acórdãos — o maior acervo
do projeto, maior que todos os pareceres somados.

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

## As cinco ferramentas, e como pedi-las

Você não chama ferramenta nenhuma: pergunta em português e o Claude escolhe qual usar. Mas ajuda
saber o que existe por baixo — são cinco, e a frase certa leva à ferramenta certa. Diga sempre **o
estado**: quatro das cinco ferramentas exigem a UF, e sem ela o Claude tem de escolher uma por conta
ou perguntar qual.

[//]: # (manter em sincronia com auli-frontend/src/pages/mcplist/McpList.tsx — FERRAMENTAS)

| Ferramenta | O que faz | Diga, por exemplo |
|---|---|---|
| `listar_entidades` | Quais estados têm acervo e o que cada um tem, com os totais. Sem parâmetros. | *"Quais estados têm acervo no Auli, e o que cada um tem?"* |
| `buscar_pareceres` | Busca por sentido na jurisprudência de uma UF. Devolve número, assunto, sinopse e link — **sem o corpo**. Aceita `top_k` (1 a 20, padrão 5) e `colecao`: `pareceres` (padrão) ou `tarf`. | *"Em Santa Catarina, o que já foi respondido sobre crédito de ICMS na energia elétrica?"* — ou *"O que o TARF do RS já decidiu sobre…"*. Quer mais? *"Traga os 15 mais próximos."* |
| `obter_parecer` | O texto integral de **um** documento, pelo número exato que a busca devolveu, na mesma coleção. O Claude só a usa se você pedir o texto completo. | *"Me mostre o texto completo do mais relevante."* — ou *"Abra a íntegra do segundo da lista."* |
| `consultar_servicos_faqs` | Para dúvidas de **como fazer**: até 5 serviços e 10 FAQs da UF mais próximos da pergunta, num bloco só. | *"No Rio Grande do Sul, como faço para emitir a segunda via da guia do IPVA?"* |
| `buscar_legislacao` | Para o que a **lei** diz: pares pergunta/resposta com o dispositivo citado e o link oficial. Disponível só onde `listar_entidades` o reporta. | *"O que a lei do RS exige de uma locadora para pagar a alíquota menor de IPVA?"* |

O encadeamento típico é `listar_entidades` → `buscar_pareceres` → `obter_parecer`: o Claude descobre
o que existe, busca, e só então abre a íntegra do que você escolheu.

## Como perguntar bem

São três frentes — a **legislação**, os **pareceres e consultas tributárias** e os **serviços de
atendimento e perguntas frequentes** dos portais —, e a busca funciona melhor quando você diz **o
estado** e **o tema**.

Bons exemplos:

> Em Santa Catarina, o que já foi decidido sobre crédito de ICMS na energia elétrica usada na
> industrialização?

> Procure pareceres de São Paulo sobre substituição tributária em autopeças.

> No Rio Grande do Sul, há parecer sobre isenção de ICMS no transporte de cargas? Me mostre o
> texto completo do mais relevante.

> Em Minas Gerais, como faço para emitir a segunda via da guia do IPVA?

> O que a lei do Rio Grande do Sul exige de uma locadora para ela pagar a alíquota menor de IPVA?

Duas dicas que mudam muito o resultado:

- **Diga o estado.** É um parâmetro obrigatório da busca; sem ele o Claude tem de escolher um por
  conta ou perguntar qual.
- **Peça o texto integral** quando quiser ler o documento. A busca devolve só o resumo; o texto
  completo vem por outra ferramenta, e o Claude só a usa se você pedir.

## O que ele não faz

**Pareceres só dos quatro estados da tabela.** Se você pedir um parecer de Minas Gerais, não há.

**Cada frente cobre estados diferentes.** Os serviços de atendimento existem em quase todas as
secretarias; as perguntas frequentes e a legislação, hoje, só no Rio Grande do Sul. Pergunte
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
