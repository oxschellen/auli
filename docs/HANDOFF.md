# HANDOFF — estado do auli em 08/08/2026

**Para:** quem assumir a partir daqui
**Substitui:** `RELATORIO-MIGRACAO-FORMATO.md` (aquela migração terminou; o que sobrou dele está no §6)
**Estado:** nada quebrado, nada em curso. Uma coleta pausada, uma regra por escrever.

---

## 1. O que entrou em `main` hoje

| commit | o que fez |
|---|---|
| `ae036f1` (#127) | scraper dos acórdãos do TARF-RS; nova árvore `data/rs/docs/tarf/` |
| `a7aab1c` (#128) | **vocabulário unificado** das árvores `.md` — exigiu migração de dados |
| `6b019ea` (#129) | invólucro único do contexto RAG (`## documento {i}: {rótulo}`) |
| `48dfd6b` | `data_backup*/` fora do git |
| `7ddf324` | zip não publica coleção incompleta; TARF ganha rótulo humano |
| `8f293a5` (#130) | aba "Acórdãos TARF" + `indice` parametrizado por coleção |
| `28519c5` (#131) | publica o TARF parcial, dizendo que a coleta continua |

Tudo verificado, migrado e **em produção**. Não há trabalho pela metade.

## 2. Estado em produção (`auli.com.br`)

| | |
|---|---|
| Documentos publicados | **29.732** em 27 estados |
| `rs.zip` | 19,6 MB — faqs 1.947 · pareceres 372 · serviços 586 · **tarf 3.800** |
| Aba nova | "Acórdãos TARF" no grupo Acervo, com ressalva de coleta em andamento |
| Packs | 27/27 manifestos recarimbados; `docs_hash` bate com o disco |

O servidor da API roda **nesta máquina** (`start_server.sh` + túnel cloudflared). O VPS serve só o app estático via Apache. Deploy do frontend: `scripts/deploy-frontend.sh` (staging + troca atômica + smoke com rollback automático).

## 3. A ÚNICA coisa pendente: a coleta do TARF

**Parada em 3.800 de ~22.522 acórdãos.** Foi pausada duas vezes de propósito: a primeira para desbloquear a migração de formato, a segunda para a árvore do `rs` parar de se mexer (enquanto ela cresce, o `docs_hash` do manifesto fica obsoleto a cada lote e trava qualquer `auli update` da entidade).

```bash
cd auli-server && ./target/debug/auli-scraper-rs tarf     # retoma de onde parou, ~5h
```

Estado preservado: 3.917 corpos em cache (117 além da árvore, que não serão re-baixados) e o "existe ⇒ pula" reconhece os 3.800.

**Ao terminar, quatro passos:**

```bash
cd auli-server && ../auli-server/target/release/auli-collections rs indice tarf   # regera o índice
cd .. && scripts/build-packs.sh rs                                                # docs_hash!
# remover `nota_tipo("tarf")` em bundle.rs e `nota` em TarfList.tsx — a coleta fechou
scripts/deploy-frontend.sh
```

O `build-packs.sh rs` **não é opcional**: sem ele o `docs_hash` do manifesto diverge da árvore e o servidor recusa o boot do `rs` (erro duro, por desenho — `auli-core/src/manifest.rs:188`).

## 4. A regra que falta escrever: o terceiro layout

O scraper extrai a **ementa** (vira `ementa`) e a **fundamentação** (vira `## resumo`) do cabeçalho do acórdão. O portal escreve esse cabeçalho de três jeitos:

| layout | fatia | forma |
|---|---|---|
| moderno (2015+) | ~34% | `<tr>` rotulada `EMENTA:`; fundamentação nas linhas seguintes de rótulo vazio |
| antigo (2005–2014, 2019) | ~66% | export do Word; `EMENTA:` numa tabela posterior, com ementa **e** fundamentação na mesma célula |
| **Pleno, sem tabela** | **~1,5%** | ementa é um parágrafo solto: `<p><strong>EMENTA</strong>: …</p>` |

Os dois primeiros são atendidos por uma regra estrutural só (varre todas as tabelas até achar a linha `EMENTA`; a ementa é o **primeiro parágrafo** da célula, a fundamentação é o resto dela mais as linhas seguintes de rótulo vazio).

O terceiro não tem âncora estrutural. Esses documentos caem no canal de anomalia: registrados em `data/rs/raw/tarf-anomalias.txt` com número, GUID e link, **sem gerar `.md`**. Hoje são **18 registros**; extrapolando 1,5%, o censo final fica perto de 340.

**Não há erro congelado.** Como nunca geram `.md`, seguem "inéditos" para sempre — quando a regra entrar, uma re-execução os emite direto do cache, sem remoção manual nem re-coleta.

**Por que não implementei:** sem tabela não há como saber onde a fundamentação termina — depois dela vêm o dispositivo, os nomes dos juízes, o relatório, e nada no HTML os separa. Inventar essa fronteira a partir de 3 exemplos é exatamente o que produziu o problema do D-TARF-6 (a regra original foi escrita de dois acórdãos modernos e teria arrastado a fundamentação inteira para dentro do `assunto` em 14 mil documentos). A regra deve sair do **censo completo**, não da amostra.

⚠️ **O `tarf-anomalias.txt` é reescrito a cada rodada** (é inteiramente derivado). Os 18 de agora são parciais — o censo que vale é o da rodada que **terminar**.

## 5. Armadilhas que já custaram tempo

**`pgrep -f`/`pkill -f` casam com a própria shell do comando.** Perdi tempo duas vezes: uma matando meu próprio wrapper, outra achando que a coleta ainda rodava. Confirme com `ps -eo pid,comm | awk '$2 ~ /nome/'`.

**`%cpu` do `ps` é a média da vida do processo, não o consumo agora.** Um processo travado há uma hora ainda exibe número alto. Para saber se está vivo:

```bash
a=$(awk '{print $14+$15}' /proc/$P/stat); sleep 5
b=$(awk '{print $14+$15}' /proc/$P/stat); echo $((b-a))   # 0 = parado
```

**Quietude no monitor não é travamento.** O embedding é CPU pura e a coleção só escreve o pack ao terminar — disco parado e log mudo por horas é o normal.

**Binários de release** são exigidos por `build-packs.sh` (`auli`) e pelo índice de pareceres (`auli-collections`). Sem o segundo, o script apenas avisa e segue.

**`auli-collections` só roda de dentro de `auli-server/`** — resolve `../config/registry.toml` por caminho relativo.

**`auli update` carrega a coleção inteira na memória**: 24,7 GB de RSS no SP (15.605 pareceres). Nesta máquina (60 GB) não aperta; num servidor de 16 GB não passaria. Dívida registrada, não urgente — o remédio é o mesmo padrão do scraper do TARF (lotes).

**A landing do frontend é um mapa SVG.** Para dirigir o app num navegador, semeie `localStorage.setItem("auli.entity", "rs")` antes de navegar — não dá para clicar o estado por texto.

## 6. Migração de formato — encerrada, mas o backup importa

Os cinco passos do D-FMT-9 foram executados e verificados: 22.380 documentos convertidos (idempotência provada por segunda passada), 6.152 regenerados, 25.932 vetorizados, zips e deploy.

**`data_backup/` é a única cópia do formato antigo e NÃO está no git** (`data_backup*/` no `.gitignore`). Se ele sumir, não há reversão de dados possível. Reverter uma árvore:

```bash
rm -rf data/<id>/docs && cp -r data_backup/<id>/docs data/<id>/docs
```

## 7. Divergências conhecidas, registradas em teste

Nenhuma é bug; todas estão travadas por teste para não virarem descoberta futura.

**Compose de serviço com `titulo` vazio.** A fórmula antiga deixava linha em branco; a nova pula o slot. Esse estado não existe na árvore — o título é a identidade e a origem do nome do arquivo. Teste: `compose_de_servico_com_titulo_vazio_e_a_unica_divergencia_conhecida`.

**Prefixos `P:`/`R:` do embed de FAQ.** A `TAREFA-FORMATO-MD` (PR #128) descrevia a fórmula errada (a pré-`TAREFA-FAQ-PR`). A vigente tem os prefixos e a resposta dentro. Resolvido pondo os prefixos no **valor** dos slots, não na fórmula — quando o gate P5 os remover, some o mapeamento, não o compose.

**Um acórdão em 22.522 usa hífen** no número (`Acórdão do pleno 100-25`). Vai para o fim da lista sem data inventada, que é o comportamento projetado da `chave_cronologica`.

## 8. O que fica para depois (não bloqueia nada)

**Chat sobre acórdãos.** Hoje a aba do TARF é só listagem — não há pack, o serving não liga a coleção. Exige o §5 inteiro do `ESTUDO-colecoes-trait`: `TipoJurisprudencia`, loop do `update` parametrizado, prompt e seletor de fonte. O rótulo `"Acórdão TARF"` já está declarado em `rag.rs` com o comentário da fase 2, para o ponto de extensão existir nomeado.

**`KINDS_SEGURADOS` está vazio**, mas o mecanismo permanece em `bundle.rs`: a próxima coleção incompleta vai precisar dele.

**Documentos de decisão** ainda em `docs/`: `DISCOVERY-TARF`, `ESTUDO-TARF-formato`, `ESTUDO-colecoes-trait`, `TAREFA-DOCUMENTO`.

As TAREFAs `TARF-SCRAPER` (#127), `FORMATO-MD` (#128) e `MARCADORES` (#129) foram apagadas depois de implantadas — o commit de merge de cada uma é o registro, e o histórico do git tem o texto. O `PEDIDO-verificacoes-TARF` também saiu, mas por outro motivo: as três perguntas dele foram respondidas pela implementação, com mais evidência do que o relatório teria produzido — o nome `Consulta` não vazava em snapshot (virou `Documento` na #133), a ementa longa e em caixa alta do TARF está em produção, e o kind `pareceres` hardcoded virou o enum `Kind` (#132), com critério de grep provando que não sobrou string solta.
