# Estudo-servicos-descricao — a descrição do serviço num acordeão, na aba Serviços

**Data:** 14/11/2026 · **Status:** implementado em 14/11/2026 na branch `feat/servicos-descricao`.
Portões 1 a 3 da §8 aprovados, mais a escolha do ícone ao lado do botão. Os ajustes pedidos na
conferência no navegador e os desvios em relação ao texto estão na §11.
**Base:** `main` em `68a95b7` (14/11/2026, frontend v0.1.87).
**Leitura prévia:** `auli-frontend/src/secoes/servicos/` (os quatro arquivos),
`src/secoes/faqs/FaqsAccordion.tsx`, `src/ui/Acordeao.tsx`, `src/lib/linkify.tsx` e
`auli-server/crates/auli-collections/src/servicos/mod.rs` (passo 4 do `process`).

> Documento para o Claude Code. Trabalhe **um passo de cada vez** e confirme comigo (Carlos) nos
> portões marcados. Branch nova a partir do `main`; nada de push ou merge sem confirmação.
> Os trechos de código deste estudo são **esboço, não compilado**: o repositório vence.

---

## 1. O pedido e a resposta curta

Hoje a aba Serviços mostra, dentro de cada classe, só o título de cada serviço, e o título é um link
para o portal. O pedido: abrir a **descrição** do serviço ali mesmo, num acordeão.

A resposta curta: **a descrição já está no navegador.** Os JSON por público que a aba baixa
(`<id>-servicos-a-empresas.json` e irmãos) trazem o campo `descricao` com o corpo limpo, o mesmo
texto que vira o `## corpo` do `docs/servicos/*.md` e que o chat lê. O tipo `Servico` do frontend
só não o declara.

Logo a entrega cabe inteira no frontend: três arquivos de produção e os testes. Scraper,
`auli-collections`, contrato, packs, servidor e `build-frontend-public.sh` não mudam, e a aba não
baixa um byte a mais.

O que pede decisão não é o dado, é o comportamento: o que o clique na linha passa a fazer, o que
acontece com os serviços sem descrição e como o texto é desenhado. É o assunto das §§3 a 5.

## 2. O que o repositório já oferece, e o que ele impõe

| O que existe | Onde | O que isso decide aqui |
| --- | --- | --- |
| `descricao` gravado em cada entrada dos JSON por público ("corpo limpo do snapshot", D-S5) | `auli-collections/src/servicos/mod.rs`, passo 4; `ServicoPerPublico` no `auli-contract` | O dado está pronto e é contrato com teste (`servico_per_publico_serializes_with_stable_field_names`) |
| A aba baixa **todas** as abas de público de uma vez | `ServicosList.tsx` | Nenhum fetch novo, nem carga tardia por serviço |
| `interface Servico { id, classe, titulo, link }` | `servicos.ts` | Falta só declarar `descricao` |
| Acordeão de um nível: a classe abre uma pilha de `<Link>` | `ServicosAccordion.tsx` | É o arquivo que muda |
| `PainelRevelado` e `comoBotao`, já usados aninhados na árvore de FAQs | `ui/Acordeao.tsx`, `FaqsAccordion.tsx` | O segundo nível não precisa de peça nova |
| A folha das FAQs: linha que abre a resposta, com ícones de copiar e de abrir no portal | `FaqsAccordion.tsx` (`TreeNode`) | É o modelo de interação a seguir |
| A folha das FAQs sem resposta abre o link em vez de expandir | idem (`toggleOrOpenNode`) | Precedente para o serviço sem descrição |
| `editLinks`: texto puro com URLs viradas em link, usado pelas Notas com `pre-wrap` | `lib/linkify.tsx`, `NotasList.tsx` | Serve ao texto da descrição, com um conserto (§5.3) |
| Busca da aba: classe e título; durante a busca todos os grupos ficam abertos | `servicos.ts` (`filterServicoGroups`), `ServicosList.tsx` | Não muda nesta entrega (D-SVC-8) |
| O `id` é sequencial **por arquivo**; a identidade do serviço é o par `(titulo, link)` | `servicos.ts` (`contarServicosDistintos`) | A `key` da linha deixa de ser o `id` (§5.4) |

