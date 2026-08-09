# Sobre a Auli

## O que é a Auli?

A Auli é uma assistente virtual inteligente projetada para auxiliar servidores públicos tributários no atendimento aos contribuintes.

Utilizando tecnologias avançadas de inteligência artificial, a Auli responde perguntas sobre serviços públicos de forma natural e precisa, agilizando o atendimento e tornando a informação pública mais acessível.

Trata-se de um projeto piloto desenvolvido com o objetivo de explorar aplicações práticas de busca semântica, bancos de dados vetoriais e geração de respostas com modelos de linguagem em cenários reais do serviço público.

A plataforma é multi-órgão: uma mesma instância pode atender diferentes entidades públicas, cada uma com sua própria base de conhecimento e instruções específicas, mantendo os dados completamente isolados.

Os conteúdos utilizados pela Auli são extraídos exclusivamente de informações e documentos públicos disponibilizados nos sites oficiais das administrações tributárias: serviços, perguntas frequentes, pareceres/respostas a consultas tributárias e acórdãos do contencioso administrativo fiscal.

## Aviso importante

A Auli é um **protótipo experimental**. As respostas são geradas automaticamente por inteligência artificial: podem variar de uma consulta para outra e, eventualmente, conter imprecisões ou omissões.

As informações fornecidas têm caráter **exclusivamente informativo** — não substituem a legislação vigente, os canais oficiais de atendimento nem a análise de um servidor responsável. Cabe a quem utiliza a ferramenta conferir as fontes e os links oficiais indicados em cada resposta antes de usar a informação.

## Como funciona?

A Auli utiliza um sistema avançado de RAG (Retrieval-Augmented Generation) que combina busca semântica com geração de linguagem natural:

Compreensão da pergunta: A pergunta é convertida em uma representação vetorial (embedding) de forma local, preservando a privacidade.

Busca inteligente: São recuperados os documentos mais relevantes da base de conhecimento da entidade (serviços públicos, FAQs, pareceres e acórdãos).

Resposta contextualizada: Os documentos recuperados são enviados como contexto para um modelo de linguagem, que gera uma resposta clara, precisa e alinhada às informações oficiais — sempre com os links das fontes.

## Consulte a Auli de dentro da sua IA

Além deste portal, o acervo pode ser consultado diretamente de dentro de assistentes de IA como ChatGPT, Claude e Copilot, por meio do protocolo **MCP** (Model Context Protocol). O servidor é público e não exige autenticação:

`https://api.auli.com.br/mcp`

Na aba **MCP** deste portal há um manual de instalação para cada assistente. Em alguns, o próprio usuário configura em poucos minutos; no Microsoft 365 Copilot, a configuração depende da área de TI da organização.

O conector expõe hoje o acervo de pareceres e respostas a consultas tributárias, os acórdãos do contencioso administrativo fiscal e a consulta de serviços de atendimento e perguntas frequentes — sempre com o link do documento na fonte oficial. Quem preferir levar os dados para a própria ferramenta pode usar a aba **Downloads**, que oferece o conteúdo de cada estado em um arquivo compactado, com um documento por arquivo.

## Privacidade

**A Auli não sabe quem você é.** Não há cadastro, login nem cookie de rastreamento, e o seu endereço IP não é gravado — nada liga uma consulta a uma pessoa, a um local ou a um histórico individual. O navegador guarda apenas a sua preferência de estado, que nunca é enviada ao servidor.

**O que fica registrado:** a pergunta, o horário e os documentos consultados. Serve para auditar a qualidade das respostas e o mascaramento automático descrito abaixo. Como a pergunta é guardada exatamente como você a escreveu, **dados pessoais que você mesmo digitar ficam gravados** — se incluir um CPF ou CNPJ, ele fica no registro. Evite informar dados que não sejam necessários à dúvida.

**O que não sai daqui:** antes de a pergunta ser enviada ao modelo de linguagem que redige a resposta, identificadores como CPF, CNPJ, telefone e e-mail são substituídos automaticamente por marcadores. O provedor do modelo não recebe esses dados. No conector MCP, nenhum modelo externo é chamado — a pergunta não sai do servidor.

## Tecnologia

A Auli é construída com tecnologias modernas e de código aberto:

Backend em Rust  
Busca semântica com banco vetorial próprio, embutido no servidor  
Embeddings gerados localmente com o modelo BGE-M3  
LLM para geração das respostas  
API de recuperação e servidor MCP para integração com outras ferramentas de IA

## Projeto aberto e colaborativo

A Auli é um projeto **de código aberto e colaborativo, sem fins comerciais**.
Todo o código é distribuído sob a **licença MIT** e está publicado no GitHub,
livre para ser usado, estudado, adaptado e aprimorado pela comunidade:

- **Código:** [github.com/oxschellen/auli](https://github.com/oxschellen/auli)

Contribuições são bem-vindas.

## Contato

Dúvidas, sugestões ou interesse em colaborar? Entre em contato pelo e-mail
[oxschellen@gmail.com](mailto:oxschellen@gmail.com).
