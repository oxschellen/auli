//! Subcomando `canonizar-servicos` — chave canônica dos sistemas citados na carta de serviços.
//!
//! Espelho do `canonizar` (dispositivos) sobre a saída do `extrair-servicos`: **determinístico, sem
//! rede, sem LLM**. Lê `servicos-extracao.jsonl` e colapsa cada `sistema.texto` literal numa chave.
//!
//! A diferença de fundo em relação aos dispositivos: lá o vocabulário é FECHADO (a norma tem
//! gramática — livro, artigo, inciso) e a chave é derivável por regex. Aqui é ABERTO: "Portal
//! e-CAC", "e-CAC", "eCAC" e "Portal de Serviços e-CAC" são o mesmo portal, e nenhuma gramática
//! deduz isso. Então a canonização tem duas camadas:
//!
//! 1. **Slug** (mecânica, cobre tudo): minúsculas, sem acento, pontuação → hífen. Já colapsa
//!    variação de caixa/acento/pontuação sozinha.
//! 2. **Aliases** (curada, cobre a cabeça da distribuição): tabela slug → chave, calibrada sobre a
//!    frequência real do acervo. A cauda longa fica no próprio slug — nada é chutado.
//!
//! O literal NUNCA é destruído: cada ocorrência sai com o `texto` original ao lado da chave, e o que
//! o filtro de ruído recusa vira `canonizavel: false` (o grafo omite o nó). O `display` é escolhido
//! por frequência da variante, com desempate lexicográfico — função do acervo inteiro, não da ordem
//! de leitura, logo idempotente.
//!
//! Saídas irmãs em `data/<id>/extracao/`: `sistemas.jsonl` (1 linha por ocorrência) e
//! `sistemas-index.json` (semente do grafo: `chave → {display, ocorrencias, variantes, servicos}`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::domain::entities::EntityConfig;
use crate::errors::Result;
use crate::extracao::extracao_dir;
use crate::extracao_servicos::Sistema;

/// Uma linha lida do `servicos-extracao.jsonl` — só o que o canonizador usa (temas ficam para o grafo).
#[derive(Deserialize)]
struct EntradaLinha {
    titulo: String,
    extracao: EntradaExtracao,
}

#[derive(Deserialize)]
struct EntradaExtracao {
    sistemas: Vec<Sistema>,
}

/// Uma linha do `sistemas.jsonl` (1 por ocorrência). `texto` = literal preservado (auditoria).
#[derive(Serialize)]
struct SaidaLinha {
    servico: String,
    texto: String,
    canon_key: Option<String>,
    canonizavel: bool,
}

/// Um nó do `sistemas-index.json` (semente do grafo).
#[derive(Serialize)]
struct IndiceEntrada {
    display: String,
    ocorrencias: usize,
    variantes: Vec<String>,
    servicos: Vec<String>,
}

/// Acumulador do índice. `variantes` é multiconjunto (contagem) porque o `display` sai da variante
/// MAIS FREQUENTE — decidir por frequência exige contar, não só coletar.
struct Acc {
    ocorrencias: usize,
    variantes: BTreeMap<String, usize>,
    servicos: BTreeSet<String>,
}

/// Sufixo de sigla entre parênteses: `Domicílio Tributário Eletrônico (DTE)` → captura `DTE`.
static SIGLA_PARENTESES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(([A-Za-zÀ-ÿ][A-Za-z0-9À-ÿ/.\- ]{1,20})\)\s*$").unwrap());

/// Órgãos, carreiras e setores que o prompt manda excluir mas escapam de vez em quando. Não são
/// sistemas: entrariam como hubs falsos e colapsariam o layout (o `Receita Estadual` liga a tudo).
const ORGAO_STOP: &[&str] = &[
    "receita-estadual",
    "sefaz",
    "sefaz-rs",
    "secretaria-da-fazenda",
    "secretaria-estadual-da-fazenda",
    "cage",
    "tesouro-do-estado",
    "tesouro",
    "estado-do-rio-grande-do-sul",
    "governo-do-estado",
    "detran",
    "detran-rs",
    "junta-comercial",
    "jucisrs",
    "banrisul",
    "auditor-fiscal",
    "procergs",
];

/// Canais genéricos sem nome próprio — idem: ruído de alta frequência, zero informação de grafo.
const GENERICO_STOP: &[&str] = &[
    "site",
    "internet",
    "e-mail",
    "email",
    "telefone",
    "portal",
    "sistema",
    "aplicativo",
    "app",
    "atendimento-presencial",
    "atendimento-virtual",
    "correio",
    "correios",
];

