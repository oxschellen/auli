// servicos collection scraper + derivação (`process`).
//
// O scrape grava a coleta no snapshot (`colecoes.servicos`): um registro por serviço, com a lista de
// `publicos` (D-S3). Dois backends:
//   - `rs` (SEFAZ-RS): headless Chrome + ureq raspam as páginas por público, gravando arquivos
//     per-tipo (recuperação de falha), e agregam em `ServicoRaw`.
//   - `sc` (SEF-SC): API JSON Next.js, sem browser; monta os `ServicoRaw` em memória (sem fan-out).
//
// `process` deriva do snapshot (offline) todos os artefatos: o contrato `<id>-servicos.json`
// (`Table<Servico>`), o print `portal-servicos.txt`, o `servicos-index.json` e os JSONs per-público
// (`<slug>.json`).

mod extrair_descricoes;
mod sc;
mod types;
mod utils;

use serde::Serialize;

use auli_scraper_kit::PerPublicoServicos;
use types::TipoServicos;

/// Raspa os serviços da entidade e grava a coleta no snapshot (`colecoes.servicos`). Não gera os
/// artefatos — [`process`] os deriva do snapshot em seguida. Despacha em `entity_id` (`rs` | `sc`).
pub fn run(entity_id: &str, data_dir: &str, use_cache: bool) -> Result<(), Box<dyn std::error::Error>> {
    match entity_id {
        "rs" => {
            let tipos = utils::get_tipo_servicos();
            let failed = extrair_descricoes::extrair_descricoes_json(data_dir, use_cache)?;
            let inputs = load_per_tipo(data_dir, &tipos)?;
            write_servicos_snapshot(entity_id, data_dir, &inputs, publicos_ordem_from(&tipos))?;
            report_failed_detail_urls(&failed);
        }
        "sc" => {
            let (inputs, publicos_ordem) = sc::scrape(data_dir, use_cache)?;
            write_servicos_snapshot(entity_id, data_dir, &inputs, publicos_ordem)?;
        }
        other => {
            return Err(format!(
                "scraper de servicos ainda não configurado para a entidade '{}'",
                other
            )
            .into());
        }
    }

    println!("🎉 Coleta de serviços gravada no snapshot.");
    Ok(())
}

/// Agrega os per-público em memória em `Vec<ServicoRaw>` e grava a coleta no snapshot.
fn write_servicos_snapshot(
    entity_id: &str,
    data_dir: &str,
    inputs: &PerPublicoServicos,
    publicos_ordem: Vec<auli_contract::Publico>,
) -> Result<(), Box<dyn std::error::Error>> {
    let items = auli_scraper_kit::aggregate_servicos(inputs);
    auli_scraper_kit::snapshot::write_servicos(
        entity_id,
        data_dir,
        &crate::scraper_info(),
        publicos_ordem,
        items,
    )?;
    Ok(())
}

/// Lê os arquivos per-tipo (na ordem de `tipos`) para a agregação — fluxo RS, onde o scrape os grava
/// incrementalmente como recuperação de falha. Arquivo ausente é ignorado.
fn load_per_tipo(
    data_dir: &str,
    tipos: &[TipoServicos],
) -> Result<PerPublicoServicos, Box<dyn std::error::Error>> {
    let mut loaded = Vec::new();
    for tipo in tipos {
        let path = format!("{}/{}.json", data_dir, tipo.filename);
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        loaded.push((tipo.tipo.clone(), utils::load_servicos_from_json(&path)?));
    }
    Ok(loaded)
}

/// `publicos_ordem` do snapshot a partir da lista de tipos (`tipo` -> `nome`, `filename` -> `slug`).
fn publicos_ordem_from(tipos: &[TipoServicos]) -> Vec<auli_contract::Publico> {
    tipos
        .iter()
        .map(|t| auli_contract::Publico { nome: t.tipo.clone(), slug: t.filename.clone() })
        .collect()
}

