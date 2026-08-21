# Manuais — como consultar o acervo do Auli em cada assistente

Todos apontam para o mesmo servidor, sem autenticação:

```text
https://api.auli.com.br/mcp
```

O que muda de um assistente para outro é **quem configura**: em alguns o próprio usuário resolve em
minutos; em outros, só a área de TI consegue.

| Manual | Para quem | Quem configura |
|---|---|---|
| [ChatGPT](manual-auli-mcp-chatgpt.md) | Usuário final | O próprio usuário — precisa de conta paga e do modo desenvolvedor ligado |
| [Claude (web)](manual-auli-mcp-claude.md) | Usuário final | O próprio usuário — plano Pro ou Max; em conta Team/Enterprise, um Owner cadastra antes |
| [GitHub Copilot](manual-auli-mcp-copilot.md) | Quem usa VS Code | O próprio usuário — só funciona no chat em modo Agente |
| [Microsoft 365 Copilot](manual-auli-mcp-copilot-studio.md) | Usuário do Copilot no Edge | **A TI.** Não existe caminho para o usuário — é preciso construir um agente no Copilot Studio e ter aprovação de administrador |

## Os dois "Copilot"

É a confusão mais comum, e vale checar antes de escolher o manual:

- **GitHub Copilot** — vive no editor de código. Aceita servidor MCP direto, você mesmo configura.
- **Microsoft 365 Copilot** — abre no Edge, com o login do trabalho e o aviso de dados protegidos da
  empresa. Não tem tela para colar URL de servidor; depende da TI.

Mesmo nome, empresas relacionadas, produtos diferentes.

## O que o acervo é, em qualquer um deles

19.780 pareceres e consultas tributárias de quatro secretarias estaduais da fazenda — SEFAZ-SP,
SEFA-PR, SEF-SC e SEFAZ-RS. Documentos públicos, cada resultado com o link da fonte oficial, que
prevalece sobre a cópia.

Cinco ferramentas: `listar_entidades` (o que cada UF tem), `buscar_pareceres` (busca por assunto,
exige a UF), `obter_parecer` (texto integral, pelo número), `consultar_servicos_faqs` (serviços de
atendimento e perguntas frequentes de uma UF, para dúvidas de "como fazer") e `buscar_legislacao`
(o texto da LEI em pares pergunta/resposta, com o dispositivo citado e o link oficial).

A divisão entre as três de conteúdo é a que o assistente precisa entender para escolher: a
**legislação** diz o que a norma determina; os **pareceres** dizem como a Receita interpreta a norma
num caso concreto; os **serviços/FAQs** dizem como fazer a coisa no portal.

As duas ferramentas de jurisprudência aceitam um parâmetro `colecao`: sem ele, consultam os **pareceres** (interpretação da legislação pela Receita Estadual); com `"tarf"`, os **acórdãos do TARF** do RS — decisões do tribunal administrativo em recursos de contribuintes, caso a caso, que não valem como norma geral. São 22.476 acórdãos, coleta encerrada em 09/08/2026.

A legislação são **509 pares pergunta/resposta**, em sete normas, hoje todas do Rio Grande do Sul:

| Norma | O que cobre |
|---|---|
| Constituição Federal | A parte tributária — competência, princípios, imunidades, ICMS, ITCMD e IPVA |
| Lei 6.537/1973 | Processo tributário administrativo do RS: infração, multa, impugnação, recurso, dívida ativa |
| Lei Complementar 123/2006 | Simples Nacional e MEI |
| Lei 8.821/1989 e Decreto 33.156/1989 | ITCD — o imposto sobre herança e doação — e o regulamento dele |
| Lei 8.115/1985 e Decreto 32.144/1985 | IPVA e o regulamento dele |

Ela não tem total por norma aqui de propósito, pela mesma razão de `docs/REGISTRO-legislacao-perguntas.md`: manter sete contagens à mão em cinco arquivos é garantir que uma delas fique errada. O agregado se confere com `find data/rs/docs/legislacao -name '*.md' | wc -l`.

**Os totais acima são mantidos à mão nos quatro manuais.** Quando entrar um estado novo com acervo
de pareceres, todos precisam ser atualizados juntos — e o mesmo vale para a tabela das normas, que
aparece nos quatro e ganha uma linha a cada lei integrada. O total do TARF ficou de fora enquanto a
coleta corria — publicar um número que muda toda semana seria pior do que omiti-lo. Com a coleta
encerrada, ele entra na mesma regra dos demais: número exato, mantido à mão.

## Quando nenhum caminho estiver disponível

Duas alternativas que não dependem de configuração nenhuma:

- **<https://auli.com.br>** — a consulta direto no navegador.
- **A aba Downloads** do portal — o `.zip` do estado, com um `.md` por documento, que qualquer IA
  que aceite anexo consegue ler.
