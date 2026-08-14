//! Subcomando `indice legislacao` — deriva o índice leve da coleção de **legislação**.
//!
//! Irmão do [`crate::indice`], separado por três diferenças que não cabiam num braço: a árvore tem
//! **subpastas** (uma por lei), o shape da entrada é outro (D-LEG-11) e a ordem é a do TEXTO LEGAL,
//! não a cronológica.
//!
//! **Shape novo, com nomes limpos** (D-LEG-11): `[{titulo, trilha, ementa, link}]`. O índice de
//! jurisprudência carrega `numero`/`assunto` por compatibilidade com uma tab que já existia; aqui
//! não há contrato legado a preservar, então os campos usam o vocabulário unificado. Não há
//! `resumo`: legislação não tem `## resumo`, e a resposta É o corpo (que não entra no índice).
//!
//! **Ordem: por lei, e dentro da lei pelo dispositivo** (D-LEG-12) — a ordem em que se lê uma lei.
//! Ordenar pela trilha quebraria nos algarismos romanos ("Título IX" > "Título X" em lexicográfico);
//! o número do artigo é monotônico no texto. Ver [`chave_dispositivo`] para o que parseia e o que
//! vai para o fim do grupo.

use std::path::Path;

use auli_contract::{Kind, lei_da_trilha, mddoc};
use serde::Serialize;

use crate::domain::entities::EntityConfig;
use crate::errors::Result;

/// Uma entrada do índice: o cabeçalho do `.md` menos o que só interessa ao servidor. É o
/// vocabulário unificado caindo 1:1 — `titulo` é a pergunta, `ementa` é o dispositivo.
#[derive(Serialize)]
struct Entrada {
    titulo: String,
    trilha: String,
    ementa: String,
    link: String,
}

pub fn run(entity: &EntityConfig) -> Result<()> {
    let dir = crate::indice::docs_dir(entity, Kind::Legislacao)?;
    if !dir.exists() {
        println!(
            "ℹ️  {} não tem árvore de {} ({}); nada a derivar.",
            entity.id,
            Kind::Legislacao.plural(),
            dir.display()
        );
        return Ok(());
    }

    let mut entradas = ler_arvore(&dir)?;

    // Por lei (alfabética) e, dentro da lei, pelo dispositivo. `sort_by_cached_key` calcula a chave
    // 1× por item e é ESTÁVEL: quem empata ou não tem chave preserva a ordem de leitura (que é a de
    // caminho), em vez de embaralhar a cada rodada.
    entradas.sort_by_cached_key(|e| {
        let lei = lei_da_trilha(&e.trilha).to_string();
        match chave_dispositivo(&e.ementa) {
            Some((num, sufixo)) => (lei, 0u8, num, sufixo),
            // `1` no segundo slot é o que empurra o sem-chave para o FIM DO GRUPO — do grupo da
            // própria lei, não da lista, porque a lei é o primeiro componente da chave.
            None => (lei, 1u8, 0, 0),
        }
    });

    let destino = format!(
        "{}/{}-{}-index.json",
        entity.data_dir,
        entity.id,
        Kind::Legislacao.as_str()
    );
    std::fs::write(&destino, serde_json::to_string_pretty(&entradas)?)?;

    let leis: Vec<&str> = {
        let mut v: Vec<&str> = entradas
            .iter()
            .map(|e| lei_da_trilha(&e.trilha))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        v.sort_unstable();
        v
    };
    println!(
        "✅ {} ({} {}, {} leis)",
        destino,
        entradas.len(),
        Kind::Legislacao.plural(),
        leis.len()
    );
    for lei in &leis {
        println!("   · {lei}");
    }

    let sem_chave: Vec<&str> = entradas
        .iter()
        .filter(|e| chave_dispositivo(&e.ementa).is_none())
        .map(|e| e.ementa.as_str())
        .collect();
    if !sem_chave.is_empty() {
        let amostra: Vec<&str> = sem_chave.iter().take(5).copied().collect();
        let reticencias = if sem_chave.len() > 5 { ", …" } else { "" };
        println!(
            "⚠️  {} de {} sem dispositivo reconhecível na `ementa` — foram para o FIM do grupo da \
             lei, sem ordem inventada: {}{}",
            sem_chave.len(),
            entradas.len(),
            amostra.join("; "),
            reticencias
        );
    }
    Ok(())
}

