# TAREFA-TRIBUTUM-LIVROS — a quinta estante: Livros

**Data:** 22/08/2026
**Base:** `801e288`.
**Doutrina:** o repositório vence.

## 1. O que é

O grupo Tributum ganha uma sexta entrada de menu, **Livros**, como **última linha** do grupo (depois de Análises). É uma estante — entra no tipo `Estante`, no catálogo, no rodapé do Curador, na conferência do JSON. Um livro é um item com autor, editora, ano e ISBN, apontado para onde se compra ou se lê; nunca hospedado, por padrão.

Ordem final do grupo: Critérios · Artigos e estudos · Instituições · Dados · Análises · **Livros**.

## 2. Decisões

- **D-TRIB-22 — Livro é item de catálogo, não de acervo.** Mesma doutrina das outras estantes: link + resenha própria do Curador. Livro não tem `pdf` nem `hospedado`: obra comercial não se hospeda, e livro em domínio público ou acesso aberto entra pelo `link` para o repositório que o publica (Ipea, Enap, editoras universitárias). Se um dia houver livro com autorização de hospedagem, é mudança de schema com decisão própria — não se prevê agora.
- **D-TRIB-23 — ISBN é opcional, link é obrigatório.** O critério editorial exige autoria, fonte localizável e data. Para livro, "fonte localizável" é o `link` (editora, repositório ou catálogo de biblioteca). ISBN é útil para desambiguar edições, mas livros antigos e publicações institucionais não têm; exigir quebraria os melhores itens (os relatórios-livro do Ipea, por exemplo).
- **D-TRIB-24 — `acesso` diz como se chega ao livro**, em três valores: `"aberto"` (PDF gratuito e legal no link), `"comercial"` (compra), `"biblioteca"` (só em catálogo de biblioteca). A lista mostra isso como um selo, para o leitor saber antes de clicar se vai encontrar um PDF ou uma loja. É o equivalente, para livros, do que `hospedado` diz para artigos.
- **D-TRIB-25 — A resenha é obrigatória e curta**, como nas outras estantes. Sinopse de editora não serve: é material promocional (critério editorial, "o que fica de fora"). O Curador escreve por que vale ler.
- **D-TRIB-26 — Sem capa.** Imagem de capa é obra protegida da editora; a lista é texto. Se um dia quiser, é link para a página da editora, não arquivo no repositório.

## 3. Catálogo

### 3.1 `types.ts`

```ts
/** Como se chega ao livro (D-TRIB-24). Mostrado como selo na lista. */
export type AcessoLivro = "aberto" | "comercial" | "biblioteca";

export interface Livro extends ItemBase {
  titulo: string;
  autores: string;
  editora: string;
  ano: number;
  /** Opcional (D-TRIB-23): edições antigas e publicações institucionais não têm. */
  isbn?: string;
  acesso: AcessoLivro;
  /** Resenha do Curador — nunca a sinopse da editora (D-TRIB-25). */
  resumo: string;
  /** Editora, repositório ou catálogo de biblioteca. Obrigatório: é a "fonte localizável". */
  link: string;
}

export type Estante = "artigos" | "instituicoes" | "dados" | "analises" | "livros";

export interface TributumCatalogo {
  artigos: Artigo[];
  instituicoes: Instituicao[];
  dados: Dataset[];
  analises: Analise[];
  livros: Livro[];
  curadores?: Partial<Record<Estante, Curador>>;
}
```

Atualizar o comentário de `Estante` ("As quatro estantes" → "As cinco estantes").

### 3.2 `useTributum.ts`

`VAZIO` ganha `livros: []`; `normalizar` ganha `livros: arr(j.livros)`. Catálogo antigo sem a chave continua válido (estante vazia) — é o comportamento já existente para chaves ausentes.

### 3.3 `public/tributum.json`

Nova chave `"livros": [...]` **entre `analises` e `curadores`** (ordem do arquivo = ordem do menu, para quem edita à mão). `curadores` ganha `"livros": { "nome": "<mesmo Curador das outras estantes, salvo decisão do Carlos>" }`.

Itens iniciais — os três abaixo têm fonte conhecida, mas **nenhum foi aberto nesta TAREFA**: entram com `placeholder: true` até o Curador conferir link, ISBN e edição. É deliberado: livro errado de edição é pior que estante curta.

