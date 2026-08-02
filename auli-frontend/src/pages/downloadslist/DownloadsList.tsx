import { useMemo } from "react";
import { Box, Button, Flex, IconButton, Stack, Table, Text } from "@chakra-ui/react";
import { MdFileDownload, MdInbox } from "react-icons/md";
import useSWR from "swr";
import {
  colunasDeTipos,
  contagem,
  formatarData,
  nomeDoEstado,
  resumoTipos,
  tipoLabel,
  totalDocumentos,
  zipHref,
  type DownloadEntry,
} from "./downloads";
import { jsonFetcher, SWR_OPTS, versioned } from "../../shared/fetchers";
import { AsyncContent } from "../../shared/AsyncContent";
import { useSelectedEntity } from "../../shared/EntityContext";

/** Manifesto GLOBAL (todos os estados) — diferente dos dados por-entidade em `/<id>/<id>-*.json`. */
const MANIFEST_PATH = "/downloads/downloads.json";

/**
 * Aba "Downloads": o pacote `.zip` do estado selecionado em destaque, e abaixo a tabela de todos os
 * estados, para quem quiser levar mais de um.
 *
 * O híbrido é deliberado. O app é escopado a uma entidade por vez, e uma tabela de 26 estados solta
 * dentro dela contradiria o header; mas o manifesto é global por natureza, e obrigar a trocar de
 * estado 26 vezes só para ver o que existe seria hostil com quem baixa pacote para alimentar o
 * próprio RAG. O destaque respeita o modelo; a tabela não esconde o resto.
 *
 * `collection: null` no Home mantém a aba sempre visível: a lista não depende de a entidade atual
 * ter pacote. Quando ela não tem (entidade ainda sem árvore `docs/`), o destaque vira um
 * estado-vazio e a tabela segue de pé.
 */