## 3. O que os dados dizem

Medido em 14/11/2026 sobre o que está publicado em `auli.com.br`: os 27 índices e os 71 JSON por
público, 6.203 entradas (uma por serviço × classe × público).

**Presença.** 17 das 27 entidades têm descrição em todas as entradas. No total, 834 entradas (13%)
não têm descrição aproveitável:

| Caso | Entradas | Onde |
| --- | --- | --- |
| `descricao` vazia | 799 | MS 647 (todas), RJ 91 (todas), PE 40 (todas), MA 11 de 38, RN 10 de 15 |
| `descricao` igual ao título | 35 | AM 14, CE 10, RO 9, PI 1, RS 1 |

Outras 1.144 têm menos de 80 caracteres (SP 580, CE 183, AM 177, RO 124): são curtas, mas dizem
algo ("Formas de acesso: Certificado Digital") e entram como estão.

**Tamanho.** Varia muito entre portais:

| Entidade | Mediana | p90 | Máximo |
| --- | --- | --- | --- |
| RS (635 entradas) | 446 | 1.370 | 5.077 |
| SC | 654 | 1.374 | 3.175 |
| SP | 76 | 139 | 311 |
| MG | 3.475 | 6.906 | 15.904 |
| PA | 1.036 | 2.683 | 14.698 |

**Formato.** É texto puro, não markdown. Nas 6.203 entradas não há um título `#`, um `**negrito**`,
um link `[x](y)`, uma tabela ou uma tag HTML. O que há:

- quebras de linha simples (`\n`) separando parágrafos e itens;
- itens começados por `- ` (SC 442, AL 166, MG 82, RS 34) ou por `•` (MG 33, RS 9);
- endereços citados entre aspas retas, `clique aqui "https://…"`, em 1.381 descrições (SC 423,
  MG 295, RS 212, BA 185, PR 184, PA 82), além de endereços soltos;
- endereços longos: 1.208 descrições têm ao menos uma URL com mais de 70 caracteres.

**Qualidade da extração.** A descrição é o que o scraper tirou do HTML, e mostrá-la expõe o que até
hoje só o RAG via. O caso mais visível é o "clique aqui" que perdeu o endereço: por heurística, da
ordem de 650 descrições, concentradas no DF (410) e na PB (142); no RS, 22. Não é assunto do
frontend (ver §10).

**Peso.** Nada novo é baixado. Hoje o RS já traz ~600 kB nas cinco abas e o maior é MG, com 1,4 MB;
esse texto chega ao navegador e é descartado.

**Tamanho das listas.** A maior classe tem 374 serviços (CE, que tem classe única), depois RO 193,
GO 100, SP 92, RS 51.

## 4. As decisões