```json
"livros": [
  {
    "id": "varsano-sistema-tributario-brasileiro",
    "titulo": "A evolução do sistema tributário brasileiro ao longo do século: anotações e reflexões para futuras reformas",
    "autores": "Ricardo Varsano",
    "editora": "Ipea (Texto para Discussão 405, edição em livro)",
    "ano": 1996,
    "acesso": "aberto",
    "resumo": "A história do sistema tributário de 1891 a 1988 contada por quem ajudou a desenhá-lo. Curto, e ainda a melhor porta de entrada.",
    "link": "https://repositorio.ipea.gov.br/",
    "placeholder": true
  },
  {
    "id": "appy-et-al-reforma-tributaria-ibs",
    "titulo": "Reforma tributária: IBS e CBS",
    "autores": "(a confirmar — coletânea sobre a EC 132/2023)",
    "editora": "(a confirmar)",
    "ano": 2024,
    "acesso": "comercial",
    "resumo": "(resenha do Curador)",
    "link": "https://",
    "placeholder": true
  },
  {
    "id": "rezende-financas-publicas",
    "titulo": "Finanças Públicas",
    "autores": "Fernando Rezende",
    "editora": "Atlas",
    "ano": 2001,
    "isbn": "(a confirmar)",
    "acesso": "comercial",
    "resumo": "O manual brasileiro de finanças públicas; o capítulo sobre federalismo fiscal explica por que o ICMS é como é.",
    "link": "https://",
    "placeholder": true
  }
]
```

O Curador substitui esses três pelo que quiser; são exemplos de formato, com `placeholder:true` justamente para não publicarem por acidente.

## 4. Código

### 4.1 `LivrosLista.tsx` (novo, em `src/tributum/`)

Copiar a estrutura de `InstituicoesLista.tsx` (é a mais simples: sem modal, sem PDF). Cartão:

- Linha 1: título (`Highlight`) + `PlaceholderBadge`.
- Linha 2, `fg.muted`: `{autores} · {editora}, {ano}` e, se houver, ` · ISBN {isbn}`.
- Linha 3: `resumo`.
- Linha 4: selo de acesso (`Badge` Chakra, `variant="subtle"`, texto "Acesso aberto" / "Venda" / "Biblioteca") + `LinkExterno` com rótulo que depende do acesso: "Ler" para aberto, "Ver na editora" para comercial, "Ver no catálogo" para biblioteca.

`campos={(l) => [l.titulo, l.autores, l.editora, l.resumo]}` — a busca acha por editora também. `substantivo={["livro", "livros"]}`.

### 4.2 `Home.tsx`

Nova entrada **depois** de `tributum-analises`:

```ts
{ id: "tributum-livros", label: "Livros", Component: LivrosLista, collection: null, grupo: "tributum", Icone: MdOutlineMenuBook },
```

Ajustar o comentário da linha 51 ("as cinco do `tributum`" → "as seis"). Confirmar que `MdOutlineMenuBook` existe em `react-icons/md` (existe em 5.x; se a versão instalada não tiver, `MdOutlineBook`).

### 4.3 Testes

- `Home.drawer.test.tsx`: contagem literal **16 → 17**, e o comentário ganha "…e 16 até "Livros" entrar como a sexta (D-TRIB-22)". O comentário é a história; mantém-se o hábito.
- `types.test.ts`:
  - as três iterações sobre `["artigos","instituicoes","dados","analises"] as const` passam a incluir `"livros"` — extrair para uma constante `ESTANTES` exportada de `types.ts` (`export const ESTANTES = [...] as const satisfies readonly Estante[]`) para que a próxima estante não exija caçar arrays literais. O teste "as quatro estantes declaram curador" vira "todas as estantes declaram curador".
  - Novo: **"nenhum livro traz `pdf` ou `hospedado`"** (D-TRIB-22) — pega quem copiar um artigo e trocar o tipo.
  - Novo: **"todo livro publicado tem `link` e `acesso` válido"** — `placeholder:false` exige `link` começando com `https://` e `acesso` em `["aberto","comercial","biblioteca"]`.
  - Novo: **"todo livro tem `resumo` não vazio"** (D-TRIB-25).
- `TributumShell.test.tsx`: nada muda — o Shell é genérico; mas se houver teste que lista estantes, usar `ESTANTES`.

### 4.4 Prosa

- `tributum-criterios.md`: a frase "Aqui você vai encontrar quatro estantes: **Artigos e estudos**, **Instituições**, **Dados** e **Análises**." passa a "cinco estantes: …, **Análises** e **Livros**." É a única mudança no texto — e só ela; o resto é prosa aprovada.
- `_leia_me` do JSON: acrescentar "LIVROS (D-TRIB-22/25): nunca hospedados; acesso em aberto|comercial|biblioteca; resumo e do Curador, nunca sinopse de editora."
- Doc comment de `CriteriosTributum.tsx` e o `auli_code.md` (§ do Tributum, se houver): "cinco estantes".

## 5. Verificação

1. `pnpm typecheck` — `Estante` novo propaga para `curadores`, Shell, testes; nada deve compilar com array literal de quatro.
2. `pnpm test` — 17 abas; `ESTANTES` com cinco; os três testes novos verdes com os três placeholders; mutação: colocar `"pdf"` num livro deve ficar vermelho.
3. A aba Livros aparece como **última** do grupo, com os três itens de exemplo marcados como placeholder, e o rodapé do Curador.
4. Grep: nenhum array literal `["artigos", "instituicoes", "dados", "analises"]` sobrou.
5. Guardas de #165/#166 verdes.

## 6. Commit

`feat(tributum): a estante Livros entra como última linha do grupo (D-TRIB-22..26)`