export function DownloadsList() {
  const entity = useSelectedEntity();
  const { data, error, isLoading } = useSWR<DownloadEntry[]>(
    versioned(MANIFEST_PATH),
    jsonFetcher<DownloadEntry[]>,
    SWR_OPTS,
  );

  const entries = useMemo(() => data ?? [], [data]);
  const atual = entries.find((d) => d.estado === entity.id);
  const colunas = useMemo(() => colunasDeTipos(entries), [entries]);

  return (
    <Flex direction="column" flex={1} w="100%" bg="bg.app">
      <Box px={4} pt={4} pb={6}>
        <AsyncContent
          loading={isLoading}
          error={error}
          loadingText="Carregando pacotes para download…"
          errorTitle="Erro ao carregar downloads"
          errorDescription={(error as Error)?.message}
        >
          <Stack gap={6} maxW="900px">
            <Text fontSize="0.95rem" color="fg" lineHeight="1.6">
              Baixe as tabelas de um estado num único arquivo <Text as="code">.zip</Text> e aponte a
              sua própria IA para as pastas de <Text as="code">.md</Text> — um documento por
              arquivo, prontos para RAG. Cada pasta traz um <Text as="code">README.md</Text> com o
              schema dos arquivos.
            </Text>

            {atual ? (
              <Box
                border="1px solid var(--chakra-colors-border)"
                borderRadius="8px"
                bg="bg.subtle"
                p={4}
              >
                <Flex align="center" justify="space-between" gap={4} flexWrap="wrap">
                  <Stack gap={1}>
                    <Text fontSize="1.05rem" fontWeight="700" color="fg">
                      {atual.nome} · {nomeDoEstado(atual.estado)}
                    </Text>
                    <Text fontSize="0.85rem" color="fg.muted">
                      {resumoTipos(atual)} · atualizado em {formatarData(atual.atualizado_em)}
                    </Text>
                  </Stack>
                  <Flex align="center" gap={3}>
                    <Text fontSize="0.85rem" color="fg.muted">
                      {atual.tamanho}
                    </Text>
                    {/* asChild → âncora real, para o atributo `download` funcionar. */}
                    <Button asChild size="md" bg="accent" color="accent.fg" _hover={{ bg: "brand.600" }}>
                      <a href={zipHref(atual)} download>
                        <MdFileDownload />
                        Baixar {atual.arquivo}
                      </a>
                    </Button>
                  </Flex>
                </Flex>
              </Box>
            ) : (
              <Flex
                direction="column"
                align="center"
                py={10}
                border="1px solid var(--chakra-colors-border)"
                borderRadius="8px"
              >
                <Stack gap={3} align="center" maxW="380px" textAlign="center" px={4}>
                  <MdInbox size={36} color="var(--chakra-colors-fg-muted)" />
                  <Text fontSize="1rem" fontWeight="600" color="fg">
                    Pacote ainda não disponível para {entity.uf}
                  </Text>
                  <Text fontSize="0.9rem" color="fg.muted" lineHeight="1.6">
                    Assim que os dados de {entity.name} forem empacotados, o download aparece aqui.
                    Os demais estados seguem disponíveis abaixo.
                  </Text>
                </Stack>
              </Flex>
            )}

            <Stack gap={2}>
              <Text as="h2" fontSize="0.95rem" fontWeight="700" color="fg">
                Todos os estados
              </Text>
              <Box border="1px solid var(--chakra-colors-border)" borderRadius="8px" overflowX="auto">
                <Table.Root size="sm">
                  <Table.Header>
                    <Table.Row bg="bg.subtle">
                      <Table.ColumnHeader color="fg" fontWeight="700">
                        Estado
                      </Table.ColumnHeader>
                      {colunas.map((tipo) => (
                        <Table.ColumnHeader key={tipo} color="fg" fontWeight="700" textAlign="end">
                          {tipoLabel(tipo)}
                        </Table.ColumnHeader>
                      ))}
                      <Table.ColumnHeader color="fg" fontWeight="700" textAlign="end">
                        Tamanho
                      </Table.ColumnHeader>
                      <Table.ColumnHeader color="fg" fontWeight="700" textAlign="end">
                        Atualizado
                      </Table.ColumnHeader>
                      <Table.ColumnHeader />
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {entries.map((e) => {
                      const selecionado = e.estado === entity.id;
                      return (
                        <Table.Row key={e.estado} bg={selecionado ? "bg.subtle" : undefined}>
                          <Table.Cell color="fg" fontWeight={selecionado ? "700" : undefined}>
                            {e.uf} · {nomeDoEstado(e.estado)}
                          </Table.Cell>
                          {colunas.map((tipo) => {
                            const n = contagem(e, tipo);
                            return (
                              <Table.Cell key={tipo} color={n ? "fg" : "fg.muted"} textAlign="end">
                                {n ? n.toLocaleString("pt-BR") : "—"}
                              </Table.Cell>
                            );
                          })}
                          <Table.Cell color="fg" textAlign="end" whiteSpace="nowrap">
                            {e.tamanho}
                          </Table.Cell>
                          <Table.Cell color="fg.muted" textAlign="end" whiteSpace="nowrap">
                            {formatarData(e.atualizado_em)}
                          </Table.Cell>
                          <Table.Cell textAlign="end">
                            <IconButton
                              asChild
                              size="xs"
                              variant="ghost"
                              color="accent"
                              aria-label={`Baixar ${e.arquivo} — ${nomeDoEstado(e.estado)}, ${totalDocumentos(e).toLocaleString("pt-BR")} documentos, ${e.tamanho}`}
                            >
                              <a href={zipHref(e)} download>
                                <MdFileDownload />
                              </a>
                            </IconButton>
                          </Table.Cell>
                        </Table.Row>
                      );
                    })}
                  </Table.Body>
                </Table.Root>
              </Box>
            </Stack>

            <Text fontSize="0.8rem" color="fg.muted" lineHeight="1.6">
              O arquivo vai para a pasta de downloads do seu navegador. Para escolher onde salvar,
              ligue a opção de perguntar o destino a cada download nas configurações dele. Conteúdo
              derivado de dados públicos das secretarias estaduais da fazenda; cada arquivo traz no
              frontmatter o link para a fonte oficial, que prevalece sobre esta cópia.
            </Text>
          </Stack>
        </AsyncContent>
      </Box>
    </Flex>
  );
}
