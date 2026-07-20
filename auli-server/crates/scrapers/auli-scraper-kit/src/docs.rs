//! Emissão da árvore `.md` de pareceres pelos produtores (G5).
//!
//! Cada scraper de pareceres grava **um `.md` por consulta inédita** em
//! `data/<id>/docs/pareceres/`, em vez de despejar tudo num `.txt` intermediário. O documento nasce
//! **pendente** (sem `## sinopse`) — quem preenche é o passo `auli-collections <id> sinopse`.
//!
//! **"Existe ⇒ pula" é o incremental**, e é também a proteção: um `.md` já na árvore pode carregar
//! uma sinopse que custou LLM, então re-coletar nunca o toca. O contrato (`mddoc`) é quem decide o
//! nome do arquivo (slug do `numero`) e a forma; aqui só ficam o laço, a contagem e o relatório.

use std::path::Path;

use anyhow::Result;
use auli_contract::mddoc;

/// Uma consulta coletada, na forma mínima que o documento precisa.
pub struct DocParaEmitir<'a> {
    pub numero: &'a str,
    pub assunto: &'a str,
    pub link: &'a str,
    pub corpo: &'a str,
}

/// Emite os documentos inéditos em `dir`; devolve `(criados, pulados)`.
///
/// Não remove nada: a árvore só cresce por aqui. `numero` que não gera slug, ou dois `numero`
/// distintos disputando o mesmo arquivo, são **erro** — violação de identidade. A checagem mora no
/// contrato (`escrever_lote_se_ausente`), que é quem enxerga o lote inteiro; aqui só adaptamos a
/// forma do scraper.
pub fn emitir_pareceres(dir: &Path, docs: &[DocParaEmitir<'_>]) -> Result<(usize, usize)> {
    let lote: Vec<(mddoc::DocHeader, String)> = docs
        .iter()
        .map(|d| {
            (
                mddoc::DocHeader {
                    numero: d.numero.to_string(),
                    assunto: d.assunto.to_string(),
                    link: d.link.to_string(),
                    sinopse_info: None, // produtor emite pendente; a sinopse vem depois
                },
                d.corpo.to_string(),
            )
        })
        .collect();
    mddoc::escrever_lote_se_ausente(dir, &lote)
}

/// Relatório padrão dos produtores, para as 4 entidades falarem a mesma língua.
pub fn relatar(dir: &Path, criados: usize, pulados: usize) {
    println!(
        "✅ árvore {}: {criados} novo(s), {pulados} já existente(s) (pulados). \
         Rode `auli-collections <id> sinopse` para preencher os pendentes.",
        dir.display()
    );
}