/// Chave de ordenação do dispositivo, tirada da `ementa`: `(número do artigo, sufixo de letra)`.
/// `None` quando não há artigo reconhecível no começo — aí a entrada vai para o fim do grupo da
/// lei, **nunca ganha uma posição chutada**.
///
/// O que parseia, medido contra as 134 ementas reais da Lei 6.537/1973:
///
/// - **`Art. {N}`** — o caso comum, com ou sem o que vem depois: `Art. 28`, `Art. 1º`,
///   `Art. 21, I`, `Art. 121, § 2º`, `Art. 11, I, a`, `Art. 1º, parágrafo único`. O que segue o
///   número não entra na chave: inciso e parágrafo ordenam junto do artigo deles, que é a ordem do
///   texto.
/// - **`Art. {N}-{L}`** — artigo acrescido por lei posterior (`Art. 17-A`, `Art. 136-F`): 10 das
///   134. O sufixo é o segundo componente da chave, então `17` < `17-A` < `18`, como no texto.
/// - **`Arts. {N} …`** — o PLURAL, que a decisão original não previa: `Arts. 85 a 89`,
///   `Arts. 136-C a 136-E`, `Arts. 32 e 33`. São **17 das 134**, e mandá-las para o fim degradaria
///   13% do índice. Chaveiam pelo PRIMEIRO artigo da faixa, que está escrito ali — é leitura, não
///   invenção, e o conservadorismo do D-LEG-12 é contra ordem chutada, não contra ler o que a
///   ementa diz.
///
/// O que NÃO parseia (e é o certo): ementa vazia, ou que comece por qualquer outra coisa — remissão
/// a outra norma, texto livre. Sem número no começo não há ordem defensável.
fn chave_dispositivo(ementa: &str) -> Option<(u32, u8)> {
    let resto = ementa.trim().strip_prefix("Art")?;
    // `Art.` e `Arts.` — e nada além disso: `Artigo` não aparece no acervo, e aceitar prefixo
    // arbitrário abriria a porta para casar `Artefato`.
    let resto = resto.strip_prefix('s').unwrap_or(resto);
    let resto = resto.strip_prefix('.')?.trim_start();

    let digitos: String = resto.chars().take_while(char::is_ascii_digit).collect();
    if digitos.is_empty() {
        return None;
    }
    let numero: u32 = digitos.parse().ok()?;

    // Sufixo de letra: só o `-A` colado ao número conta. `Art. 136 - A` (com espaços) não existe no
    // acervo e não é reconhecido — chave sem sufixo é a leitura conservadora.
    let sufixo = resto[digitos.len()..]
        .strip_prefix('-')
        .and_then(|s| s.chars().next())
        .filter(char::is_ascii_uppercase)
        .map_or(0, |c| c as u8);
    Some((numero, sufixo))
}

