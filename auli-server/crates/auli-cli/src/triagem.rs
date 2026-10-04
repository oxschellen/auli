//! Triagem dos documentos de serviços+FAQs antes da resposta (D-SF-9).
//!
//! A resposta do chat de atendimento passa a ser feita em **duas chamadas** ao LLM. A primeira — esta
//! — recebe a pergunta e os documentos recuperados e devolve só um veredito por documento; a segunda
//! redige a resposta vendo apenas os que sobreviveram. É a forma explícita da regra central da D-SF-1:
//! os serviços são alternativas para situações excludentes, e o que não trata da situação perguntada
//! nem deveria estar na mesa quando o texto é escrito.
//!
//! **Fail-open de propósito.** Triagem que falha (rede, JSON inválido, rótulo que não existe no
//! contexto) devolve `Err`, e o chamador segue com o contexto INTEGRAL — exatamente a resposta de uma
//! chamada só. A triagem é um filtro de qualidade, não uma guarda: perdê-la piora a resposta, mas
//! não pode derrubá-la. Pela mesma razão, documento que o modelo **omitiu** do JSON é mantido: só
//! sai o que foi explicitamente descartado.
//!
//! Este módulo é PURO (sem rede, sem I/O): interpreta o texto que o modelo devolveu. A chamada vive
//! no `rag.rs`, junto da outra.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

/// O que o modelo diz de UM documento.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Veredito {
    /// Trata da situação perguntada.
    Aplica,
    /// Trata da situação, mas depende de uma condição que a pergunta não informa (o regime, o
    /// público). **Fica** no contexto: é justamente o irmão que a resposta deve apresentar como
    /// alternativa, com a condição que decide.
    Condicional,
    /// Não trata da situação perguntada. Sai do contexto da segunda chamada.
    Descarta,
}

impl Veredito {
    /// A grafia de fio — a mesma do JSON que o modelo devolve e da lista de fontes (D-SF-10).
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Veredito::Aplica => "aplica",
            Veredito::Condicional => "condicional",
            Veredito::Descarta => "descarta",
        }
    }
}

#[derive(Debug, Deserialize)]
struct Item {
    documento: String,
    veredito: Veredito,
    // O `motivo` é pedido pelo prompt e lido só no log de auditoria (texto cru). Aceito e ignorado
    // aqui, para que a ausência dele não derrube a triagem.
    #[serde(default)]
    #[allow(dead_code)]
    motivo: String,
}

#[derive(Debug, Deserialize)]
struct Resposta {
    documentos: Vec<Item>,
}

/// Interpreta a resposta da triagem: o veredito de cada documento, pelo rótulo canônico
/// (`"Serviço 3"`, `"FAQ 7"`). Documento que o modelo omitiu simplesmente não está no mapa.
///
/// `rotulos` são os que existem no contexto enviado. Um rótulo desconhecido na resposta é `Err`:
/// o modelo está falando de um documento que não viu, e nada do que ele disse sobre os outros
/// merece confiança.
pub(crate) fn vereditos(
    resposta: &str,
    rotulos: &[String],
) -> Result<BTreeMap<String, Veredito>, String> {
    let json = extrair_objeto(resposta).ok_or("a resposta não contém um objeto JSON")?;
    let r: Resposta = serde_json::from_str(json).map_err(|e| format!("JSON inválido: {e}"))?;
    let mut mapa = BTreeMap::new();
    for item in r.documentos {
        let rotulo = normalizar(&item.documento);
        if !rotulos.contains(&rotulo) {
            return Err(format!("rótulo fora do contexto: '{}'", item.documento));
        }
        mapa.insert(rotulo, item.veredito);
    }
    Ok(mapa)
}

/// Os rótulos **descartados** — os únicos que saem do contexto da segunda chamada. Omitido e
/// `condicional` ficam (fail-open por documento).
pub(crate) fn descartados(vereditos: &BTreeMap<String, Veredito>) -> BTreeSet<String> {
    vereditos
        .iter()
        .filter(|(_, v)| **v == Veredito::Descarta)
        .map(|(r, _)| r.clone())
        .collect()
}

