//! Face MCP do Auli — servidor Model Context Protocol sobre o MESMO motor do chat.
//!
//! Quatro ferramentas (D-MCP-7, ampliada nesta tarefa), pensadas para a IA de um
//! auditor/analista fiscal e para o atendimento ao contribuinte:
//!   - `listar_entidades`          — o que cada UF tem indexado (pareceres, serviços, FAQs)
//!   - `buscar_pareceres`          — busca semântica; metadados + sinopse + link (sem corpo)
//!   - `obter_parecer`             — corpo integral de UM parecer, pelo número exato
//!   - `consultar_servicos_faqs`   — serviços + FAQs no MESMO bloco de contexto do chat
//!
//! Privacidade (D-MCP-5): a pergunta é embedada localmente e NUNCA sai do processo; nenhum LLM
//! externo é chamado neste caminho; o tracing registra só metadados.

use std::sync::Arc;
use std::time::Instant;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};

use auli_anon::{Anonimizador, TEXTO_FALLBACK_ERRO};
use auli_contract::Kind;
use auli_core::corpus::{FAQS, SERVICES};
use auli_retrieval::Engine;

use crate::rag::TemposConsulta;
use crate::{entities, rag};

/// Teto de top_k das buscas MCP (mesmo racional do /v1/retrieve).
const MAX_TOP_K: usize = 20;
const DEFAULT_TOP_K: usize = 5;

/// Kind dos pareceres — o único que a v1 expunha por MCP (D-MCP-7).
const KIND: &str = Kind::Pareceres.as_str();

/// Kinds da consulta Serviços+FAQs — a mesma dupla do tipo `ServicosFaqs` do chat.
const KIND_SERVICOS: &str = Kind::Servicos.as_str();
const KIND_FAQS: &str = Kind::Faqs.as_str();

