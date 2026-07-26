//! Subcomando `indice` — deriva o **índice leve de pareceres** que o frontend consome.
//!
//! Fonte é a árvore `data/<id>/docs/pareceres/*.md` (fonte única desde a G5b); destino é
//! `data/<id>/raw/<id>-pareceres-index.json`, de onde o `build-frontend-public.sh` já copia todo
//! `raw/*.json` para `auli-frontend/public/<id>/`.
//!
//! **Leve = sem o `corpo`.** A tab de Pareceres passa a mostrar a sinopse e um link para o portal,
//! e busca sobre `[numero, assunto, resumo]`. É a mesma escolha da G3 no servidor (pack leve +
//! leitura tardia do corpo), agora do lado do navegador: SP cai de 147 MB para ~20 MB.
//!
//! **Ordem: do mais recente para o mais antigo**, por [`chave_cronologica`]. A lista era ordenada
//! por nome de arquivo (`slug(numero)`), o que punha o número antes do ano — os nº 1 de todos os
//! anos vinham juntos. Ordenar aqui, e não no navegador, é o que torna isso grátis: o artefato chega
//! pronto, e a aba do SP renderiza progressivamente 15,6 mil entradas.
//!
//! Derivação pura: rode de novo depois de qualquer passo que mexa na árvore (`pareceres`, `sinopse`,
//! ou os scrapers) e o resultado é função só do que está no disco.

use std::cmp::Reverse;
use std::path::Path;

use auli_contract::mddoc;
use serde::Serialize;

use crate::domain::entities::EntityConfig;
use crate::errors::Result;
use crate::sinopse::docs_dir;

/// Uma entrada do índice. Espelha o `ConsultaPackPayload` do contrato menos o `doc_path` (que só
/// interessa ao servidor) — o navegador não tem como ler a árvore.
#[derive(Serialize)]
struct Entrada {
    numero: String,
    assunto: String,
    /// A sinopse do documento. Vazia se o `.md` ainda está pendente.
    resumo: String,
    link: String,
}

pub fn run(entity: &EntityConfig) -> Result<()> {
    let dir = docs_dir(entity)?;
    if !dir.exists() {
        println!(
            "ℹ️  {} não tem árvore de pareceres ({}); nada a derivar.",
            entity.id,
            dir.display()
        );
        return Ok(());
    }

    let mut entradas = ler_arvore(&dir)?;
    let pendentes = entradas.iter().filter(|e| e.resumo.is_empty()).count();

    // Do mais RECENTE para o mais antigo — a aba do frontend renderiza progressivamente, então esta
    // ordem decide o que o usuário vê primeiro, e ordenar 15,6 mil entradas no navegador custaria
    // caro. `sort_by_cached_key` calcula a chave 1× por item (não a cada comparação) e é estável, o
    // que preserva a ordem de nome de arquivo entre os que empatam ou não têm chave.
    entradas.sort_by_cached_key(|e| match chave_cronologica(&e.numero) {
        Some((ano, seq)) => (0u8, Reverse(ano), Reverse(seq)),
        None => (1u8, Reverse(0), Reverse(0)),
    });

    let sem_chave: Vec<&str> = entradas
        .iter()
        .filter(|e| chave_cronologica(&e.numero).is_none())
        .map(|e| e.numero.as_str())
        .collect();

    let destino = format!("{}/{}-pareceres-index.json", entity.data_dir, entity.id);
    std::fs::write(&destino, serde_json::to_string_pretty(&entradas)?)?;

    println!(
        "✅ {} ({} pareceres, {pendentes} sem sinopse)",
        destino,
        entradas.len()
    );
    if pendentes > 0 {
        println!(
            "⚠️  rode `auli-collections {} sinopse` e derive de novo.",
            entity.id
        );
    }
    if !sem_chave.is_empty() {
        let amostra: Vec<&str> = sem_chave.iter().take(5).copied().collect();
        let reticencias = if sem_chave.len() > 5 { ", …" } else { "" };
        println!(
            "⚠️  {} de {} sem ano no `numero` — foram para o FIM da lista, sem data inventada: {}{}",
            sem_chave.len(),
            entradas.len(),
            amostra.join("; "),
            reticencias
        );
    }
    Ok(())
}