/// O trecho entre a primeira `{` e a última `}`. Cobre a cerca markdown (```` ```json ````) e o
/// texto antes ou depois que o modelo às vezes acrescenta apesar do "JSON puro".
fn extrair_objeto(s: &str) -> Option<&str> {
    let ini = s.find('{')?;
    let fim = s.rfind('}')?;
    (fim > ini).then(|| &s[ini..=fim])
}

/// `"[Serviço 3]"`, `" serviço  3 "` → `"Serviço 3"`. Tolera colchetes, caixa e espaços; o tipo
/// volta à grafia canônica do rótulo para casar com a lista do contexto.
fn normalizar(s: &str) -> String {
    let limpo: String = s
        .trim()
        .trim_matches(|c| c == '[' || c == ']')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut partes = limpo.splitn(2, ' ');
    let tipo = partes.next().unwrap_or_default();
    let resto = partes.next().unwrap_or_default();
    let tipo = match tipo.to_lowercase().as_str() {
        "serviço" | "servico" => "Serviço",
        "faq" => "FAQ",
        _ => tipo,
    };
    format!("{tipo} {resto}").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O caminho do `rag.rs`: vereditos e, deles, os descartados.
    fn fora(r: &str) -> Result<BTreeSet<String>, String> {
        vereditos(r, &rotulos()).map(|m| descartados(&m))
    }

    fn rotulos() -> Vec<String> {
        ["Serviço 1", "Serviço 2", "FAQ 1", "FAQ 2"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn so_o_descartado_sai_e_condicional_fica() {
        let r = r#"{"documentos": [
            {"documento": "Serviço 1", "veredito": "aplica", "motivo": "x"},
            {"documento": "Serviço 2", "veredito": "condicional", "motivo": "regime"},
            {"documento": "FAQ 1", "veredito": "descarta", "motivo": "outro tema"}
        ]}"#;
        let fora = fora(r).unwrap();
        // A FAQ 2 foi omitida pelo modelo: fica (fail-open por documento).
        assert_eq!(fora, BTreeSet::from(["FAQ 1".to_string()]));
    }

    #[test]
    fn cerca_markdown_e_texto_em_volta_sao_tolerados() {
        let r = "Segue:\n```json\n{\"documentos\": [{\"documento\": \"FAQ 2\", \"veredito\": \"descarta\"}]}\n```";
        let fora = fora(r).unwrap();
        assert_eq!(fora, BTreeSet::from(["FAQ 2".to_string()]));
    }

    #[test]
    fn rotulo_com_colchete_e_caixa_diferente_e_normalizado() {
        let r = r#"{"documentos": [{"documento": "[servico 2]", "veredito": "descarta"}]}"#;
        let fora = fora(r).unwrap();
        assert_eq!(fora, BTreeSet::from(["Serviço 2".to_string()]));
    }

    #[test]
    fn rotulo_fora_do_contexto_invalida_a_triagem_inteira() {
        let r = r#"{"documentos": [
            {"documento": "FAQ 1", "veredito": "descarta"},
            {"documento": "Serviço 9", "veredito": "aplica"}
        ]}"#;
        assert!(fora(r).is_err());
    }

    #[test]
    fn veredito_desconhecido_e_resposta_sem_json_sao_erro() {
        let r = r#"{"documentos": [{"documento": "FAQ 1", "veredito": "talvez"}]}"#;
        assert!(fora(r).is_err());
        assert!(fora("não sei").is_err());
    }

    #[test]
    fn vereditos_trazem_os_tres_valores_e_omitido_fica_de_fora_do_mapa() {
        let r = r#"{"documentos": [
            {"documento": "Serviço 1", "veredito": "aplica"},
            {"documento": "Serviço 2", "veredito": "condicional"},
            {"documento": "FAQ 1", "veredito": "descarta"}
        ]}"#;
        let m = vereditos(r, &rotulos()).unwrap();
        assert_eq!(m.get("Serviço 1").map(|v| v.as_str()), Some("aplica"));
        assert_eq!(m.get("Serviço 2").map(|v| v.as_str()), Some("condicional"));
        assert_eq!(m.get("FAQ 1").map(|v| v.as_str()), Some("descarta"));
        assert!(!m.contains_key("FAQ 2"));
    }
}
