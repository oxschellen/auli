//! `GET /v1/log/{uuid}` — devolve o registro de auditoria de UMA consulta a quem tem o UUID dela.
//!
//! **O modelo de segurança é a *capability*, não a autenticação.** O UUID v7 nasce no
//! [`crate::rag::log_question`], vai no campo `log_id` da resposta do chat e, portanto, só chega a
//! quem fez a pergunta. Não existe rota que liste, conte ou enumere logs — sem o UUID não há como
//! chegar a um arquivo. Isso mantém a premissa da §7.0 do `auli_operations.md`: o log é
//! deliberadamente ÍNTEGRO (contém a pergunta crua, com PII) e a proteção dele é de **acesso**;
//! esta rota só troca "acesso ao filesystem da máquina" por "posse do UUID daquela consulta".
//!
//! Cada um lê apenas o que ele mesmo digitou, mais mecânica interna sobre acervo público (contexto
//! RAG, distâncias, tempos).

use std::path::{Path, PathBuf};

use axum::{
    extract::Path as AxumPath,
    http::{StatusCode, header},
    response::IntoResponse,
};
use tracing::warn;
use uuid::Uuid;

/// Mesma resposta para "nunca existiu" e para "foi podado pela retenção" — a distinção é
/// justamente o que não se quer revelar a quem chuta UUIDs.
const NAO_ENCONTRADO: &str = "Log expirado ou inexistente.";

/// Corpo do 400. Não ecoa o que o cliente mandou: devolver a entrada num `text/plain` é como se
/// abre a porta para XSS refletido quando alguém, um dia, renderiza esta resposta como HTML.
const FORMATO_INVALIDO: &str = "Identificador inválido.";

pub async fn log_handler(AxumPath(id): AxumPath<String>) -> impl IntoResponse {
    // **A validação de formato É a defesa contra path traversal**, e por isso vem ANTES de tocar o
    // disco. Nada de sanitizar caminho (`..`, `/`, `%2e%2e`, unicode homoglyph, dupla decodificação
    // — a lista nunca fecha): o que não é um UUID canônico não vira caminho nenhum. `Uuid::parse_str`
    // é a peneira, e o `id` original é descartado logo em seguida em favor do UUID re-serializado
    // pela biblioteca, para que nem uma variação de grafia (maiúsculas, chaves) atravesse daqui.
    let Ok(uuid) = Uuid::parse_str(&id) else {
        return (StatusCode::BAD_REQUEST, texto(), FORMATO_INVALIDO).into_response();
    };

    let dir = crate::rag::log_dir();
    match localizar(Path::new(&dir), uuid) {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(conteudo) => (StatusCode::OK, texto(), conteudo).into_response(),
            // Achou pelo nome mas não conseguiu ler (permissão, corrida com o expurgo). Do ponto de
            // vista de quem chama é indistinguível de não existir, e é assim que deve ser.
            Err(e) => {
                warn!("log {uuid} localizado mas ilegível: {e}");
                (StatusCode::NOT_FOUND, texto(), NAO_ENCONTRADO).into_response()
            }
        },
        None => (StatusCode::NOT_FOUND, texto(), NAO_ENCONTRADO).into_response(),
    }
}

/// `charset=utf-8` explícito: o registro tem acentuação (`ADERÊNCIA`, `PERGUNTA`) e sem o charset
/// o browser adivinha — e adivinha latin-1 com frequência suficiente para virar bug de exibição.
fn texto() -> [(header::HeaderName, &'static str); 1] {
    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")]
}

/// Procura o arquivo cujo nome termina em `_{uuid}.txt` — primeiro na SUBPASTA do mês derivada do
/// próprio UUID (D-LOG-6) e, no miss, na raiz plana onde vivem os logs anteriores à subpasta
/// (D-LOG-7: legado congelado, sem migração — coerente com a D-LOG-1 —, morre com a retenção).
///
/// A varredura deixa de ser da retenção inteira e passa a ser de UM mês: o custo por requisição
/// fica limitado pelo volume mensal, não pelo histórico — inclusive para quem chuta UUIDs e recebe
/// 404, que antes varria tudo para concluir que não havia nada. O nome exato não é reconstruível:
/// o prefixo vem de `chrono::Local::now()`, um relógio DIFERENTE do embutido no UUID, e os dois
/// podem divergir de segundo — daí varrer o mês em vez de abrir direto.
///
/// Comparação por sufixo do nome, nunca por `contains`: um UUID que aparecesse no MEIO de outro
/// nome não pode casar. Os logs anteriores a 09/08/2026 não têm sufixo de UUID e portanto seguem
/// inalcançáveis por aqui, nos DOIS passos — decisão da D-LOG-1 (sem migração retroativa). Também
/// é o que impede que um arquivo antigo com DOIS registros colados (a colisão de mesmo segundo que
/// a D-LOG-1 conserta) seja servido.
fn localizar(dir: &Path, uuid: Uuid) -> Option<PathBuf> {
    // UUID sem timestamp (não-v7) nunca foi emitido como capability: 404 sem tocar o disco
    // (D-LOG-8) — quem chuta formatos não compra varredura nenhuma.
    let mes = crate::rag::mes_do_log(uuid)?;
    let sufixo = format!("_{uuid}.txt");
    achar_por_sufixo(&dir.join(mes), &sufixo).or_else(|| achar_por_sufixo(dir, &sufixo))
}

/// Varre UM diretório (sem recursão) atrás do sufixo. Diretório ausente é `None`, não erro — é o
/// mês que ainda não recebeu log, ou a instalação que ainda não recebeu consulta.
fn achar_por_sufixo(dir: &Path, sufixo: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(sufixo))
        })
}

