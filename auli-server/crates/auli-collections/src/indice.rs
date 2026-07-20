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
//! Derivação pura: rode de novo depois de qualquer passo que mexa na árvore (`pareceres`, `sinopse`,
//! ou os scrapers) e o resultado é função só do que está no disco.

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
        println!("ℹ️  {} não tem árvore de pareceres ({}); nada a derivar.", entity.id, dir.display());
        return Ok(());
    }

    let entradas = ler_arvore(&dir)?;
    let pendentes = entradas.iter().filter(|e| e.resumo.is_empty()).count();

    let destino = format!("{}/{}-pareceres-index.json", entity.data_dir, entity.id);
    std::fs::write(&destino, serde_json::to_string_pretty(&entradas)?)?;

    println!("✅ {} ({} pareceres, {pendentes} sem sinopse)", destino, entradas.len());
    if pendentes > 0 {
        println!("⚠️  rode `auli-collections {} sinopse` e derive de novo.", entity.id);
    }
    Ok(())
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
            format!("`{}` não parseia ({e}) — corrija antes de derivar o índice", caminho.display())
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
