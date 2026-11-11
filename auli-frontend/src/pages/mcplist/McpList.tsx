import { useState, type ReactNode } from "react";
import { Box, Button, Flex, Stack, Table, Text } from "@chakra-ui/react";
import { MdArrowBack, MdMenuBook } from "react-icons/md";
import ReactMarkdown from "react-markdown";
import useSWR from "swr";
import { textFetcher, SWR_OPTS, versioned } from "../../shared/fetchers";
import { AsyncContent } from "../../shared/AsyncContent";
import { proseMarkdownComponents, markdownPlugins } from "../../shared/markdown";

/**
 * Um manual de instalação do conector MCP. A lista é hardcoded de propósito: espelha
 * `manuais/README.md` (a fonte autorada) e um manual novo é uma entrada a mais aqui —
 * um `manuais.json` gerado seria infraestrutura especulativa para 4 itens.
 */
interface Manual {
  /** Nome de exibição do assistente ("Claude (web)"). */
  titulo: string;
  /** Público do manual, como no README ("Usuário final"). */
  paraQuem: string;
  /** Quem consegue fazer a configuração ("O próprio usuário — plano Pro ou Max…"). */
  quemConfigura: string;
  /** Nome do arquivo em `public/manuais/` (copiado de `manuais/` pelo build-frontend-public.sh). */
  arquivo: string;
}

/** Espelha a tabela de `manuais/README.md`; manter as duas em sincronia ao editar. */
const MANUAIS: Manual[] = [
  {
    titulo: "ChatGPT",
    paraQuem: "Usuário final",
    quemConfigura: "O próprio usuário — precisa de conta paga e do modo desenvolvedor ligado",
    arquivo: "manual-auli-mcp-chatgpt.md",
  },
  {
    titulo: "Claude (web)",
    paraQuem: "Usuário final",
    quemConfigura:
      "O próprio usuário — plano Pro ou Max; em conta Team/Enterprise, um Owner cadastra antes",
    arquivo: "manual-auli-mcp-claude.md",
  },
  {
    titulo: "GitHub Copilot",
    paraQuem: "Quem usa VS Code",
    quemConfigura: "O próprio usuário — só funciona no chat em modo Agente",
    arquivo: "manual-auli-mcp-copilot.md",
  },
  {
    titulo: "Microsoft 365 Copilot",
    paraQuem: "Usuário do Copilot no Edge",
    quemConfigura:
      "A TI — é preciso construir um agente no Copilot Studio e ter aprovação de administrador",
    arquivo: "manual-auli-mcp-copilot-studio.md",
  },
];

/** Uma ferramenta do servidor MCP, como o bloco "O que dá para pedir" a apresenta. */
interface Ferramenta {
  /** Nome exato da `#[tool]` ("listar_entidades"). */
  nome: string;
  /** ReactNode porque carrega `<Text as="code">` e ênfase. */
  oQueFaz: ReactNode;
  /** Frases que levam o assistente a esta ferramenta. */
  exemplo: ReactNode;
}

/** Frase-exemplo entre aspas e em itálico. As aspas vão na string, iguais às do manual. */
const frase = (texto: string) => (
  <Text as="em" fontStyle="italic">
    {`"${texto}"`}
  </Text>
);

/**
 * As cinco ferramentas do `/mcp`. "O que faz" resume — não copia — as `description` das `#[tool]`
 * em `auli-server/crates/auli-cli/src/mcp.rs`, que é a fonte. As frases-exemplo são as mesmas da
 * seção "As cinco ferramentas, e como pedi-las" de `manuais/manual-auli-mcp-claude.md`; manter as
 * duas em sincronia ao editar. Sem totais: envelhecem, e o `listar_entidades` os dá na hora.
 */