/// Prints a summary of the detail-page URLs that failed to load during the scrape.
fn report_failed_detail_urls(failed: &[String]) {
    if failed.is_empty() {
        println!("✅ Todas as páginas de detalhe carregaram com sucesso.");
        return;
    }

    eprintln!(
        "\n⚠️  {} página(s) de detalhe falharam ao carregar:",
        failed.len()
    );
    for url in failed {
        eprintln!("  - {}", url);
    }
}

/// Deriva os artefatos de serviços da coleta do snapshot (offline): o contrato `Table<Servico>`
/// (`<id>-servicos.json`), o print `portal-servicos.txt`, o `servicos-index.json` e os JSONs
/// per-público (`<slug>.json`). Não lê rede nem os per-tipo — só o snapshot.
pub fn process(
    id: &str,
    data_dir: &str,
    coleta: &auli_contract::ColetaServicos,
) -> Result<(), Box<dyn std::error::Error>> {
    let ordem = &coleta.publicos_ordem;

    // 1. Contrato: id sequencial (1..), tipo = público primário, text_to_embed materializado; a
    //    descricao já é o corpo limpo no snapshot.
    let items: Vec<auli_contract::Servico> = coleta
        .items
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (tipo, classe) = primary_ocorrencia(s, ordem)
                .map(|o| (o.publico.clone(), o.classe.clone()))
                .unwrap_or_default();
            let text_to_embed = servico_text_to_embed(&tipo, &classe, &s.titulo, &s.descricao);
            auli_contract::Servico {
                id: i + 1,
                tipo,
                classe,
                orgao: s.orgao.clone(),
                link: s.link.clone(),
                titulo: s.titulo.clone(),
                descricao: s.descricao.clone(),
                text_to_embed,
            }
        })
        .collect();
    let table = auli_contract::Table::new(id, "servicos", items);
    let contract_out = format!("{}/{}-servicos.json", data_dir, id);
    if let Some(parent) = std::path::Path::new(&contract_out).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&contract_out, serde_json::to_string_pretty(&table)?)?;
    println!("Wrote {} ({} serviços únicos)", contract_out, table.len());

    // 2. portal-servicos.txt: um bloco por serviço, breadcrumb `tipo | classe` (tipo = primário).
    let mut portal = String::new();
    for (i, s) in coleta.items.iter().enumerate() {
        let (tipo, classe) = primary_ocorrencia(s, ordem)
            .map(|o| (o.publico.as_str(), o.classe.as_str()))
            .unwrap_or_default();
        portal.push_str(&format!(
            "// {}.\n## pergunta\n{} | {}\n{}\n\n## resposta\n{}\nLink: {}\n\n",
            i + 1,
            tipo,
            classe,
            s.titulo,
            s.descricao,
            s.link
        ));
    }
    let portal_out = format!("{}/portal-servicos.txt", data_dir);
    std::fs::write(&portal_out, &portal)?;
    println!("Wrote {} ({} serviços únicos)", portal_out, coleta.items.len());

    // 3. servicos-index.json: { tipo: nome, filename: slug } na ordem de `publicos_ordem`.
    write_servicos_index(data_dir, ordem)?;

    // 4. per-público JSONs: fan-out — um arquivo por público, **uma entrada por `(link, classe)`**
    //    (restaura as listagens multi-classe do portal — D-F2.4.3); id local reiniciando em 1, na
    //    ordem dos items do snapshot; descricao = corpo limpo do snapshot (D-S5).
    for pubx in ordem {
        let mut local: Vec<types::Servico> = Vec::new();
        for s in &coleta.items {
            for oc in s.ocorrencias.iter().filter(|o| o.publico == pubx.nome) {
                local.push(types::Servico {
                    id: local.len() + 1,
                    tipo: pubx.nome.clone(),
                    classe: oc.classe.clone(),
                    orgao: s.orgao.clone(),
                    link: s.link.clone(),
                    titulo: s.titulo.clone(),
                    descricao: s.descricao.clone(),
                });
            }
        }
        let out = format!("{}/{}.json", data_dir, pubx.slug);
        std::fs::write(&out, serde_json::to_string_pretty(&local)?)?;
        println!("Wrote {} ({} serviços)", out, local.len());
    }

    Ok(())
}