/// Kinds que o `listar_entidades` reporta, na ordem de exibição.
const KINDS_LISTADOS: [&str; 3] = [KIND, KIND_SERVICOS, KIND_FAQS];

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BuscarPareceresArgs {
    /// Sigla da UF em minúsculas (ex.: "rs", "sc", "sp", "pr"). Use `listar_entidades` para ver
    /// as disponíveis.
    pub uf: String,
    /// Pergunta ou tema em linguagem natural (ex.: "crédito de ICMS na aquisição de energia
    /// elétrica pela indústria").
    pub pergunta: String,
    /// Quantos resultados devolver (1 a 20; padrão 5).
    #[serde(default)]
    pub top_k: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConsultarServicosFaqsArgs {
    /// Sigla da UF em minúsculas (ex.: "rs", "sp"). Use `listar_entidades` para ver as
    /// disponíveis.
    pub uf: String,
    /// Pergunta em linguagem natural, como um contribuinte faria (ex.: "como emitir a segunda
    /// via da guia do IPVA?").
    pub pergunta: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ObterParecerArgs {
    /// Sigla da UF em minúsculas (ex.: "sc").
    pub uf: String,
    /// Número EXATO como devolvido por `buscar_pareceres` (ex.: "CONSULTA COPAT nº 0091/17").
    pub numero: String,
}

#[derive(Clone)]
pub struct AuliMcp {
    engine: Arc<Engine>,
    /// Usado SÓ para produzir a seção `PERGUNTA (ANONIMIZADA)` do registro de auditoria. Este
    /// caminho não chama LLM externo (D-MCP-5), então não há texto a proteger na saída — o par
    /// original/anonimizada existe pelo mesmo motivo do chat: auditar o que o `auli-anon` deixa
    /// passar (§7.0 do `auli_operations.md`).
    anonimizador: Arc<Anonimizador>,
    tool_router: ToolRouter<AuliMcp>,
}

#[tool_router]
impl AuliMcp {
    pub fn new(engine: Arc<Engine>, anonimizador: Arc<Anonimizador>) -> Self {
        Self {
            engine,
            anonimizador,
            tool_router: Self::tool_router(),
        }
    }

    /// Grava o registro de auditoria de uma chamada MCP pelo **mesmo caminho do chat**: mesmo
    /// diretório (`$AULI_LOG_DIR`), mesmas permissões (0700/0600), mesmo formato — sem a seção
    /// `RESPOSTA`, que aqui não existe (nenhum LLM é chamado; quem redige é o assistente do
    /// usuário).
    ///
    /// **O que NÃO é registrado, por construção:** nome, IP, sessão ou qualquer identificador de
    /// quem chamou. O registro é `pergunta + horário + o que foi devolvido` — a mesma doutrina da
    /// §7.0, que troca privacidade-no-disco por auditabilidade, com os controles compensatórios
    /// de acesso e retenção.
    ///
    /// Falha de gravação **não derruba a chamada**: é uma API pública somente-leitura, e recusar
    /// uma consulta porque o disco falhou seria pior que perder um registro. Diverge aqui do chat
    /// (que propaga o erro) de propósito.
    ///
    /// `aderencia` é a seção de proximidade já montada (vazia quando a chamada não fez busca
    /// vetorial — só o `obter_parecer`, que vai pelo número).
    fn registrar(
        &self,
        uf: &str,
        ferramenta: &str,
        pergunta: &str,
        aderencia: &str,
        devolvido: &str,
        tempos: TemposConsulta,
    ) {
        let anonimizada = match self.anonimizador.anonimizar(pergunta) {
            Ok(a) => a.texto,
            Err(e) => {
                tracing::warn!("anonimização falhou no log MCP: {e}");
                TEXTO_FALLBACK_ERRO.to_string()
            }
        };
        if let Err(e) = rag::log_question(&rag::RegistroConsulta {
            entidade: uf,
            tipo: &format!("mcp:{ferramenta}"),
            original: pergunta,
            sanitizada: &anonimizada,
            answer: None, // sem LLM neste caminho
            aderencia,
            rag: devolvido,
            tempos,
        }) {
            tracing::warn!("falha ao gravar o log da chamada MCP: {e}");
        }
    }

    #[tool(
        description = "Lista as UFs (secretarias estaduais de Fazenda) com acervo indexado no \
        Auli, informando o que cada uma tem e os totais: pareceres tributários (para \
        `buscar_pareceres`/`obter_parecer`), serviços de atendimento e FAQs (para \
        `consultar_servicos_faqs`)."
    )]
    fn listar_entidades(&self) -> Result<CallToolResult, McpError> {
        let texto = formatar_entidades(&self.engine);
        Ok(CallToolResult::success(vec![ContentBlock::text(texto)]))
    }

    #[tool(
        description = "Busca semântica no acervo de pareceres tributários de uma UF. Devolve \
        para cada resultado: número, assunto (ementa), sinopse com palavras-chave, link oficial e \
        score de proximidade (menor = mais próximo). NÃO devolve o corpo integral — use \
        `obter_parecer` com o número para lê-lo."
    )]
    async fn buscar_pareceres(
        &self,
        Parameters(args): Parameters<BuscarPareceresArgs>,
    ) -> Result<CallToolResult, McpError> {
        let uf = args.uf.trim().to_lowercase();

        // Guarda ANTES do embed, e pelo teste CERTO: com o `load_all` do auli-cli, toda entidade
        // registrada tem store de pareceres (possivelmente VAZIO), então `store().is_some()` não
        // distingue nada. `entidades_com` exige store não-vazio = "tem acervo de verdade".
        // De quebra, este caminho de erro fica testável sem carregar o modelo.
        if !self.engine.entidades_com(KIND).contains(&uf) {
            return Err(McpError::invalid_params(erro_uf_sem_acervo(&uf), None));
        }

        let top_k = args.top_k.unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K);
        let engine = self.engine.clone();
        let pergunta = args.pergunta.clone();
        let uf2 = uf.clone();

        // Embed + scan são CPU-bound: fora do runtime async (mesma disciplina das outras faces).
        let t = Instant::now();
        let hits = tokio::task::spawn_blocking(move || {
            engine.search_pareceres(&uf2, &pergunta, top_k, 0, f32::INFINITY)
        })
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        // `ColecaoAusente` é inalcançável aqui (guarda acima); o que restar é interno de verdade.
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // D-MCP-5: só metadados no log — nunca o texto da pergunta.
        let ms = t.elapsed().as_millis() as u64;
        tracing::info!(uf = %uf, top_k, hits = hits.len(), ms, "mcp buscar_pareceres");

        // JSON estruturado no content de texto: é o formato que assistentes consomem melhor.
        let json = serde_json::to_string_pretty(&hits)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        // `search_pareceres` funde embed + varredura, então o split não existe aqui: o tempo vai
        // inteiro em `retrieve_ms` (rotulado "retrieve+montagem" na linha TEMPOS).
        // O JSON devolvido já carrega o score de cada parecer; a seção é montada assim mesmo para
        // que o registro tenha o MESMO formato nas quatro ferramentas e no chat.
        let aderencia =
            rag::montar_aderencia(&rag::aderencia("parecer", hits.iter().map(|h| h.score)));
        self.registrar(
            &uf,
            "buscar_pareceres",
            &args.pergunta,
            &aderencia,
            &json,
            TemposConsulta {
                retrieve_ms: ms,
                total_ms: ms,
                ..Default::default()
            },
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "Devolve o corpo integral de um parecer tributário de uma UF, dado o \
        número exato (como devolvido por `buscar_pareceres`). Inclui assunto, sinopse e link \
        oficial."
    )]
    async fn obter_parecer(
        &self,
        Parameters(args): Parameters<ObterParecerArgs>,
    ) -> Result<CallToolResult, McpError> {
        let uf = args.uf.trim().to_lowercase();

        // Mesma guarda do `buscar_pareceres`, pelo mesmo motivo.
        if !self.engine.entidades_com(KIND).contains(&uf) {
            return Err(McpError::invalid_params(erro_uf_sem_acervo(&uf), None));
        }

        let numero = args.numero.clone();
        let engine = self.engine.clone();
        let uf2 = uf.clone();

        // I/O de disco + varredura da lista: também fora do runtime.
        let t = Instant::now();
        let achado = tokio::task::spawn_blocking(move || engine.parecer_por_numero(&uf2, &numero))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        tracing::info!(uf = %uf, achou = achado.is_some(), ms = t.elapsed().as_millis() as u64,
            "mcp obter_parecer");

        let ms = t.elapsed().as_millis() as u64;
        let devolvido = match &achado {
            Some(p) => serde_json::to_string_pretty(p)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
            None => erro_numero_nao_achado(&args.numero, &uf),
        };
        // Aqui a "pergunta" é o número pedido — é o que o usuário digitou nesta chamada.
        self.registrar(
            &uf,
            "obter_parecer",
            &args.numero,
            "", // sem busca vetorial: o parecer é achado pelo número exato
            &devolvido,
            TemposConsulta {
                retrieve_ms: ms,
                total_ms: ms,
                ..Default::default()
            },
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(devolvido)]))
    }

    #[tool(
        description = "Consulta os serviços de atendimento e as perguntas frequentes (FAQs) \
        de uma UF. Devolve UM bloco de texto com os serviços e FAQs mais próximos da pergunta \
        (até 10 serviços e até 20 FAQs) — o MESMO contexto que o chat do portal Auli usa para \
        responder. Ideal para dúvidas de 'como fazer' do contribuinte (guias, cadastros, \
        certidões, parcelamentos, prazos de atendimento); para fundamentação tributária \
        (interpretação da legislação), prefira `buscar_pareceres`."
    )]
    async fn consultar_servicos_faqs(
        &self,
        Parameters(args): Parameters<ConsultarServicosFaqsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let uf = args.uf.trim().to_lowercase();

        // Guarda no padrão das outras ferramentas, mas em DUPLA: a UF serve se tiver serviços
        // OU FAQs (hoje só o RS tem FAQs; quase todas as UFs têm serviços). O caso "só um dos
        // dois" NÃO é erro — o chat tem a mesma tolerância e a parte ausente sai vazia.
        let tem_svc = self.engine.entidades_com(KIND_SERVICOS).contains(&uf);
        let tem_faq = self.engine.entidades_com(KIND_FAQS).contains(&uf);
        if !tem_svc && !tem_faq {
            return Err(McpError::invalid_params(
                erro_uf_sem_servicos_faqs(&uf),
                None,
            ));
        }

        // Embed UMA vez, fora do runtime async (CPU-bound, mesma disciplina das outras faces).
        // D-MCP-5: a pergunta nunca sai do processo.
        let t = Instant::now();
        let engine = self.engine.clone();
        let pergunta = args.pergunta.clone();
        let embedding = tokio::task::spawn_blocking(move || engine.embed(&pergunta))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let embed_ms = t.elapsed().as_millis() as u64;
        let t_ret = Instant::now();

        // As MESMAS constantes do chat — a paridade byte a byte depende disso. `rag::retrieve`
        // é tolerante a coleção ausente (contribui vazio), exatamente como no chat.
        let svc_fut = rag::retrieve(
            self.engine.clone(),
            format!("{uf}-{KIND_SERVICOS}"),
            "svc",
            embedding.clone(),
            SERVICES.n_results,
            rag::SVC_FLOOR,
            rag::SVC_BAND,
        );
        let faq_fut = rag::retrieve(
            self.engine.clone(),
            format!("{uf}-{KIND_FAQS}"),
            "faq",
            embedding,
            FAQS.n_results,
            rag::FAQ_FLOOR,
            rag::FAQ_BAND,
        );
        let (svc_hits, faq_hits) = tokio::try_join!(svc_fut, faq_fut)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // D-MCP-5: só metadados no log — nunca o texto da pergunta.
        tracing::info!(uf = %uf, servicos = svc_hits.len(), faqs = faq_hits.len(),
            ms = t.elapsed().as_millis() as u64, "mcp consultar_servicos_faqs");

        let bloco =
            rag::montar_rag_servicos_faqs(&rag::payloads(&svc_hits), &rag::payloads(&faq_hits));
        // Os mesmos rótulos do chat — mesma pergunta, mesmos números nos dois registros.
        let mut itens = rag::aderencia("servico", svc_hits.iter().map(|h| Some(h.score)));
        itens.extend(rag::aderencia(
            "faq",
            faq_hits.iter().map(|h| Some(h.score)),
        ));
        let aderencia = rag::montar_aderencia(&itens);
        // Único caminho MCP onde embed e retrieve são medidos separadamente — é o que torna a
        // linha TEMPOS comparável com a do chat para a MESMA pergunta.
        self.registrar(
            &uf,
            "consultar_servicos_faqs",
            &args.pergunta,
            &aderencia,
            &bloco,
            TemposConsulta {
                embed_ms,
                retrieve_ms: t_ret.elapsed().as_millis() as u64,
                total_ms: t.elapsed().as_millis() as u64,
                ..Default::default()
            },
        );
        Ok(CallToolResult::success(vec![ContentBlock::text(bloco)]))
    }
}