/// Aliases curados (slug → chave canônica), calibrados sobre a frequência REAL do acervo do RS: só
/// entra par que o corpus justifica, mais a grafia sem hífen das siglas (variação barata e comum).
/// A cauda longa fica no próprio slug — a tabela cobre a cabeça, não pretende cobrir o vocabulário.
/// Quando a chave de destino é o próprio slug (`protocolo-eletronico`), não há linha: é identidade.
const ALIAS: &[(&str, &str)] = &[
    // e-CAC — o portal do contribuinte, campeão de variantes (46 "Portal e-CAC" + 19 "e-CAC").
    ("ecac", "e-cac"),
    ("portal-e-cac", "e-cac"),
    ("portal-ecac", "e-cac"),
    // Portal Pessoa Física.
    (
        "portal-pessoa-fisica-da-receita-estadual",
        "portal-pessoa-fisica",
    ),
    // Domicílio Tributário Eletrônico (a forma `... (DTE)` cai na regra da sigla).
    ("dt-e", "dte"),
    ("domicilio-tributario-eletronico", "dte"),
    // Documentos fiscais eletrônicos.
    ("nfe", "nf-e"),
    ("nota-fiscal-eletronica", "nf-e"),
    ("portal-da-nf-e", "nf-e"),
    ("nfce", "nfc-e"),
    ("cte", "ct-e"),
    ("mdfe", "mdf-e"),
    ("nota-fiscal-gaucha", "nfg"),
    // Escrituração digital. GIA-ST NÃO colapsa em GIA: é outra declaração.
    ("efd-icms-ipi", "efd"),
    // Cadastro geral de contribuintes: duas grafias da mesma sigla.
    ("cgc-te", "cgcte"),
    // REDESIM e RHE: portais/ambientes do mesmo sistema.
    ("portal-redesim-rs", "redesim"),
    ("portal-redesim", "redesim"),
    ("rhe-producao", "rhe"),
    ("rhe-historico", "rhe"),
    ("rhe-servicos", "rhe"),
    ("sistema-de-agendamento-do-estado", "sistema-de-agendamento"),
];

fn alias_map() -> BTreeMap<&'static str, &'static str> {
    ALIAS.iter().copied().collect()
}

/// Slug canônico: minúsculas, sem acento, tudo que não é alfanumérico vira hífen, hífens colapsados.
/// É a camada mecânica — sozinha já colapsa caixa, acento e pontuação.
fn slug(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for c in t.trim().chars() {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' | 'É' | 'Ê' | 'È' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            outro => outro,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if !s.ends_with('-') {
            s.push('-');
        }
    }
    s.trim_matches('-').to_string()
}

/// Canoniza UM literal (pura, testável). `None` = ruído: vazio, longo demais (o modelo despejou uma
/// frase em vez do nome), órgão ou canal genérico.
fn canonizar_sistema(texto: &str) -> Option<String> {
    let bruto = texto.split_whitespace().collect::<Vec<_>>().join(" ");
    // Nome próprio de sistema é curto; acima disso é frase despejada, não nome.
    if bruto.chars().count() > 60 {
        return None;
    }
    let aliases = alias_map();

    // A sigla entre parênteses é a forma que o acervo usa no resto do texto: `... (DTE)` → `dte`.
    // Só vale se a sigla JÁ FOR uma identidade conhecida (destino de algum alias) — senão o nome por
    // extenso é a identidade melhor, e nada é chutado.
    if let Some(c) = SIGLA_PARENTESES.captures(&bruto) {
        let sigla = slug(&c[1]);
        if ALIAS.iter().any(|(_, destino)| *destino == sigla) {
            return Some(sigla);
        }
    }

    let s = slug(&bruto);
    if s.is_empty() || ORGAO_STOP.contains(&s.as_str()) || GENERICO_STOP.contains(&s.as_str()) {
        return None;
    }
    Some(
        aliases
            .get(s.as_str())
            .map(|k| (*k).to_string())
            .unwrap_or(s),
    )
}

