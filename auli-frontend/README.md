# Auli — Assistente Virtual para Impostos Estaduais

**Auli** é um projeto piloto, aberto e cooperativo, de um assistente virtual inteligente projetado para **auxiliar o servidor público no atendimento ao cidadão** sobre impostos estaduais no Brasil (com foco no Rio Grande do Sul).

O frontend permite que o usuário faça perguntas em **linguagem natural** e receba respostas geradas por IA, além de navegar pelo acervo de cada estado (Serviços, FAQs, Pareceres, Acórdãos TARF, Legislação, Notas e Conteúdos), pela curadoria do **Tributum** e pelas integrações (Conectar sua IA e Downloads).

O aplicativo é **multi-tenant por estado**: o usuário escolhe a Secretaria da Fazenda estadual (*entidade*) num mapa do Brasil, e cada estado aberto vira uma **aba de estado**, com a sua conversa, a sua seção e a sua rolagem. Hoje são **27 entidades**, uma por unidade da federação, geradas de `config/registry.toml`.

## Funcionalidades

- **Seleção de estado** num **mapa do Brasil interativo**; vários estados ficam abertos ao mesmo tempo, em **abas de estado** no cabeçalho, e as abas abertas são persistidas (`localStorage`)
- Chat conversacional com balões diferenciados para usuário e assistente
- Navegação por uma **barra lateral** (`Home`) com 17 seções em 4 grupos (Chat, Acervo, Tributum e Integrações), recolhível numa faixa de ícones (Ctrl+B) e num drawer no celular; cada seção, depois de aberta, fica montada para preservar estado (rolagem, busca, histórico)
- **Minhas perguntas**: as perguntas enviadas ficam no navegador, com favoritas, num painel à direita do chat
- Páginas de referência com conteúdo estático, servidas por estado de [public/](public/)`<estado>/` (JSON e texto), com busca no cliente; seções sem dados para o estado aparecem desativadas na barra lateral
- Botão de copiar mensagens com um clique
- **Modo claro/escuro** que respeita a preferência do sistema e persiste a escolha do usuário (alternador no cabeçalho)
- Layout **mobile-first** com ajuste inteligente ao teclado virtual (iOS/Android)
- Timeout de 70 segundos com mensagem amigável em caso de lentidão da API

## Tecnologias Utilizadas

- **TypeScript** (modo `strict`) — todo o código da aplicação é `.ts`/`.tsx`; apenas os 4 *snippets* utilitários do Chakra em `ui/` seguem como `.jsx`
- **React 19** + **Vite 8** (Rolldown)
- **Chakra UI v3** (`@chakra-ui/react`, `@emotion/react`)
- **SWR** + **Axios** (busca de dados)
- **Framer Motion** (animações)
- **react-markdown** (respostas do assistente e página Sobre)
- **react-icons**, **next-themes** (modo de cor)
- **Vitest** + **Testing Library** (testes)

> A navegação é pela barra lateral dentro de `Home` (single-page, sem roteador): toda a navegação é em memória e a URL permanece em `/`.

## Tema e cores

Todas as cores vêm de um único sistema de tokens semânticos em [`src/theme/system.js`](src/theme/system.js), com valores para o modo claro e escuro. Componentes nunca usam cores literais — veja [THEME.md](THEME.md) para o vocabulário de tokens e a regra de lint que garante isso.

## Como Rodar o Projeto

```bash
npm install      # instalar dependências
npm run dev      # servidor de desenvolvimento (Vite)
npm run build    # checagem de tipos (tsc --noEmit) + build de produção
npm run preview  # visualizar o build de produção
npm run lint     # ESLint sobre js/jsx/ts/tsx
npm run typecheck# apenas checagem de tipos
npm test         # executa a suíte de testes (Vitest)
npm run test:watch # Vitest em modo watch
```

O aplicativo consome a API backend da Auli em `https://api.auli.com.br/v1/question`
(POST `{ question, entity?, type }`, onde `entity` é o estado da aba e `type` o tipo de consulta; pode ser
sobrescrita com a variável de ambiente `VITE_API_URL`).

## Testes

Testes com **Vitest**: lógica pura (parsers, validação do prompt, `editLinks`,
`callServerAPI`) no ambiente Node e testes de renderização de componentes
(`SearchInput`, `AsyncContent`, `Input`, `Messages`) em **jsdom** via Testing
Library. Veja [DESCRIPTION.md](DESCRIPTION.md) para detalhes da arquitetura.