/// Chave cronológica extraída do `numero`: `(ano, sequência)`. `None` quando o formato não é
/// reconhecido — aí o item vai para o FIM da lista, nunca ganha uma data chutada.
///
/// **Por que sair do `numero`:** não existe data de publicação em lugar nenhum do pipeline. O
/// frontmatter do `.md` tem só `sinopse_gerada_em`, que é quando NÓS geramos a sinopse, não quando o
/// parecer saiu. O ano vive dentro do próprio `numero`, em dois formatos — ambos conferidos contra o
/// acervo real (SP 15.605 / SC 1.743 / PR 2.060 / RS 372: 100% parseáveis):
///
/// - **`<seq>/<ano>`** — SP (`RESPOSTA À CONSULTA TRIBUTÁRIA 1000/2012`), PR (`CONSULTA Nº 1/2007`),
///   SC (`CONSULTA COPAT nº 0001/11`). Ano de 4 ou 2 dígitos.
/// - **`<AA><NNN>` colado** — RS (`PARECER Nº 99028` = 1999 nº 028; `PARECER Nº 00070` = 2000 nº
///   070). Exige os 5 dígitos zero-padded: com menos, não há como distinguir ano+sequência de uma
///   sequência solta, e chutar seria pior que mandar para o fim.
///
/// **Pivô de século** no ano de 2 dígitos: `> 30` ⇒ 1900+, senão 2000+. Sem ele o parecer do RS de
/// 1999 (`99…`) apareceria como o mais recente do acervo inteiro. O corte em 30 cobre a faixa real
/// (RS 96–26, SC 11–26) com folga até 2030.
///
/// A ordenação por esta chave é a razão de a lista NÃO ser mais lexicográfica: `slug(numero)` põe o
/// número antes do ano, então os nº 1 de todos os anos vinham juntos.
fn chave_cronologica(numero: &str) -> Option<(i32, u32)> {
    // `<seq>/<ano>`: a barra é o separador em SP/PR/SC. `rsplit_once` para o caso de haver outra
    // barra antes (nenhum acervo tem hoje, mas o ano é sempre o último campo).
    if let Some((antes, depois)) = numero.rsplit_once('/') {
        let (ano_bruto, digitos) = digitos_iniciais(depois)?;
        return Some((ano_pleno(ano_bruto, digitos)?, digitos_finais(antes)?.0));
    }
    // RS: `AANNN` colado, exatamente 5 dígitos.
    let (v, digitos) = digitos_finais(numero)?;
    if digitos != 5 {
        return None;
    }
    Some((ano_pleno(v / 1000, 2)?, v % 1000))
}

/// Ano de 2 ou 4 dígitos → ano pleno. Qualquer outra largura é `None` (formato desconhecido).
fn ano_pleno(v: u32, digitos: usize) -> Option<i32> {
    match digitos {
        4 => Some(v as i32),
        2 => Some(if v > 30 {
            1900 + v as i32
        } else {
            2000 + v as i32
        }),
        _ => None,
    }
}

/// Corrida de dígitos no INÍCIO de `s` (após aparar espaços) + quantos dígitos tem — o comprimento
/// é o que decide 2 vs 4 dígitos de ano, então não pode ser perdido no `parse`.
fn digitos_iniciais(s: &str) -> Option<(u32, usize)> {
    let d: String = s
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    (!d.is_empty()).then(|| Some((d.parse().ok()?, d.len())))?
}

/// Corrida de dígitos no FIM de `s` (após aparar espaços) + quantos dígitos tem.
fn digitos_finais(s: &str) -> Option<(u32, usize)> {
    let rev: String = s
        .trim_end()
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .collect();
    let d: String = rev.chars().rev().collect();
    (!d.is_empty()).then(|| Some((d.parse().ok()?, d.len())))?
}