// ---- Funções livres: a lógica testável sem Engine-com-modelo, no espírito da D-MCP-4 ----

/// Coleta do `listar_entidades`: só toca o Engine (métodos que não embedam) e delega a montagem
/// do texto a [`linhas_entidades`]. A separação é a D-MCP-4 na prática — a lógica de exibição
/// (união das UFs entre kinds, omissão de kind vazio, ordem) fica testável sem construir um
/// `Engine`, que carregaria o BGE-M3.
fn formatar_entidades(engine: &Engine) -> String {
    use std::collections::BTreeSet;

    // BTreeSet: a UF aparece uma vez só e a ordem alfabética sai de graça.
    let mut ufs: BTreeSet<String> = BTreeSet::new();
    for kind in KINDS_LISTADOS {
        ufs.extend(engine.entidades_com(kind));
    }

    let dados: Vec<AcervoUf> = ufs
        .into_iter()
        .map(|id| {
            let nome = entities::get_entity(Some(&id))
                .map(|c| c.name.clone())
                .unwrap_or_else(|_| id.clone());
            let totais = KINDS_LISTADOS
                .iter()
                .map(|&kind| {
                    let total = engine
                        .store(&format!("{id}-{kind}"))
                        .map(|s| s.len())
                        .unwrap_or(0);
                    (kind, total)
                })
                .collect();
            (id, nome, totais)
        })
        .collect();

    linhas_entidades(&dados)
}