/// Lê a árvore `docs/legislacao/<lei>/*.md` — **um nível de subpastas** (D-LEG-3), em ordem de
/// caminho. Mesma leitura do `update`, e pelo mesmo motivo de ordem: determinismo entre rodadas.
///
/// `.md` solto na raiz da coleção é lido também: ele tem trilha, então tem lei, e sai no grupo
/// certo. Quem recusa a árvore malformada é o `preparar` do `auli update`, com a guarda do
/// `doc_path` — não cabe a um derivador puro ter uma segunda opinião sobre isso.
fn ler_arvore(dir: &Path) -> Result<Vec<Entrada>> {
    let mut caminhos: Vec<std::path::PathBuf> = Vec::new();
    for entrada in std::fs::read_dir(dir)? {
        let caminho = entrada?.path();
        if caminho.is_dir() {
            for neto in std::fs::read_dir(&caminho)? {
                let neto = neto?.path();
                if neto.is_file() && neto.extension().is_some_and(|e| e == "md") {
                    caminhos.push(neto);
                }
            }
        } else if caminho.extension().is_some_and(|e| e == "md") {
            caminhos.push(caminho);
        }
    }
    caminhos.sort();

    let mut out = Vec::with_capacity(caminhos.len());
    for caminho in caminhos {
        let texto = std::fs::read_to_string(&caminho)?;
        let (header, _resumo, _corpo) = mddoc::parse_doc(&texto).map_err(|e| {
            format!(
                "`{}` não parseia ({e}) — corrija antes de derivar o índice",
                caminho.display()
            )
        })?;
        out.push(Entrada {
            titulo: header.titulo,
            trilha: header.trilha,
            ementa: header.ementa,
            link: header.link,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os formatos REAIS da ementa, tirados das 134 da Lei 6.537/1973 — um por linha, com a chave
    /// que devem produzir. É o teste que a Fase 4 pedia: ordinal, inciso, parágrafo e faixa.
    #[test]
    fn chave_cobre_os_formatos_reais_da_ementa() {
        let casos = [
            // O comum, e o ordinal `º` — que não pode virar parte do número nem cortar o parse.
            ("Art. 28", (28, 0)),
            ("Art. 1º", (1, 0)),
            ("Art. 9º", (9, 0)),
            ("Art. 104", (104, 0)),
            // Inciso, alínea e parágrafo: ordenam junto do artigo deles, não por si.
            ("Art. 21, I", (21, 0)),
            ("Art. 11, I, a", (11, 0)),
            ("Art. 121, § 2º", (121, 0)),
            ("Art. 96, §§ 5º a 9º", (96, 0)),
            ("Art. 1º, parágrafo único", (1, 0)),
            // Artigo acrescido: o sufixo é o segundo componente.
            ("Art. 17-A", (17, b'A')),
            ("Art. 136-F", (136, b'F')),
            // Plural e faixa: chaveiam pelo PRIMEIRO artigo.
            ("Arts. 85 a 89", (85, 0)),
            ("Arts. 32 e 33", (32, 0)),
            ("Arts. 136-C a 136-E", (136, b'C')),
        ];
        for (ementa, esperada) in casos {
            assert_eq!(
                chave_dispositivo(ementa),
                Some(esperada),
                "ementa: {ementa:?}"
            );
        }
    }

    /// O conservadorismo do D-LEG-12: sem artigo reconhecível no começo, NENHUMA chave. O item vai
    /// para o fim do grupo, e não para uma posição inventada.
    #[test]
    fn ementa_sem_artigo_no_comeco_nao_ganha_chave() {
        for ementa in [
            "",
            "   ",
            "Lei 8.820/1989, art. 5º", // remissão a outra norma: o começo não é o artigo
            "Parágrafo único",
            "Artigo 28",   // grafia por extenso não existe no acervo
            "Art.",        // sem número
            "Art. º",      // ordinal sem número
            "Arts. e 33",  // plural sem primeiro número
            "Artefato 12", // o que o `strip_prefix("Art")` cru deixaria passar
        ] {
            assert_eq!(chave_dispositivo(ementa), None, "ementa: {ementa:?}");
        }
    }

    /// A ordem que o índice produz é a de LEITURA da lei: artigo a artigo, com o acrescido logo
    /// depois do seu, e o que não parseia no fim — do grupo, não da lista.
    #[test]
    fn a_ordem_e_a_do_texto_legal_com_o_sem_chave_no_fim_do_grupo() {
        let mut ementas = [
            "Arts. 85 a 89",
            "Art. 18",
            "remissão sem artigo",
            "Art. 17-A",
            "Art. 17, § 3º",
            "Art. 2º",
            "Art. 136-F",
            "Art. 136-A",
        ];
        ementas.sort_by_key(|e| match chave_dispositivo(e) {
            Some((n, s)) => (0u8, n, s),
            None => (1u8, 0, 0),
        });
        assert_eq!(
            ementas,
            [
                "Art. 2º",
                "Art. 17, § 3º",
                "Art. 17-A", // o acrescido vem DEPOIS do 17 e antes do 18
                "Art. 18",
                "Arts. 85 a 89",
                "Art. 136-A",
                "Art. 136-F",
                "remissão sem artigo", // sem chave: fim
            ]
        );
    }
}
