//! Base compartilhada pelos reconhecedores da **Fase 4** (razão social, endereço, nome).
//!
//! Os nove reconhecedores das Fases 0/1 duplicam o próprio `limites_ok` porque cada um tem uma
//! variação da regra (a de "separador colado a dígito", que protege CNPJ/protocolo de serem
//! fatiados). Os três da Fase 4 casam **texto**, não dígito: a regra de separador não se aplica e
//! a de fronteira precisa enxergar acento — daí um módulo em vez de uma quarta cópia.

/// Fronteira do span: o vizinho de cada lado não pode ser letra ou dígito.
///
/// Diferente do `limites_ok` dos reconhecedores numéricos, usa [`char::is_alphanumeric`] (Unicode)
/// e não `is_ascii_alphanumeric`: aqui o vizinho é texto pt-BR, e `is_ascii_alphanumeric('ã')` é
/// `false` — o que deixaria passar um match colado ao fim de uma palavra acentuada.
pub(super) fn limites_ok(text: &str, start: usize, end: usize) -> bool {
    let livre = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    livre(text[..start].chars().next_back()) && livre(text[end..].chars().next())
}

/// Termos institucionais capitalizados que aparecem onde um nome ou uma razão social apareceria.
///
/// **Precisão vence recall** (regra de ouro da fase): a lista rejeita o candidato inteiro se
/// qualquer token seu constar aqui. O custo é conhecido e aceito — uma `Transportadora Rio Grande
/// Ltda` real não seria mascarada, porque `Rio`/`Grande` estão na lista. O ganho é que
/// `Simples Nacional ME`, `Vossa Senhoria` e `Contribuinte Substituto Tributário` — todos medidos
/// como falso positivo antes desta trava — não viram PII.
pub(super) const STOPLIST: &[&str] = &[
    // Qualificação do sujeito passivo
    "Pessoa",
    "Física",
    "Fisica",
    "Jurídica",
    "Juridica",
    "Contribuinte",
    "Substituto",
    "Substituição",
    "Substituicao",
    "Vossa",
    "Senhoria",
    // Regimes e documentos
    "Nota",
    "Fiscal",
    "Fiscais",
    "Simples",
    "Nacional",
    "Regime",
    "Especial",
    "Inscrição",
    "Inscricao",
    "Estadual",
    "Estaduais",
    "Cadastro",
    "Tributário",
    "Tributária",
    "Tributaria",
    "ICMS",
    "IPVA",
    "ITCD",
    // Órgãos, cargos e lugares
    "Auditor",
    "Analista",
    "Receita",
    "Fazenda",
    "Sefaz",
    "SEFAZ",
    "Secretaria",
    "Federal",
    "Portal",
    "Atendimento",
    "Junta",
    "Estado",
    "Município",
    "Municipio",
    "Prefeitura",
    "Porto",
    "Alegre",
    "Rio",
    "Grande",
    "Sul",
];

/// `true` se algum token de `candidato` está na [`STOPLIST`]. A pontuação colada é descartada
/// antes da comparação (`"Física,"` conta como `"Física"`).
pub(super) fn tem_stop(candidato: &str) -> bool {
    candidato
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
        .any(|t| STOPLIST.contains(&t))
}

/// `true` se o token começa com maiúscula. Usado onde basta "parece nome próprio".
pub(super) fn comeca_maiuscula(token: &str) -> bool {
    token.chars().next().is_some_and(char::is_uppercase)
}

/// `true` se o token é *Titlecase*: começa com maiúscula **e** tem alguma minúscula.
///
/// É o que separa `Andrade` (nome próprio) de `ICMS` (sigla em caixa alta) — a distinção que
/// impede `PARCELAMENTO ICMS ME` de virar razão social.
pub(super) fn titlecase(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next().is_some_and(char::is_uppercase) && chars.any(char::is_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limites_enxergam_acento_colado() {
        // "informação" + "Ltda": o vizinho antes do span é 'o' (letra) → recusa.
        let t = "informaçãoLtda Silva";
        let start = t.find("Ltda").expect("posição");
        assert!(!limites_ok(t, start, start + 4));
        // Com espaço no meio, a fronteira está livre.
        let t = "informação Ltda Silva";
        let start = t.find("Ltda").expect("posição");
        assert!(limites_ok(t, start, start + 4));
    }

    #[test]
    fn stoplist_ignora_pontuacao_colada() {
        assert!(tem_stop("Pessoa Física,"));
        assert!(tem_stop("Vossa Senhoria"));
        assert!(!tem_stop("João da Silva Pereira"));
    }

    #[test]
    fn titlecase_separa_nome_de_sigla() {
        assert!(titlecase("Andrade"));
        assert!(!titlecase("ICMS"));
        assert!(!titlecase("de"));
        // `comeca_maiuscula` é mais frouxo de propósito: a sigla passa.
        assert!(comeca_maiuscula("ICMS"));
    }
}