/// Lê a árvore em ordem estável (nome do arquivo — a mesma ordem do `auli update`). Arquivo que não
/// parseia é **erro**: um documento sumir do índice em silêncio é pior do que a derivação falhar.
fn ler_arvore(dir: &Path) -> Result<Vec<Entrada>> {
    let mut caminhos: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    caminhos.sort();

    let mut out = Vec::with_capacity(caminhos.len());
    for caminho in caminhos {
        let texto = std::fs::read_to_string(&caminho)?;
        let (header, sinopse, _corpo) = mddoc::parse_doc(&texto).map_err(|e| {
            format!(
                "`{}` não parseia ({e}) — corrija antes de derivar o índice",
                caminho.display()
            )
        })?;
        out.push(Entrada {
            numero: header.numero,
            assunto: header.assunto,
            resumo: sinopse.unwrap_or_default(),
            link: header.link,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os formatos REAIS dos quatro acervos, um por linha, com a chave que devem produzir.
    #[test]
    fn chave_cobre_os_formatos_reais_dos_quatro_acervos() {
        let casos = [
            // SP e PR: ano de 4 dígitos depois da barra.
            ("RESPOSTA À CONSULTA TRIBUTÁRIA 1000/2012", (2012, 1000)),
            ("RESPOSTA À CONSULTA TRIBUTÁRIA 997/2012", (2012, 997)),
            ("CONSULTA Nº 1/2007", (2007, 1)),
            ("CONSULTA Nº 99/2022", (2022, 99)),
            // SC: ano de 2 dígitos, sequência zero-padded.
            ("CONSULTA COPAT nº 0001/11", (2011, 1)),
            ("CONSULTA COPAT nº 0175/14", (2014, 175)),
            // RS: AANNN colado, com as 7 grafias de prefixo do acervo.
            ("PARECER Nº 99028", (1999, 28)),
            ("PARECER Nº 00070", (2000, 70)),
            ("Informação n.º 15001", (2015, 1)),
            ("INFORMAÇÃO Nº 26003", (2026, 3)),
            ("PARECER N° 21015", (2021, 15)),
        ];
        for (numero, esperada) in casos {
            assert_eq!(chave_cronologica(numero), Some(esperada), "{numero}");
        }
    }

    /// O pivô de século é o que impede o RS de 1999 de virar o mais recente do acervo.
    #[test]
    fn pivo_de_seculo_separa_1999_de_2026() {
        let noventa_nove = chave_cronologica("PARECER Nº 99028").unwrap();
        let vinte_seis = chave_cronologica("PARECER Nº 26003").unwrap();
        assert_eq!(noventa_nove.0, 1999);
        assert_eq!(vinte_seis.0, 2026);
        assert!(
            vinte_seis > noventa_nove,
            "2026 tem de ser mais recente que 1999"
        );
        // A fronteira do pivô: 30 ainda é 2030; 31 cai para 1931.
        assert_eq!(ano_pleno(30, 2), Some(2030));
        assert_eq!(ano_pleno(31, 2), Some(1931));
    }

    /// Formato desconhecido devolve `None` — o item vai para o fim, sem data chutada.
    #[test]
    fn formato_desconhecido_nao_ganha_data() {
        for numero in [
            "PARECER SEM NUMERO",
            "PARECER Nº 70",     // 2 dígitos: indistinguível de sequência solta
            "PARECER Nº 1234",   // 4 dígitos colados: idem
            "PARECER Nº 123456", // 6 dígitos: fora do AANNN
            "CONSULTA Nº 1/7",   // ano de 1 dígito
            "CONSULTA Nº /2012", // sem sequência
            "",
        ] {
            assert_eq!(chave_cronologica(numero), None, "{numero:?}");
        }
    }

    /// A ordenação que o `run` aplica: mais recente primeiro, sem-chave no fim, estável entre eles.
    #[test]
    fn ordena_do_mais_recente_ao_mais_antigo_com_sem_chave_no_fim() {
        let numeros = [
            "CONSULTA Nº 1/2007",
            "SEM CHAVE A",
            "CONSULTA Nº 5/2024",
            "CONSULTA Nº 1/2024",
            "SEM CHAVE B",
            "PARECER Nº 99028",
        ];
        let mut v: Vec<&str> = numeros.to_vec();
        v.sort_by_cached_key(|n| match chave_cronologica(n) {
            Some((ano, seq)) => (0u8, Reverse(ano), Reverse(seq)),
            None => (1u8, Reverse(0), Reverse(0)),
        });
        assert_eq!(
            v,
            vec![
                "CONSULTA Nº 5/2024", // 2024 nº 5 antes de 2024 nº 1 — desempate pela sequência
                "CONSULTA Nº 1/2024",
                "CONSULTA Nº 1/2007",
                "PARECER Nº 99028", // 1999, o mais antigo COM chave
                "SEM CHAVE A",      // ordem de chegada preservada entre os sem chave
                "SEM CHAVE B",
            ]
        );
    }
}
