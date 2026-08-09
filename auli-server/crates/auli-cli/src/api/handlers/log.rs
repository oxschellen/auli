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

/// Procura o arquivo cujo nome termina em `_{uuid}.txt`.
///
/// Varre o diretório em vez de reconstruir o nome porque o **prefixo é o timestamp da gravação**,
/// que o chamador não conhece. Comparação por sufixo do nome, nunca por `contains`: um UUID que
/// aparecesse no MEIO de outro nome não pode casar.
///
/// Os logs anteriores a 09/08/2026 não têm sufixo de UUID e portanto são inalcançáveis por aqui —
/// decisão da D-LOG-1 (sem migração retroativa). Também é o que impede que um arquivo antigo com
/// DOIS registros colados (a colisão de mesmo segundo que a D-LOG-1 conserta) seja servido.
fn localizar(dir: &Path, uuid: Uuid) -> Option<PathBuf> {
    let sufixo = format!("_{uuid}.txt");
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(&sufixo))
        })
}

#[cfg(test)]
mod tests {
    use super::{NAO_ENCONTRADO, localizar};
    use std::fs;
    use uuid::Uuid;

    /// O caminho feliz: nome com prefixo de data + sufixo de UUID é encontrado pelo UUID sozinho.
    #[test]
    fn localiza_pelo_sufixo_sem_conhecer_o_prefixo() {
        let dir = tempdir();
        let id = Uuid::now_v7();
        let alvo = dir.join(format!("2026-08-09_10-15-05_{id}.txt"));
        fs::write(&alvo, "conteúdo").unwrap();
        fs::write(dir.join("2026-08-09_10-15-04_outro.txt"), "vizinho").unwrap();

        assert_eq!(localizar(&dir, id), Some(alvo));
    }

    /// Regressão da doutrina do sufixo: casar por `contains` deixaria um nome forjado — um arquivo
    /// cujo nome CONTÉM o UUID mas não termina nele — ser servido no lugar do log verdadeiro.
    #[test]
    fn nao_casa_uuid_no_meio_do_nome() {
        let dir = tempdir();
        let id = Uuid::now_v7();
        fs::write(dir.join(format!("2026-08-09_10-15-05_{id}_extra.txt")), "x").unwrap();

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

    fn tempdir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!("auli-log-{}", Uuid::now_v7()));
        fs::create_dir_all(&base).unwrap();
        base
    }
}