const FERRAMENTAS: Ferramenta[] = [
  {
    nome: "listar_entidades",
    oQueFaz: (
      <>
        Quais estados têm acervo e o que cada um tem (pareceres, TARF, legislação, serviços, FAQs),
        com os totais. Sem parâmetros.
      </>
    ),
    exemplo: frase("Quais estados têm acervo no Auli, e o que cada um tem?"),
  },
  {
    nome: "buscar_pareceres",
    oQueFaz: (
      <>
        Busca por sentido na jurisprudência de uma UF. Devolve número, assunto, sinopse e link —{" "}
        <Text as="strong">sem o corpo</Text>. Parâmetros: <Text as="code">uf</Text>,{" "}
        <Text as="code">pergunta</Text>, <Text as="code">top_k</Text> (1 a 20, padrão 5) e{" "}
        <Text as="code">colecao</Text>: <Text as="code">pareceres</Text> (padrão) ou{" "}
        <Text as="code">tarf</Text>.
      </>
    ),
    exemplo: (
      <>
        {frase("Em Santa Catarina, o que já foi respondido sobre crédito de ICMS na energia elétrica?")}{" "}
        — ou, para os acórdãos: {frase("O que o TARF do RS já decidiu sobre…")}. Quer mais
        resultados? {frase("Traga os 15 mais próximos.")}
      </>
    ),
  },
  {
    nome: "obter_parecer",
    oQueFaz: (
      <>
        O texto integral de <Text as="strong">um</Text> documento, pelo número exato que a busca
        devolveu, na mesma coleção. O assistente só a usa se você pedir o texto completo.
      </>
    ),
    // Sem exemplo "pelo número": a ferramenta exige o título exato que a busca devolveu
    // ("RESPOSTA À CONSULTA TRIBUTÁRIA 23333/2021"), que ninguém digita de cabeça.
    exemplo: (
      <>
        {frase("Me mostre o texto completo do mais relevante.")} — ou{" "}
        {frase("Abra a íntegra do segundo da lista.")}
      </>
    ),
  },
  {
    nome: "consultar_servicos_faqs",
    oQueFaz: (
      <>
        Para dúvidas de <Text as="strong">como fazer</Text>: devolve, num bloco só, até 5 serviços e
        10 FAQs da UF mais próximos da pergunta — o mesmo contexto que o chat deste portal usa.
      </>
    ),
    exemplo: frase("No Rio Grande do Sul, como faço para emitir a segunda via da guia do IPVA?"),
  },
  {
    nome: "buscar_legislacao",
    oQueFaz: (
      <>
        Para o que a <Text as="strong">lei</Text> diz: pares pergunta/resposta sobre a norma, cada um
        com o dispositivo citado ({`"Art. 18-A, § 2º"`}) e o link oficial. Disponível só onde{" "}
        <Text as="code">listar_entidades</Text> o reporta.
      </>
    ),
    exemplo: frase("O que a lei do RS exige de uma locadora para pagar a alíquota menor de IPVA?"),
  },
];

/** Leitor de um manual: botão de voltar + o .md renderizado (mesmo padrão da aba Sobre). */
function LeitorManual({ manual, onVoltar }: { manual: Manual; onVoltar: () => void }) {
  const { data, error, isLoading } = useSWR(
    versioned(`/manuais/${manual.arquivo}`),
    textFetcher,
    SWR_OPTS,
  );

  return (
    <Box px={4} pt={4} pb={10} w="100%" maxW="720px" mx="auto">
      <Button size="xs" variant="outline" onClick={onVoltar} mb={4}>
        <MdArrowBack />
        Voltar aos manuais
      </Button>
      <AsyncContent
        loading={isLoading}
        error={error}
        loadingText="Carregando manual…"
        errorTitle="Erro ao carregar o manual"
        errorDescription={(error as Error)?.message}
      >
        <Box
          fontSize="1rem"
          lineHeight="1.7"
          color="fg"
          fontFamily="body"
          className="markdown-body"
        >
          <ReactMarkdown remarkPlugins={markdownPlugins} components={proseMarkdownComponents}>
            {data ?? ""}
          </ReactMarkdown>
        </Box>
      </AsyncContent>
    </Box>
  );
}

/**
 * Aba "MCP": como consultar o acervo do Auli de dentro de cada assistente de IA.
 *
 * Duas telas num estado só: `null` = a lista (tabela com os 4 manuais), um `Manual` = o leitor.
 * Sem rota — o app inteiro é abas sem router (ver App.tsx), e o estado sobrevive à troca de aba
 * porque o Home mantém o painel montado.
 */