/// Ocorrência primária de um serviço: a primeira encontrada iterando os públicos na ordem de
/// `publicos_ordem` (fallback: a primeira ocorrência na ordem de descoberta). Dá o `(tipo, classe)`
/// primário do contrato e o breadcrumb do print — mesma semântica first-occurrence da fase 1.
fn primary_ocorrencia<'a>(
    s: &'a auli_contract::ServicoRaw,
    ordem: &[auli_contract::Publico],
) -> Option<&'a auli_contract::Ocorrencia> {
    ordem
        .iter()
        .find_map(|p| s.ocorrencias.iter().find(|o| o.publico == p.nome))
        .or_else(|| s.ocorrencias.first())
}

/// `text_to_embed` for a service (D2): the breadcrumb `tipo | classe`, the title, and the start of
/// the description body. Provisional formula — the PLANO leaves the exact `servicos` key as a pending
/// item; re-vectorization is expected (the goal is retrieval equivalence, not bit-parity).
fn servico_text_to_embed(tipo: &str, classe: &str, titulo: &str, body: &str) -> String {
    let snippet: String = body.chars().take(300).collect();
    format!("{} | {}\n{}\n{}", tipo, classe, titulo, snippet.trim())
        .trim()
        .to_string()
}

/// One entry of `servicos-index.json` — drives the frontend's audience tabs.
#[derive(Serialize)]
struct ServicoIndexEntry {
    tipo: String,
    filename: String,
}