| | O quê | Por quê |
| --- | --- | --- |
| **D-SVC-1** | A descrição vem do campo `descricao` dos JSON por público. Só o tipo `Servico` do frontend muda | O dado já está lá e é contrato. Zero efeito em packs, `docs_hash`, chat e deploy |
| **D-SVC-2** | Dois níveis: classe → serviço → descrição. Clicar na linha do serviço abre e fecha a descrição | É o pedido, no formato da folha das FAQs |
| **D-SVC-3** | O portal continua a um clique: um ícone "Abrir no portal" na linha, que é um `<a>` de verdade | Hoje o título é um `<a>`: um clique, Ctrl+clique e "copiar endereço do link" funcionam. Com `window.open` os dois últimos se perderiam |
| **D-SVC-4** | Serviço sem descrição (vazia, ou igual ao título) não vira acordeão: a linha continua sendo o link de hoje | 834 entradas. Abrir um painel vazio, ou que repete o título, é pior que o comportamento atual. MS, RJ e PE ficam exatamente como estão |
| **D-SVC-5** | A descrição é desenhada como texto puro: `white-space: pre-wrap` e `editLinks`. **Não** passa pelo `ReactMarkdown` | Não há markdown no acervo (§3). No markdown a quebra simples some, e os parágrafos do RS virariam um bloco só |
| **D-SVC-6** | O `editLinks` ganha dois consertos: a URL termina na aspa, e a limpeza do fim repete até estabilizar | Com a regra de hoje, 180 dos 5.892 endereços das descrições virariam link quebrado (§5.3). Com os dois consertos, nenhum |
| **D-SVC-7** | O "aberto" é estado local da linha; várias podem ficar abertas; a `key` passa a ser `titulo + NUL + link` | Igual a FAQs e Pareceres. O `id` reinicia em cada arquivo, então a mesma `key` é outro serviço na outra aba (§5.4) |
| **D-SVC-8** | A busca não muda: classe e título | Pôr a descrição na busca muda o que a lista devolve e pede outro desenho (§10) |
| **D-SVC-9** | Sem teto de altura nem "ver mais" no painel | O acordeão já é o recolher. As respostas das FAQs também não têm teto |

## 5. O desenho

### 5.1 A linha

```text
▾ Atendimento - Site, e-CAC, PPF e DT-e                      (classe, como hoje)
  ┌──────────────────────────────────────────────────────────────────────┐
  │ Agendamento para Atendimento Presencial                      ↗   ⌄  │
  │ Comunicação por WhatsApp                                     ↗   ⌃  │
  │   O Whatsapp da Receita Estadual é utilizado para o envio de…       │
  │   Exemplos de mensagens que podem ser encaminhadas: a) …            │
  │   b) atrasos de pagamento;                                           │
  │   … Fale Conosco "https://atendimento.receita.rs.gov.br/faleconosco"│
  │ Serviço sem descrição (link, como hoje)                              │
  └──────────────────────────────────────────────────────────────────────┘
```

- **Com descrição:** título em `fg` (não mais em `accent`, porque deixou de ser link), o ícone
  `MdOpenInNew` e o chevron à direita. A linha usa `comoBotao` e `aria-expanded`.
- **Sem descrição:** o mesmo `<Link>` em `accent` de hoje, sem chevron. A cor diz o que o clique
  faz: azul sai para o portal, neutro abre aqui.
- **Painel:** fundo `bg.app`, `0.95rem`, `line-height 1.7` (os valores da resposta das FAQs),
  recuado em relação ao título.

Esboço do item, dentro do `ServicosAccordion.tsx`:

```tsx
function ServicoItem({ s, terms }: { s: Servico; terms: string[] }) {
  const [aberto, setAberto] = useState(false);
  if (!temDescricao(s)) return <LinkDeHoje s={s} terms={terms} />; // o <Link> atual, intacto
  return (
    <Box>
      <Flex {...comoBotao(() => setAberto((a) => !a))} aria-expanded={aberto} /* … */>
        <Text flex={1}><Highlight text={s.titulo} terms={terms} /></Text>
        <Link
          href={s.link} target="_blank" rel="noopener noreferrer"
          aria-label={`Abrir ${s.titulo} no portal oficial`}
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => e.stopPropagation()} // ver a nota abaixo
        >
          <MdOpenInNew size={14} />
        </Link>
        {aberto ? <MdExpandLess size={20} /> : <MdExpandMore size={20} />}
      </Flex>
      <PainelRevelado aberto={aberto}>
        <Box whiteSpace="pre-wrap" overflowWrap="anywhere" /* … */>
          {editLinks(s.descricao)}
        </Box>
      </PainelRevelado>
    </Box>
  );
}
```

**Nota sobre o teclado.** O `comoBotao` trata Enter e Espaço no `onKeyDown` da linha e chama
`preventDefault`. O evento de um elemento focável **dentro** da linha sobe até ela; sem o
`stopPropagation` no `onKeyDown` do ícone, o Enter sobre o ícone abriria o acordeão em vez de
seguir o link. Pela leitura do código, os ícones "Copiar resposta" e "Abrir no portal" da folha das
FAQs têm esse mesmo caminho hoje. Não testei em navegador, e fica **fora desta entrega**: aqui só
registro, para conferência à parte.

