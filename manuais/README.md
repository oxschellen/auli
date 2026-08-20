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

**Os totais acima são mantidos à mão nos quatro manuais.** Quando entrar um estado novo com acervo
de pareceres, todos precisam ser atualizados juntos. O total do TARF ficou de fora enquanto a coleta
corria — publicar um número que muda toda semana seria pior do que omiti-lo. Com a coleta encerrada,
ele entra na mesma regra dos demais: número exato, mantido à mão.

## Quando nenhum caminho estiver disponível

Duas alternativas que não dependem de configuração nenhuma:

- **<https://auli.com.br>** — a consulta direto no navegador.
- **A aba Downloads** do portal — o `.zip` do estado, com um `.md` por documento, que qualquer IA
  que aceite anexo consegue ler.