/// Writes `servicos-index.json`: the `{ tipo, filename }` tabs, in `publicos_ordem` order, so the
/// frontend can render the right audience tabs (and load the right files) without hardcoding them.
fn write_servicos_index(
    data_dir: &str,
    ordem: &[auli_contract::Publico],
) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<ServicoIndexEntry> = ordem
        .iter()
        .map(|p| ServicoIndexEntry { tipo: p.nome.clone(), filename: p.slug.clone() })
        .collect();

    let json = serde_json::to_string_pretty(&entries)?;
    let out = format!("{}/servicos-index.json", data_dir);
    std::fs::write(&out, json)?;
    println!("Wrote {} ({} tabs)", out, entries.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use auli_contract::Embeddable;

    #[test]
    fn text_to_embed_is_breadcrumb_title_and_body_start() {
        let key = servico_text_to_embed("Empresas", "ICMS", "Emitir guia", "Passos para emitir.");
        assert_eq!(key, "Empresas | ICMS\nEmitir guia\nPassos para emitir.");
    }

    #[test]
    fn primary_ocorrencia_follows_publicos_ordem() {
        let ordem = vec![
            auli_contract::Publico { nome: "Cidadãos".into(), slug: "rs-c".into() },
            auli_contract::Publico { nome: "Empresas".into(), slug: "rs-e".into() },
        ];
        // ocorrências fora de ordem: o primário deve seguir publicos_ordem (Cidadãos), não a lista.
        let s = auli_contract::ServicoRaw {
            titulo: "T".into(),
            descricao: "corpo".into(),
            link: "l".into(),
            orgao: "O".into(),
            ocorrencias: vec![
                auli_contract::Ocorrencia { publico: "Empresas".into(), classe: "X".into() },
                auli_contract::Ocorrencia { publico: "Cidadãos".into(), classe: "Y".into() },
            ],
        };
        let oc = primary_ocorrencia(&s, &ordem).unwrap();
        assert_eq!((oc.publico.as_str(), oc.classe.as_str()), ("Cidadãos", "Y"));
    }

    /// Equivalência golden (etapa E) — inerte sem `AULI_GOLDEN_DATA` (raiz do `data/` do repo, ex.:
    /// `/home/ubu/Desktop/auli/data`). Sintetiza as coletas a partir dos intermediários existentes
    /// (`faqs.json` + per-tipo), roda as derivações do `process` num diretório temporário e compara
    /// byte a byte os 5 artefatos RS. Rodar com: `AULI_GOLDEN_DATA=<...> cargo test -- --ignored golden`.
    #[test]
    #[ignore = "gated por AULI_GOLDEN_DATA"]
    fn golden_rs_equivalence() {
        let Ok(root) = std::env::var("AULI_GOLDEN_DATA") else { return };
        let rs_raw = format!("{}/rs/raw", root);

        // Coleta de faqs: árvore faqs.json -> flatten (sem text_to_embed).
        let tree_bytes = std::fs::read(format!("{}/faqs.json", rs_raw)).unwrap();
        let tree: crate::faqs::FaqNode = serde_json::from_slice(&tree_bytes).unwrap();
        let coleta_faqs = auli_contract::ColetaFaqs {
            coletado_em: String::new(),
            items: crate::faqs::flatten_faqs_raw(&tree),
        };

        // Coleta de serviços: agrega os per-tipo (mesma ordem/dedup do contrato antigo).
        let tipos = utils::get_tipo_servicos();
        let inputs = load_per_tipo(&rs_raw, &tipos).unwrap();
        let coleta_servicos = auli_contract::ColetaServicos {
            coletado_em: String::new(),
            publicos_ordem: publicos_ordem_from(&tipos),
            items: auli_scraper_kit::aggregate_servicos(&inputs),
        };

        // Deriva num diretório temporário e compara com o golden.
        let out = std::env::temp_dir().join(format!("auli_golden_{}", std::process::id()));
        let out = out.to_str().unwrap();
        std::fs::create_dir_all(out).unwrap();
        crate::faqs::process("rs", out, &coleta_faqs).unwrap();
        process("rs", out, &coleta_servicos).unwrap();

        let mut checked = 0;
        for f in [
            "rs-faqs.json",
            "rs-servicos.json",
            "portal-faqs.txt",
            "portal-servicos.txt",
            "servicos-index.json",
        ] {
            let golden = format!("{}/{}", rs_raw, f);
            if !std::path::Path::new(&golden).exists() {
                eprintln!("⏭️  golden ausente, pulando: {}", f);
                continue;
            }
            let got = std::fs::read(format!("{}/{}", out, f)).unwrap();
            let want = std::fs::read(&golden).unwrap();
            // Tolera diferença de só newline(s) finais: o `to_string_pretty` (código antigo e novo) não
            // emite `\n` final, mas alguns golden em disco foram normalizados por editor.
            let trim = |v: &[u8]| {
                let mut e = v.len();
                while e > 0 && v[e - 1] == b'\n' {
                    e -= 1;
                }
                v[..e].to_vec()
            };
            if got != want {
                eprintln!("ℹ️  {}: difere só por newline final (got {}, want {} bytes)", f, got.len(), want.len());
            }
            assert!(
                trim(&got) == trim(&want),
                "artefato diverge do golden (conteúdo): {} (got {} bytes, want {} bytes)",
                f,
                got.len(),
                want.len()
            );
            checked += 1;
        }
        eprintln!("✅ golden RS: {} artefato(s) conferidos byte a byte", checked);
        let _ = std::fs::remove_dir_all(out);
    }

    #[test]
    fn contract_servico_stored_repr_matches_print_block() {
        // descricao já é o CORPO (sem o header tipo/classe/titulo), como gravado no contrato.
        let s = auli_contract::Servico {
            id: 1,
            tipo: "Empresas".into(),
            classe: "ICMS".into(),
            orgao: "SEFAZ".into(),
            link: "https://x/svc/1".into(),
            titulo: "Emitir guia".into(),
            descricao: "Passos para emitir a guia.".into(),
            text_to_embed: "Empresas | ICMS\nEmitir guia\nPassos para emitir a guia.".into(),
        };
        // Mesmo conteúdo do bloco de portal-servicos.txt (sem o `// N.` e a newline final).
        let expected = "## pergunta\nEmpresas | ICMS\nEmitir guia\n\n## resposta\nPassos para emitir a guia.\nLink: https://x/svc/1";
        assert_eq!(s.stored_repr(), expected);
    }
}