### 5.2 O helper

```ts
/** Há o que mostrar? Vazia, ou só o título repetido, não conta. */
export function temDescricao(s: Servico): boolean {
  const d = normalizeText(s.descricao ?? "");
  return d.length > 0 && d !== normalizeText(s.titulo);
}
```

Puro e testável, no `servicos.ts`, no idioma do `filterServicoGroups`. O `?? ""` cobre um JSON
antigo em cache sem o campo. O `normalizeText` é o que o `lib/textSearch.ts` já exporta (caixa,
acentos e espaços), então "igual ao título" usa a mesma régua da busca.

### 5.3 Os links dentro do texto

O acervo cita endereço assim: `(clique aqui "https://x/servicos?servico=1666").` A regra de hoje
captura até o próximo espaço e tira a pontuação do fim uma vez só. Sobre os 5.892 endereços das
descrições (simulação em Python das mesmas expressões):

| Regra | Links com sobra no `href` |
| --- | --- |
| Hoje | 180 (RS 88, MG 47, PR 33, ES 4, BA 3, PA 3, SE 1, TO 1) |
| A URL termina na aspa (`[^\s"“”]`) | 7 |
| Idem, e a limpeza do fim repete até estabilizar | **0** |

Três casos reais que viram teste do `linkify`:

```text
…(clique aqui "https://atendimento.receita.rs.gov.br/pessoa-juridica/servicos?servico=1666").
…acesse "https://www.ba.gov.br/".O contribuinte…
…(http://www.simplifica.es.gov.br);
```

Esboço:

```ts
const urlSplitRegex = /(https?:\/\/[^\s"“”]+)/g;

function splitTrailing(part: string) {
  let url = part;
  for (;;) {
    const antes = url;
    // ")" sobrando: mais fechamentos que aberturas (URL com parênteses casados fica intacta)
    if (url.endsWith(")") && conta(url, ")") > conta(url, "(")) url = url.slice(0, -1);
    url = url.replace(trailingPunctuation, "");
    if (url === antes) break;
  }
  return { url, trailing: part.slice(url.length) };
}
```

O `editLinks` é compartilhado com as Notas. A mudança só encurta um link que hoje engoliria aspa ou
pontuação, e os testes existentes do `linkify.test.tsx` têm de continuar passando sem edição.

`overflowWrap: anywhere` no painel é obrigatório: sem ele, uma URL de 100 caracteres alarga a
página no celular.

### 5.4 O estado e as chaves

O "aberto" mora na linha (`useState`), como no `ParecerItem` e no `TreeNode`. Fechar a classe
desmonta as linhas e zera o estado; trocar de aba já fecha as classes.

A `key` de hoje é `s.id`, que recomeça em 1 em cada arquivo. Com estado na linha, a mesma `key`
dentro de uma classe de mesmo nome em outra aba reaproveitaria a instância, e o "aberto" de um
serviço passaria a outro. Classes repetidas entre abas são a regra (RS 21 de 30, SP 32 de 36,
SC 19 de 20). O efeito é marginal, porque a troca de aba fecha os grupos, mas o conserto custa uma
linha: `key={`${s.titulo}\u0000${s.link}`}`, o idioma do `ConteudosAccordion`. Medido: nenhuma
colisão desse par dentro de uma mesma classe nas 27 entidades.

### 5.5 Quanto pesa na tela

A linha passa de 1 elemento de DOM para cerca de 7. Na pior classe (CE, 374) são ~2.600 elementos
com o grupo aberto; numa busca que abre todos os grupos da maior aba, alguns milhares. É duas
ordens de grandeza abaixo do que obrigou os Pareceres ao render em lotes (~140 mil). Não precisa de
lote nem de virtualização.

## 6. Caminhos descartados