/// Uma UF na listagem: `(id, nome da secretaria, [(kind, total)])`.
type AcervoUf = (String, String, Vec<(&'static str, usize)>);

/// A parte PURA do `listar_entidades`: dados já coletados → texto. Kind com zero documentos é
/// OMITIDO da linha (mesma semântica de `entidades_com`, que é quem decide se a UF entra na lista).
fn linhas_entidades(dados: &[AcervoUf]) -> String {
    if dados.is_empty() {
        return "Nenhuma UF com acervo carregado.".to_string();
    }
    let linhas: Vec<String> = dados
        .iter()
        .map(|(id, nome, totais)| {
            let partes: Vec<String> = totais
                .iter()
                .filter(|(_, total)| *total > 0)
                .map(|(kind, total)| format!("{total} {kind}"))
                .collect();
            format!("- {id} ({nome}): {}", partes.join(", "))
        })
        .collect();
    format!("UFs com acervo indexado:\n{}", linhas.join("\n"))
}

fn erro_uf_sem_acervo(uf: &str) -> String {
    format!("UF '{uf}' sem acervo de pareceres. Use `listar_entidades`.")
}

fn erro_uf_sem_servicos_faqs(uf: &str) -> String {
    format!("UF '{uf}' sem acervo de serviços nem de FAQs. Use `listar_entidades`.")
}

fn erro_numero_nao_achado(numero: &str, uf: &str) -> String {
    format!(
        "Nenhum parecer com número '{numero}' na UF '{uf}'. Confira o número exato via \
         `buscar_pareceres`."
    )
}

// `router = self.tool_router` aponta o macro para o campo montado UMA vez no `new()`. Sem isso o
// default é `Self::tool_router()`, que reconstrói o roteador a cada `list_tools`/`call_tool` (e
// deixa o campo morto — foi o aviso de dead_code que revelou a diferença).
#[tool_handler(router = self.tool_router)]
impl ServerHandler for AuliMcp {
    fn get_info(&self) -> ServerInfo {
        // NÃO usar `Implementation::from_build_env()`: o `env!` dele expande no build do RMCP, e
        // o servidor se anunciaria como "rmcp 2.2.0" para o assistente. O nome que o auditor vê
        // no cliente MCP tem que ser o nosso.
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("auli", env!("CARGO_PKG_VERSION"))
                    .with_title("Auli — acervo tributário das Fazendas estaduais"),
            )
            .with_instructions(
                "Acervo Auli das Secretarias da Fazenda estaduais brasileiras (conteúdo \
                 público, com links oficiais). Dois fluxos: (1) fundamentação tributária — \
                 `listar_entidades` → `buscar_pareceres` (uma busca por UF) → `obter_parecer` \
                 para o corpo integral; scores são distância cosseno, menor = mais próximo. \
                 (2) atendimento ao contribuinte ('como fazer') — `consultar_servicos_faqs`, \
                 que devolve os serviços e FAQs da UF num bloco de texto pronto para \
                 fundamentar a resposta."
                    .to_string(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auli_retrieval::Collections;
    use std::sync::Arc;
    use vector_store::{ReadStore, Record};

    fn store_de(payloads: Vec<&str>) -> Arc<ReadStore<String>> {
        let records = payloads
            .into_iter()
            .enumerate()
            .map(|(i, p)| Record {
                id: format!("id-{i}"),
                embedding: vec![1.0, 0.0],
                payload: p.to_string(),
            })
            .collect();
        Arc::new(ReadStore::from_records(records))
    }

    /// Coleções sintéticas: `sc` com acervo, `mg` registrada mas VAZIA (o caso que
    /// `store().is_some()` deixaria passar), `zz` nem existe.
    fn cols() -> Collections {
        let mut c = Collections::new();
        c.insert("sc-pareceres".into(), store_de(vec!["{}"]));
        c.insert(
            "mg-pareceres".into(),
            Arc::new(ReadStore::from_records(vec![])),
        );
        c
    }

    #[test]
    fn a_guarda_da_uf_recusa_tanto_a_inexistente_quanto_a_de_acervo_vazio() {
        // Esta é a asserção central do blocker 1 da revisão: os DOIS casos precisam ser recusados,
        // e o de acervo vazio é o que uma guarda por `store().is_some()` aceitaria por engano.
        let c = cols();
        let com_acervo = auli_retrieval::entidades_com(&c, KIND);

        assert!(com_acervo.contains(&"sc".to_string()), "sc tem acervo");
        assert!(
            !com_acervo.contains(&"mg".to_string()),
            "mg está registrada mas com store VAZIO"
        );
        assert!(!com_acervo.contains(&"zz".to_string()), "zz não existe");

        // E o store de mg EXISTE — é exatamente por isso que a guarda não pode ser `is_some()`.
        assert!(c.contains_key("mg-pareceres"), "o store vazio está no mapa");
    }

    #[test]
    fn mensagem_de_uf_sem_acervo_aponta_para_listar_entidades() {
        let msg = erro_uf_sem_acervo("mg");
        assert!(msg.contains("mg"), "cita a UF: {msg}");
        assert!(
            msg.contains("listar_entidades"),
            "ensina o próximo passo: {msg}"
        );
    }

    #[test]
    fn mensagem_de_numero_nao_achado_ensina_o_proximo_passo() {
        let msg = erro_numero_nao_achado("PARECER Nº 999", "sc");
        assert!(
            msg.contains("PARECER Nº 999") && msg.contains("sc"),
            "msg: {msg}"
        );
        assert!(msg.contains("buscar_pareceres"), "msg: {msg}");
    }

    #[test]
    fn listar_entidades_ignora_store_vazio_e_ordena() {
        // A lógica do texto, sem Engine: `entidades_com` é a fonte da listagem.
        let mut c = cols();
        c.insert("rs-pareceres".into(), store_de(vec!["{}", "{}"]));
        assert_eq!(auli_retrieval::entidades_com(&c, KIND), vec!["rs", "sc"]);
    }

    #[test]
    fn top_k_e_limitado_como_no_retrieve() {
        // Mesmo contrato da rota HTTP: 1..=20, com 5 de padrão. A expressão vem por closure, e não
        // aplicada a `None`/`Some(_)` literal, porque o literal dispara `unnecessary_literal_unwrap`
        // — e a reescrita que o lint sugere (o valor já desembrulhado) apagaria justamente o que se
        // testa aqui: que a `Option` do argumento cai no padrão.
        let efetivo = |top_k: Option<usize>| top_k.unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K);
        assert_eq!(efetivo(None), 5);
        assert_eq!(efetivo(Some(9999)), MAX_TOP_K);
        assert_eq!(efetivo(Some(0)), 1);
    }

    // ---- `consultar_servicos_faqs` ----

    #[test]
    fn a_guarda_de_servicos_faqs_e_um_ou_para_os_dois_kinds() {
        // O desenho da guarda: a UF entra se tiver serviços OU FAQs; só a dupla ausência recusa.
        // Exercitado via `entidades_com`, que é o que a guarda consulta — e que também aplica o
        // critério de store NÃO-VAZIO (o blocker 1 da revisão dos pareceres, aqui de graça).
        let mut c = Collections::new();
        c.insert("sp-servicos".into(), store_de(vec!["{}"])); // só serviços
        c.insert("rs-servicos".into(), store_de(vec!["{}"]));
        c.insert("rs-faqs".into(), store_de(vec!["{}"])); // serviços E faqs
        c.insert("xx-faqs".into(), store_de(vec!["{}"])); // só faqs (hipotético)
        c.insert(
            "mg-servicos".into(),
            Arc::new(ReadStore::from_records(vec![])), // registrada, mas VAZIA
        );

        let svc = auli_retrieval::entidades_com(&c, KIND_SERVICOS);
        let faq = auli_retrieval::entidades_com(&c, KIND_FAQS);
        let entra = |uf: &str| svc.contains(&uf.to_string()) || faq.contains(&uf.to_string());

        assert!(entra("sp"), "só serviços basta");
        assert!(entra("rs"), "os dois, com mais razão");
        assert!(entra("xx"), "só faqs basta");
        assert!(!entra("mg"), "store vazio não conta");
        assert!(!entra("zz"), "inexistente recusa");
    }

    #[test]
    fn mensagem_de_uf_sem_servicos_faqs_aponta_para_listar_entidades() {
        let msg = erro_uf_sem_servicos_faqs("mg");
        assert!(msg.contains("mg"), "cita a UF: {msg}");
        assert!(
            msg.contains("listar_entidades"),
            "ensina o próximo passo: {msg}"
        );
    }

    // ---- `listar_entidades`: a montagem do texto (parte pura) ----

    #[test]
    fn linhas_entidades_omite_kind_zerado_e_preserva_a_ordem() {
        // A regra que importa: kind com zero documentos NÃO aparece na linha. Sem isso, uma UF
        // sem FAQs (que é quase toda) anunciaria "0 faqs" e convidaria o assistente a chamar
        // `consultar_servicos_faqs` esperando FAQs que não existem.
        let dados: Vec<AcervoUf> = vec![
            (
                "rs".into(),
                "SEFAZ-RS".into(),
                vec![("pareceres", 2905), ("servicos", 586), ("faqs", 1937)],
            ),
            (
                "sp".into(),
                "SEFAZ-SP".into(),
                vec![("pareceres", 16123), ("servicos", 537), ("faqs", 0)],
            ),
        ];
        assert_eq!(
            linhas_entidades(&dados),
            "UFs com acervo indexado:\n\
             - rs (SEFAZ-RS): 2905 pareceres, 586 servicos, 1937 faqs\n\
             - sp (SEFAZ-SP): 16123 pareceres, 537 servicos"
        );
    }

    #[test]
    fn linhas_entidades_sem_dados_avisa_em_vez_de_devolver_vazio() {
        // Server sem pack nenhum carregado: o assistente precisa ler uma frase, não uma string
        // vazia que ele interpretaria como falha da ferramenta.
        assert_eq!(linhas_entidades(&[]), "Nenhuma UF com acervo carregado.");
    }
}