#[cfg(test)]
mod tests {
    use super::{NAO_ENCONTRADO, localizar};
    use std::fs;
    use uuid::Uuid;

    /// O caminho feliz: nome com prefixo de data + sufixo de UUID é encontrado pelo UUID sozinho,
    /// na SUBPASTA do mês (D-LOG-6) — que é onde a gravação põe e onde a varredura começa.
    #[test]
    fn localiza_pelo_sufixo_sem_conhecer_o_prefixo() {
        let dir = tempdir();
        let id = Uuid::now_v7();
        let mes = mes_de(&dir, id);
        fs::create_dir_all(&mes).unwrap();
        let alvo = mes.join(format!("2026-08-09_10-15-05_{id}.txt"));
        fs::write(&alvo, "conteúdo").unwrap();
        fs::write(mes.join("2026-08-09_10-15-04_outro.txt"), "vizinho").unwrap();

        assert_eq!(localizar(&dir, id), Some(alvo));
    }

    /// Regressão da doutrina do sufixo: casar por `contains` deixaria um nome forjado — um arquivo
    /// cujo nome CONTÉM o UUID mas não termina nele — ser servido no lugar do log verdadeiro.
    ///
    /// **São dois fixtures, e só o segundo separa `contains` de `ends_with`.** O primeiro
    /// (`_{id}_extra.txt`) não contém sequer o sufixo `_{id}.txt`, então nenhuma das duas
    /// implementações casa com ele — ele exercita a doutrina, mas não a defende. O que defende é
    /// `_{id}.txt.bak`: `contains` o serviria, `ends_with` não. Sem esse caso o teste passava
    /// verde com o defeito reposto (medido na verificação por mutação da D-LOG-6).
    #[test]
    fn nao_casa_uuid_no_meio_do_nome() {
        let dir = tempdir();
        let id = Uuid::now_v7();
        let mes = mes_de(&dir, id);
        fs::create_dir_all(&mes).unwrap();
        fs::write(mes.join(format!("2026-08-09_10-15-05_{id}_extra.txt")), "x").unwrap();
        fs::write(mes.join(format!("2026-08-09_10-15-05_{id}.txt.bak")), "x").unwrap();

        assert_eq!(localizar(&dir, id), None);
    }

    /// Os arquivos anteriores à D-LOG-1 (só timestamp, sem UUID) não são alcançáveis. É o que
    /// garante que um log antigo com dois registros colados nunca seja servido por esta rota.
    #[test]
    fn log_antigo_sem_uuid_e_inalcancavel() {
        let dir = tempdir();
        fs::write(dir.join("2026-08-08_22-41-18.txt"), "registro antigo").unwrap();

        assert_eq!(localizar(&dir, Uuid::now_v7()), None);
    }

    /// Diretório inexistente devolve `None`, não panica: é o estado de uma instalação que ainda
    /// não recebeu consulta nenhuma.
    #[test]
    fn diretorio_ausente_nao_panica() {
        let dir = tempdir().join("nao-existe");
        assert_eq!(localizar(&dir, Uuid::now_v7()), None);
    }

    /// A mensagem do 404 é a MESMA para inexistente e para podado — não revela qual dos dois.
    #[test]
    fn mensagem_do_404_nao_distingue_os_dois_casos() {
        assert!(!NAO_ENCONTRADO.to_lowercase().contains("podad"));
        assert!(!NAO_ENCONTRADO.to_lowercase().contains("nunca"));
    }

    /// D-LOG-7: log anterior à subpasta (raiz plana) continua alcançável — o fallback existe para
    /// os `log_id` emitidos entre 09/08 e a subpasta mensal, que não têm para onde migrar.
    /// Verificação por mutação: remover o `.or_else` do `localizar` mata este teste.
    #[test]
    fn legado_na_raiz_e_achado_no_fallback() {
        let dir = tempdir();
        let id = Uuid::now_v7();
        let alvo = dir.join(format!("2026-08-15_10-00-00_{id}.txt"));
        fs::write(&alvo, "legado").unwrap();

        assert_eq!(localizar(&dir, id), Some(alvo));
    }

    /// D-LOG-8: UUID sem timestamp (não-v7) devolve `None` SEM varrer nada — o arquivo plantado na
    /// raiz com o sufixo dele existe, e mesmo assim não é achado, porque a busca nem começa.
    /// Verificação por mutação: remover o early-return do `localizar` faz o fallback ACHAR este
    /// arquivo e o teste morrer.
    #[test]
    fn uuid_sem_timestamp_nao_compra_varredura() {
        let dir = tempdir();
        let id = Uuid::new_v4();
        fs::write(dir.join(format!("2026-08-15_10-00-00_{id}.txt")), "x").unwrap();

        assert_eq!(localizar(&dir, id), None);
    }

    /// A subpasta do mês do UUID, dentro do `dir` do teste — pela MESMA função que a produção usa,
    /// e não por um mês copiado à mão, que envelheceria em silêncio.
    fn mes_de(dir: &std::path::Path, id: Uuid) -> std::path::PathBuf {
        dir.join(crate::rag::mes_do_log(id).unwrap())
    }

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("auli-log-{}", Uuid::now_v7()));
        fs::create_dir_all(&base).unwrap();
        base
    }
}