- **Buscar a descrição sob demanda** (um arquivo por serviço, ou a árvore `docs/servicos/*.md`).
  O texto já está baixado; seria rede e código para trazer de novo o que se tem.
- **`ReactMarkdown` com `remark-breaks`.** Preservaria as quebras, mas trata `1.`, `*`, `_` e `#`
  do texto como sintaxe. Num acervo sem markdown, é risco sem ganho.
- **Abrir a descrição num modal ou num painel lateral.** Tira o serviço do contexto da classe e não
  é o que foi pedido.
- **Acordeão exclusivo** (abrir um fecha o outro). Impede comparar dois serviços parecidos, que é
  justamente o uso de quem escolhe entre casos pontuais.
- **Tornar todo serviço expansível, com "Sem descrição" no painel.** Três estados inteiros
  ganhariam um clique a mais para chegar ao portal, em troca de nada.

## 7. Passo a passo

### Passo 1 — O tipo e o helper

`descricao: string` em `Servico`; `temDescricao` em `servicos.ts`; o `svc()` do `servicos.test.ts`
passa a preencher o campo. Testes do helper: vazia, só espaços, igual ao título com caixa e
acento diferentes, descrição curta e legítima.
**Verificar:** `npm run typecheck` e `vitest run src/secoes/servicos`.

### Passo 2 — O `editLinks`

Os dois consertos da §5.3 e os três casos reais como testes novos.
**Verificar:** os testes antigos do `linkify.test.tsx` passam sem edição.
**Mutação:** devolver a expressão antiga e ver os três novos falharem.

### Passo 3 — A linha (portão humano ao fim)

`ServicoItem` no `ServicosAccordion.tsx`, a troca da `key`, e um `ServicosAccordion.test.tsx` novo
(modelo: `PareceresAccordion.test.tsx`):

1. linha com descrição: `aria-expanded` vai de `false` a `true` e o texto aparece;
2. linha sem descrição: não tem `aria-expanded` e continua sendo `<a>` com o `href` do portal;
3. o ícone do portal é `<a>` com `href`, `target="_blank"` e `rel="noopener noreferrer"`;
4. Enter no ícone **não** muda o `aria-expanded` da linha.

**Mutação:** tirar o `temDescricao` (o 2 falha); tirar o `stopPropagation` do `onKeyDown` (o 4
falha).
**Verificar:** `npm run typecheck`, `npm run lint`, `vitest run`, `npm run doctor`.
**Portão:** conferir no navegador (§8).

### Passo 4 — Registrar e fechar

`auli-frontend/DESCRIPTION.md` (§2, Serviços) e a linha "Serviços — agrupados por classe, em
acordeões" do `docs/auli_features.md`. Este estudo entra em `docs/` com o status atualizado.

## 8. Portões humanos — o que eu preciso aprovar

Antes do Passo 1:

1. **O clique na linha deixa de abrir o portal** e passa a abrir a descrição; o portal vai para o
   ícone (D-SVC-2 e D-SVC-3). É a única mudança de hábito para quem já usa a aba.
2. **Serviço sem descrição continua link** (D-SVC-4), o que deixa dois tipos de linha na mesma
   lista em AM, CE, RO, MA e RN.
3. **Mexer no `editLinks`** (D-SVC-6), que as Notas também usam.

Depois do Passo 3, no navegador:

4. RS › Empresas: abrir e fechar por clique, Enter e Espaço; o ícone abre o portal e não mexe no
   acordeão; a busca continua marcando o título.
5. MG (descrições longas), SC (itens com `- ` e URL entre aspas), SP (curtas) e MS (nenhuma
   descrição: a aba tem de estar idêntica à de hoje).
6. No celular: URL longa não alarga a página.

## 9. Escolhas baratas de trocar

- **Copiar.** A folha das FAQs tem "Copiar resposta". Aqui caberia o mesmo ícone, copiando título,
  descrição e link. Deixei fora por não ter sido pedido; são poucas linhas.
