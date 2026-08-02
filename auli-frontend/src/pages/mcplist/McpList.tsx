import { useState } from "react";
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
