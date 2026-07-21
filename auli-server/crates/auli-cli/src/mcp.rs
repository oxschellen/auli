//! Face MCP do Auli — servidor Model Context Protocol sobre o MESMO motor do chat.
//!
//! Três ferramentas (D-MCP-7), pensadas para a IA de um auditor/analista fiscal:
//!   - `listar_entidades`   — quais UFs têm acervo de pareceres indexado
//!   - `buscar_pareceres`   — busca semântica; devolve metadados + sinopse + link (sem corpo)
//!   - `obter_parecer`      — corpo integral de UM parecer, pelo número exato
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

use auli_retrieval::Engine;

use crate::entities;

/// Teto de top_k das buscas MCP (mesmo racional do /v1/retrieve).
const MAX_TOP_K: usize = 20;
const DEFAULT_TOP_K: usize = 5;

/// Kind único que a v1 expõe por MCP (D-MCP-7).
const KIND: &str = "pareceres";

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
pub struct ObterParecerArgs {
    /// Sigla da UF em minúsculas (ex.: "sc").
    pub uf: String,
    /// Número EXATO como devolvido por `buscar_pareceres` (ex.: "CONSULTA COPAT nº 0091/17").
    pub numero: String,
}

#[derive(Clone)]
pub struct AuliMcp {
    engine: Arc<Engine>,
    tool_router: ToolRouter<AuliMcp>,
}

#[tool_router]
impl AuliMcp {
    pub fn new(engine: Arc<Engine>) -> Self {
        Self { engine, tool_router: Self::tool_router() }
    }

    #[tool(
        description = "Lista as UFs (secretarias estaduais de Fazenda) com acervo de pareceres \
        tributários indexado no Auli, com o nome da secretaria e o total de documentos."
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
        tracing::info!(uf = %uf, top_k, hits = hits.len(), ms = t.elapsed().as_millis() as u64,
            "mcp buscar_pareceres");

        // JSON estruturado no content de texto: é o formato que assistentes consomem melhor.
        let json = serde_json::to_string_pretty(&hits)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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

        match achado {
            Some(p) => {
                let json = serde_json::to_string_pretty(&p)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
            }
            None => Ok(CallToolResult::success(vec![ContentBlock::text(erro_numero_nao_achado(
                &args.numero,
                &uf,
            ))])),
        }
    }
}

// ---- Funções livres: a lógica testável sem Engine-com-modelo, no espírito da D-MCP-4 ----

/// Texto do `listar_entidades`. Recebe o Engine porque só usa métodos que não embedam.
fn formatar_entidades(engine: &Engine) -> String {
    let linhas: Vec<String> = engine
        .entidades_com(KIND)
        .into_iter()
        .map(|id| {
            let nome = entities::get_entity(Some(&id)).map(|c| c.name.clone()).unwrap_or_else(|_| id.clone());
            let total = engine.store(&format!("{id}-{KIND}")).map(|s| s.len()).unwrap_or(0);
            format!("- {id} ({nome}): {total} pareceres")
        })
        .collect();
    if linhas.is_empty() {
        "Nenhuma UF com acervo de pareceres carregado.".to_string()
    } else {
        format!("UFs com acervo de pareceres:\n{}", linhas.join("\n"))
    }
}

fn erro_uf_sem_acervo(uf: &str) -> String {
    format!("UF '{uf}' sem acervo de pareceres. Use `listar_entidades`.")
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
                    .with_title("Auli — acervo de pareceres tributários estaduais"),
            )
            .with_instructions(
                "Acervo Auli de pareceres tributários estaduais brasileiros (conteúdo público \
                 das Secretarias da Fazenda, com links oficiais). Fluxo típico: \
                 `listar_entidades` → `buscar_pareceres` (uma busca por UF; para comparar \
                 estados, busque em cada UF) → `obter_parecer` para ler o corpo integral. \
                 Scores são distância cosseno: menor = mais próximo."
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
        c.insert("mg-pareceres".into(), Arc::new(ReadStore::from_records(vec![])));
        c
    }

    #[test]
    fn a_guarda_da_uf_recusa_tanto_a_inexistente_quanto_a_de_acervo_vazio() {
        // Esta é a asserção central do blocker 1 da revisão: os DOIS casos precisam ser recusados,
        // e o de acervo vazio é o que uma guarda por `store().is_some()` aceitaria por engano.
        let c = cols();
        let com_acervo = auli_retrieval::entidades_com(&c, KIND);

        assert!(com_acervo.contains(&"sc".to_string()), "sc tem acervo");
        assert!(!com_acervo.contains(&"mg".to_string()), "mg está registrada mas com store VAZIO");
        assert!(!com_acervo.contains(&"zz".to_string()), "zz não existe");

        // E o store de mg EXISTE — é exatamente por isso que a guarda não pode ser `is_some()`.
        assert!(c.get("mg-pareceres").is_some(), "o store vazio está no mapa");
    }

    #[test]
    fn mensagem_de_uf_sem_acervo_aponta_para_listar_entidades() {
        let msg = erro_uf_sem_acervo("mg");
        assert!(msg.contains("mg"), "cita a UF: {msg}");
        assert!(msg.contains("listar_entidades"), "ensina o próximo passo: {msg}");
    }

    #[test]
    fn mensagem_de_numero_nao_achado_ensina_o_proximo_passo() {
        let msg = erro_numero_nao_achado("PARECER Nº 999", "sc");
        assert!(msg.contains("PARECER Nº 999") && msg.contains("sc"), "msg: {msg}");
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
        // Mesmo contrato da rota HTTP: 1..=20, com 5 de padrão.
        assert_eq!(None.unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K), 5);
        assert_eq!(Some(9999usize).unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K), MAX_TOP_K);
        assert_eq!(Some(0usize).unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K), 1);
    }
}