- **"Abrir no portal" também dentro do painel** (`LinkExterno`, como nos Pareceres), além do ícone
  da linha. O ícone de 14 px é alvo pequeno no celular.
- **Cor do título expansível:** `fg` (proposto) ou manter `accent`.
- **Abrir o primeiro resultado sozinho** quando a busca devolve um serviço só.

## 10. Fora de escopo

- **A descrição na busca.** Hoje a busca olha classe e título. Incluir a descrição faria um serviço
  entrar na lista sem que o título mostre o porquê; pediria abrir o painel sozinho ou exibir o
  trecho que casou, e o `Highlight` teria de conviver com o `editLinks`. O custo de normalizar não
  é o obstáculo (o texto do RS é ~40 vezes menor que o das sinopses do SP, que precisaram de índice
  pré-computado). É uma entrega própria, depois desta.
- **A qualidade do texto extraído** ("clique aqui" sem endereço, frases coladas). É da extração de
  cada scraper de serviços, não do frontend; o conversor único que já existe
  (`docs/ESTUDO-kit-html-para-texto.md`) cobre só as FAQs. Vale olhar o DF e a PB antes de divulgar
  a novidade nesses estados.
- **Preencher as descrições de MS, RJ e PE.** É coleta, não tela.
- **O teclado nos ícones da folha das FAQs** (nota da §5.1).

## 11. O que a implementação mudou em relação ao estudo

1. **O ícone do portal é irmão do botão, não filho** (aprovado junto com os portões). O esboço da
   §5.1 punha o `<a>` dentro do elemento `role="button"` e contava com o `stopPropagation` no
   `onKeyDown`. Lado a lado, nenhum `stopPropagation` é preciso, e o link não fica dentro de um
   botão, cujos filhos são apresentacionais para o leitor de tela. A ordem na linha ficou
   título ⌄ ↗, e o ícone ganhou `p={2}`, um alvo maior no celular (a preocupação da §9). A mutação
   do Passo 3 mostrou um segundo defeito do aninhamento: o rótulo do ícone ("Abrir … no portal
   oficial") entrava no nome do botão. Por isso o teste do teclado acha a linha pelo
   `aria-expanded`, não pelo nome.
2. **`descricao?: string`**, opcional no tipo. Com o campo obrigatório, o `?? ""` do
   `temDescricao` seria código morto para o TypeScript; opcional, ele cobre o JSON de deploy
   antigo, como o fallback de abas do `ServicosList`. Por isso o `svc()` do `servicos.test.ts` não
   precisou mudar.
3. **"Não há markdown no acervo" (§3) não era exato, e isso reforça a D-SVC-5.** Há 13 linhas
   começadas por `* ` (MG), cerca de 100 começadas por `1. `, dois `**` soltos (DF e RS) e trechos
   como `<Selecione>` (MG) e `<Manual de Orientação>` (SC). O `ReactMarkdown` viraria listas e
   apagaria esses trechos como HTML; em texto puro, saem literais. Virou teste do componente.
4. **Ajustes visuais pedidos no navegador:** o título do serviço recua 20 px sob a classe (o passo
   por nível da árvore das FAQs) e tem peso 500 (entre o 600 da classe e o 400 do texto); a
   descrição recua mais 20 px sob o título. Recuo e peso valem também para a linha sem descrição,
   para não desalinhar as listas mistas (RN, MA, AM, CE, RO). **Consequência:** a aba do MS deixa
   de ficar idêntica à de antes (§8, item 5) nesses dois pontos; o comportamento é o mesmo.
5. **A medição dos links (§5.3)** deu 179 com sobra pela regra antiga, não 180 (o TO não apareceu
   na nova simulação, diferença de heurística), e 0 com a regra nova; a conclusão não muda.
6. **O teste do componente** tem cinco casos: os quatro do Passo 3 mais o do texto puro (item 3).
   Mutações conferidas: sem a comparação com o título no `temDescricao`, sem o `temDescricao` na
   linha, o `linkify` do `main` e o ícone aninhado no botão; cada uma derruba o teste esperado.