/// Escrita atômica (`.tmp` + rename) — saída derivada, reescrita inteira a cada rodada.
fn escrever_atomico(path: &Path, conteudo: &str) -> Result<()> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, conteudo)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn run(entity: &EntityConfig) -> Result<()> {
    let dir = extracao_dir(entity)?;
    let in_path = dir.join("servicos-extracao.jsonl");
    if !in_path.exists() {
        return Err(format!(
            "saída da extração ausente: {} — rode `auli-collections {} extrair-servicos` antes.",
            in_path.display(),
            entity.id
        )
        .into());
    }

    let texto = std::fs::read_to_string(&in_path)?;
    let mut saidas: Vec<SaidaLinha> = Vec::new();
    let mut index: BTreeMap<String, Acc> = BTreeMap::new();

    for (i, l) in texto.lines().enumerate() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        let entrada: EntradaLinha = serde_json::from_str(l)
            .map_err(|e| format!("{}: linha {} malformada ({e})", in_path.display(), i + 1))?;
        for sis in entrada.extracao.sistemas {
            let canon = canonizar_sistema(&sis.texto);
            if let Some(k) = &canon {
                let acc = index.entry(k.clone()).or_insert_with(|| Acc {
                    ocorrencias: 0,
                    variantes: BTreeMap::new(),
                    servicos: BTreeSet::new(),
                });
                acc.ocorrencias += 1;
                *acc.variantes.entry(sis.texto.clone()).or_insert(0) += 1;
                acc.servicos.insert(entrada.titulo.clone());
            }
            saidas.push(SaidaLinha {
                servico: entrada.titulo.clone(),
                canonizavel: canon.is_some(),
                canon_key: canon,
                texto: sis.texto,
            });
        }
    }

    // Ordem estável: por serviço, depois por literal.
    saidas.sort_by(|a, b| {
        a.servico
            .cmp(&b.servico)
            .then_with(|| a.texto.cmp(&b.texto))
    });

    let mut buf = String::new();
    for s in &saidas {
        buf.push_str(&serde_json::to_string(s).map_err(|e| format!("serializando saída: {e}"))?);
        buf.push('\n');
    }
    escrever_atomico(&dir.join("sistemas.jsonl"), &buf)?;

    let index_out: BTreeMap<String, IndiceEntrada> = index
        .into_iter()
        .map(|(k, a)| {
            // Display = variante mais frequente (desempate lexicográfico) — função do acervo
            // inteiro, não da ordem de leitura, logo idempotente.
            let display = a
                .variantes
                .iter()
                .max_by(|x, y| x.1.cmp(y.1).then_with(|| y.0.cmp(x.0)))
                .map(|(v, _)| v.clone())
                .unwrap_or_else(|| k.clone());
            (
                k,
                IndiceEntrada {
                    display,
                    ocorrencias: a.ocorrencias,
                    variantes: a.variantes.into_keys().collect(),
                    servicos: a.servicos.into_iter().collect(),
                },
            )
        })
        .collect();
    let json = serde_json::to_string_pretty(&index_out)
        .map_err(|e| format!("serializando índice: {e}"))?;
    escrever_atomico(&dir.join("sistemas-index.json"), &format!("{json}\n"))?;

    let total = saidas.len();
    let canon = saidas.iter().filter(|s| s.canonizavel).count();
    println!(
        "🔗 {}: ocorrências {total} | canonizáveis {canon} → {} chaves | ruído {}",
        entity.id,
        index_out.len(),
        total - canon
    );
    println!(
        "📄 saídas: {} e {}",
        dir.join("sistemas.jsonl").display(),
        dir.join("sistemas-index.json").display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_colapsa_caixa_acento_e_pontuacao() {
        assert_eq!(slug("Portal e-CAC"), "portal-e-cac");
        assert_eq!(slug("  Nota Fiscal Gaúcha  "), "nota-fiscal-gaucha");
        assert_eq!(slug("EFD/SPED"), "efd-sped");
    }

    #[test]
    fn aliases_colapsam_as_variantes_do_mesmo_portal() {
        for v in [
            "e-CAC",
            "eCAC",
            "Portal e-CAC",
            "portal E-CAC",
            "PORTAL ECAC",
        ] {
            assert_eq!(
                canonizar_sistema(v).as_deref(),
                Some("e-cac"),
                "variante {v:?} devia colapsar em e-cac"
            );
        }
    }

    #[test]
    fn sigla_entre_parenteses_vence_quando_e_alias_conhecido() {
        assert_eq!(
            canonizar_sistema("Domicílio Tributário Eletrônico (DTE)").as_deref(),
            Some("dte")
        );
        // Sigla desconhecida: o nome por extenso é a identidade melhor (nada é chutado).
        assert_eq!(
            canonizar_sistema("Sistema de Coisas Novas (SCN)").as_deref(),
            Some("sistema-de-coisas-novas-scn")
        );
    }

    #[test]
    fn orgao_e_generico_sao_ruido() {
        for v in ["Receita Estadual", "SEFAZ", "CAGE", "site", "e-mail"] {
            assert!(
                canonizar_sistema(v).is_none(),
                "{v:?} não é sistema — vira ruído"
            );
        }
    }

    #[test]
    fn frase_despejada_e_ruido() {
        let frase = "o contribuinte deve acessar o portal e preencher o formulário disponível na página do serviço";
        assert!(canonizar_sistema(frase).is_none(), "frase não é nome");
    }

    #[test]
    fn cauda_longa_fica_no_proprio_slug() {
        assert_eq!(
            canonizar_sistema("Acordo Gaúcho").as_deref(),
            Some("acordo-gaucho"),
            "sem alias, o slug É a chave — nada é descartado por não estar na tabela"
        );
    }
}