export function McpList() {
  const [aberto, setAberto] = useState<Manual | null>(null);

  if (aberto) {
    return (
      <Flex direction="column" flex={1} w="100%" bg="bg.app">
        <LeitorManual manual={aberto} onVoltar={() => setAberto(null)} />
      </Flex>
    );
  }

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      <Box px={4} pt={4} pb={6}>
        <Stack gap={6} maxW="900px">
          <Text fontSize="0.95rem" color="fg" lineHeight="1.6">
            O acervo do Auli pode ser consultado de dentro do seu assistente de IA, via{" "}
            <Text as="strong">MCP</Text> (Model Context Protocol). Todos os manuais apontam para o
            mesmo servidor, público e sem autenticação:{" "}
            <Text as="code">https://api.auli.com.br/mcp</Text>. O que muda de um assistente para
            outro é quem configura: em alguns o próprio usuário resolve em minutos; em outros, só a
            área de TI consegue.
          </Text>

          <Stack gap={2}>
            <Text as="h2" fontSize="0.95rem" fontWeight="700" color="fg">
              Manuais de instalação
            </Text>
            <Box border="1px solid var(--chakra-colors-border)" borderRadius="8px" overflowX="auto">
              <Table.Root size="sm">
                <Table.Header>
                  <Table.Row bg="bg.subtle">
                    <Table.ColumnHeader color="fg" fontWeight="700">
                      Assistente
                    </Table.ColumnHeader>
                    <Table.ColumnHeader color="fg" fontWeight="700">
                      Para quem
                    </Table.ColumnHeader>
                    <Table.ColumnHeader color="fg" fontWeight="700">
                      Quem configura
                    </Table.ColumnHeader>
                    <Table.ColumnHeader />
                  </Table.Row>
                </Table.Header>
                <Table.Body>
                  {MANUAIS.map((m) => (
                    <Table.Row key={m.arquivo}>
                      <Table.Cell color="fg" fontWeight="600" whiteSpace="nowrap">
                        {m.titulo}
                      </Table.Cell>
                      <Table.Cell color="fg" whiteSpace="nowrap">
                        {m.paraQuem}
                      </Table.Cell>
                      <Table.Cell color="fg.muted">{m.quemConfigura}</Table.Cell>
                      <Table.Cell textAlign="end">
                        <Button
                          size="xs"
                          variant="outline"
                          color="accent"
                          onClick={() => setAberto(m)}
                          aria-label={`Ler o manual do ${m.titulo}`}
                        >
                          <MdMenuBook />
                          Ler
                        </Button>
                      </Table.Cell>
                    </Table.Row>
                  ))}
                </Table.Body>
              </Table.Root>
            </Box>
          </Stack>

          {/* Borda pelo token `border`, não `brand.50`: o primitivo não tem `_dark` e viraria um
              contorno claro sobre o `bg.resumo` escuro. */}
          <Box
            bg="bg.resumo"
            border="1px solid var(--chakra-colors-border)"
            borderRadius="10px"
            py="16px"
            px="18px"
          >
            <Stack gap={2}>
              <Text as="h2" fontSize="0.95rem" fontWeight="700" color="fg">
                O que dá para pedir
              </Text>
              <Text fontSize="0.9rem" color="fg" lineHeight="1.6">
                Depois de conectado, você não chama ferramenta nenhuma: pergunta em português e o
                assistente escolhe qual usar. Mas ajuda saber o que existe por baixo — são cinco
                ferramentas, e a frase certa leva à ferramenta certa. Diga sempre{" "}
                <Text as="strong">o estado</Text>: quatro das cinco ferramentas exigem a UF, e sem
                ela o assistente tem de escolher uma por conta ou perguntar qual.
              </Text>
              <Box
                border="1px solid var(--chakra-colors-border)"
                borderRadius="8px"
                overflowX="auto"
              >
                <Table.Root size="sm" aria-label="Ferramentas do servidor MCP">
                  <Table.Header>
                    <Table.Row bg="bg.subtle">
                      <Table.ColumnHeader color="fg" fontWeight="700">
                        Ferramenta
                      </Table.ColumnHeader>
                      <Table.ColumnHeader color="fg" fontWeight="700" minW="16rem">
                        O que faz
                      </Table.ColumnHeader>
                      <Table.ColumnHeader color="fg" fontWeight="700" minW="16rem">
                        Diga, por exemplo
                      </Table.ColumnHeader>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {FERRAMENTAS.map((f) => (
                      <Table.Row key={f.nome}>
                        <Table.Cell color="fg" whiteSpace="nowrap" verticalAlign="top">
                          <Text as="code">{f.nome}</Text>
                        </Table.Cell>
                        <Table.Cell color="fg" verticalAlign="top">
                          {f.oQueFaz}
                        </Table.Cell>
                        <Table.Cell color="fg" verticalAlign="top">
                          {f.exemplo}
                        </Table.Cell>
                      </Table.Row>
                    ))}
                  </Table.Body>
                </Table.Root>
              </Box>
              <Text fontSize="0.9rem" color="fg" lineHeight="1.6">
                O encadeamento típico é <Text as="code">listar_entidades</Text> →{" "}
                <Text as="code">buscar_pareceres</Text> → <Text as="code">obter_parecer</Text>: o
                assistente descobre o que existe, busca, e só então abre a íntegra do que você
                escolheu. Qual das três frentes responde decide-se pelo tipo de pergunta: a{" "}
                <Text as="strong">legislação</Text> diz o que a norma determina; os{" "}
                <Text as="strong">pareceres</Text> e acórdãos, como a Receita e o tribunal a
                interpretam num caso; os <Text as="strong">serviços e FAQs</Text>, como fazer a
                coisa no portal.
              </Text>
            </Stack>
          </Box>

          <Stack gap={2}>
            <Text as="h2" fontSize="0.95rem" fontWeight="700" color="fg">
              Os dois “Copilot”
            </Text>
            <Text fontSize="0.9rem" color="fg" lineHeight="1.6">
              É a confusão mais comum, e vale checar antes de escolher o manual. O{" "}
              <Text as="strong">GitHub Copilot</Text> vive no editor de código, aceita servidor MCP
              direto e você mesmo configura. O <Text as="strong">Microsoft 365 Copilot</Text> abre
              no Edge, com o login do trabalho e o aviso de dados protegidos da empresa — não tem
              tela para colar URL de servidor e depende da TI. Mesmo nome, empresas relacionadas,
              produtos diferentes.
            </Text>
          </Stack>

          <Text fontSize="0.8rem" color="fg.muted" lineHeight="1.6">
            Se nenhum caminho estiver disponível, há duas alternativas que não dependem de
            configuração nenhuma: a consulta direto neste portal (aba Chat) e a aba Downloads — o{" "}
            <Text as="code">.zip</Text> do estado, com um <Text as="code">.md</Text> por documento,
            que qualquer IA que aceite anexo consegue ler.
          </Text>
        </Stack>
      </Box>
    </Flex>
  );
}
