// servicos — derivação (`process`) dos artefatos a partir da coleta do snapshot.
//
// Deriva (offline) o contrato `<id>-servicos.json` (`Table<Servico>`), o print `<id>-portal-servicos.txt`,
// o `<id>-servicos-index.json` e os JSONs per-público (`<id>-<slug>.json`, uma entrada por
// `(link, classe)` — restaura o multi-classe). **Todo artefato de `raw/` é prefixado por `<id>-`**
// (ver `raw_out`). Os scrapers são os binários `auli-scraper-rs` / `auli-scraper-sc`.

mod types;

use serde::Serialize;

use crate::errors::Result;

/// Caminho de um artefato de `raw/`, prefixado pela entidade: `raw_out("../data/rs/raw", "rs",
/// "servicos-index.json")` -> `../data/rs/raw/rs-servicos-index.json`. Namespaceia todo o `raw/` por
/// entidade (os contratos do engine já seguiam essa convenção); o prefixo `<id>-` deixa de ser só
/// uma preocupação do boundary do `public/` e passa a valer na origem.
fn raw_out(data_dir: &str, id: &str, name: &str) -> String {
    format!("{}/{}-{}", data_dir, id, name)
}

/// Deriva os artefatos de serviços da coleta do snapshot (offline): contrato `Table<Servico>`,
/// `portal-servicos.txt`, `servicos-index.json` e os JSONs per-público. Não lê rede — só o snapshot.
pub fn process(id: &str, data_dir: &str, coleta: &auli_contract::ColetaServicos) -> Result<()> {
    let ordem = &coleta.publicos_ordem;
    let coletados = fundir_duplicatas(&coleta.items);

    // 1. Contrato: id sequencial (1..), tipo = público primário, text_to_embed materializado; a
    //    descricao já é o corpo limpo no snapshot.
    let items: Vec<auli_contract::Servico> = coletados
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
    let contract_out = raw_out(data_dir, id, "servicos.json");
    if let Some(parent) = std::path::Path::new(&contract_out).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&contract_out, serde_json::to_string_pretty(&table)?)?;
    println!("Wrote {} ({} serviços únicos)", contract_out, table.len());

    // 2. portal-servicos.txt: um bloco por serviço, breadcrumb `tipo | classe` (tipo = primário).
    let mut portal = String::new();
    for (i, s) in coletados.iter().enumerate() {
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
    let portal_out = raw_out(data_dir, id, "portal-servicos.txt");
    std::fs::write(&portal_out, &portal)?;
    println!("Wrote {} ({} serviços únicos)", portal_out, coletados.len());

    // 3. <id>-servicos-index.json: { tipo: nome, filename: slug } na ordem de `publicos_ordem`.
    write_servicos_index(data_dir, id, ordem)?;

    // 4. per-público JSONs: fan-out — um arquivo por público, **uma entrada por `(link, classe)`**
    //    (restaura as listagens multi-classe do portal — D-F2.4.3); id local reiniciando em 1, na
    //    ordem dos items do snapshot; descricao = corpo limpo do snapshot (D-S5).
    for pubx in ordem {
        let mut local: Vec<types::Servico> = Vec::new();
        for s in &coletados {
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
        let out = raw_out(data_dir, id, &format!("{}.json", pubx.slug));
        std::fs::write(&out, serde_json::to_string_pretty(&local)?)?;
        println!("Wrote {} ({} serviços)", out, local.len());
    }

    // 5. Árvore `.md` — a FONTE do `auli update` para serviços (TAREFA-SERVICOS-MD). Regenerada do
    //    zero a cada process: serviços mudam no portal, então não há incremental (D1). Os artefatos
    //    de `raw/` acima seguem intactos — o frontend consome os per-público, o update lê a árvore.
    let base = std::path::Path::new(data_dir)
        .parent()
        .ok_or_else(|| format!("data_dir sem pai: {data_dir}"))?;
    let docs_dir = base.join("docs").join("servicos");
    let docs: Vec<(auli_contract::mddoc::DocHeader, String)> = table
        .items
        .iter()
        .map(|s| (cabecalho_servico(s), s.descricao.clone()))
        .collect();
    let gravados = auli_contract::mddoc::materializar_arvore(
        &docs_dir,
        &docs,
        auli_contract::mddoc::nome_servico,
    )
    .map_err(|e| format!("materialização da árvore de serviços: {e}"))?;
    println!(
        "📄 docs: {} serviços materializados em {}",
        gravados,
        docs_dir.display()
    );

    Ok(())
}

/// Funde itens que a árvore veria como o MESMO documento — o mesmo serviço catalogado sob mais de
/// uma classe —, unindo as ocorrências (sem repetir pares `(publico, classe)` iguais). Ordem
/// preservada: o primeiro item de cada grupo vence a posição e os campos escalares.
///
/// **A chave é o nome de arquivo** (`slug_servico` = `slug(titulo) + hash(link)`), não o par
/// `(titulo, link)` cru: o slug dobra caixa, acento e pontuação, então "…não Inscritos" e
/// "…Não Inscritos" no mesmo link são o mesmo `.md`. Fundir por uma chave mais fina deixaria passar
/// exatamente os casos que a árvore rejeita — o SP tem os dois tipos.
///
/// **Por que aqui e não no scraper:** o `link` é a chave natural do snapshot, mas nem todo scraper
/// dedupa por ele (o SP monta os `ServicoRaw` direto, e lá 43 serviços chegam a compartilhar a URL
/// de login). Este é o ponto único por onde todos passam. Sem a fusão, o par colide na
/// materialização da árvore — cujo nome de arquivo é `slug(titulo) + hash(link)` —, e a colisão é
/// erro alto, não pulo silencioso.
///
/// **O que se perde:** nada nos JSONs per-público, que continuam emitindo uma entrada por
/// `(link, classe)` — as classes secundárias sobrevivem nas ocorrências fundidas. No contrato
/// agregado e no print, a entrada duplicada some (o que é o objetivo: uma por serviço). Se as
/// descrições divergirem entre os itens fundidos, a do primeiro vence e o caso é impresso — é
/// perda real de texto, então não pode acontecer em silêncio.
///
/// "O primeiro" herda uma ordem com significado: a da coleta, que segue a listagem do próprio
/// portal. As variantes não são truncamentos uma da outra — são textos que o portal escreve POR
/// CLASSE.
/// Cabeçalho do serviço no vocabulário unificado (D-FMT-2): o título é o `titulo`, o breadcrumb
/// `tipo | classe` é a `trilha` (pré-composta pelo contrato, byte-idêntica à do `text_to_embed` e à
/// do bloco RAG), a `ementa` fica vazia e o `orgao` é o passthrough de D-FMT-3.
fn cabecalho_servico(s: &auli_contract::Servico) -> auli_contract::mddoc::DocHeader {
    auli_contract::mddoc::DocHeader {
        titulo: s.titulo.clone(),
        trilha: auli_contract::trilha_servico(&s.tipo, &s.classe),
        ementa: String::new(),
        link: s.link.clone(),
        orgao: Some(s.orgao.clone()),
        resumo_info: None,
    }
}

fn fundir_duplicatas(items: &[auli_contract::ServicoRaw]) -> Vec<auli_contract::ServicoRaw> {
    let mut out: Vec<auli_contract::ServicoRaw> = Vec::with_capacity(items.len());
    let mut onde: std::collections::HashMap<String, usize> =
        std::collections::HashMap::with_capacity(items.len());
    let mut divergentes: Vec<&str> = Vec::new();

    for s in items {
        let chave = auli_contract::mddoc::nome_servico_de(&s.titulo, &s.link);
        match onde.get(&chave) {
            Some(&i) => {
                let alvo = &mut out[i];
                if alvo.descricao != s.descricao {
                    divergentes.push(&s.titulo);
                }
                for oc in &s.ocorrencias {
                    if !alvo
                        .ocorrencias
                        .iter()
                        .any(|o| o.publico == oc.publico && o.classe == oc.classe)
                    {
                        alvo.ocorrencias.push(oc.clone());
                    }
                }
            }
            None => {
                onde.insert(chave, out.len());
                out.push(s.clone());
            }
        }
    }

    let fundidos = items.len() - out.len();
    if fundidos > 0 {
        println!(
            "🔗 {fundidos} item(ns) fundido(s) por identidade de arquivo — mesmo serviço em mais de uma classe"
        );
    }
    if !divergentes.is_empty() {
        println!(
            "⚠️  descrição divergente entre itens fundidos (vale a do primeiro): {}",
            divergentes.join("; ")
        );
    }
    out
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
/// the description body. A fórmula mora no CONTRATO (`compose_servico_text_to_embed`) porque os dois
/// lados precisam dela — este produtor e o `auli update`, que a rematerializa ao ler a árvore
/// `docs/servicos/*.md` (TAREFA-SERVICOS-MD). Aqui fica só a delegação.
fn servico_text_to_embed(tipo: &str, classe: &str, titulo: &str, body: &str) -> String {
    auli_contract::compose_servico_text_to_embed(tipo, classe, titulo, body)
}

/// One entry of `servicos-index.json` — drives the frontend's audience tabs.
#[derive(Serialize)]
struct ServicoIndexEntry {
    tipo: String,
    filename: String,
}

/// Writes `<id>-servicos-index.json`: the `{ tipo, filename }` tabs, in `publicos_ordem` order, so the
/// frontend can render the right audience tabs (and load the right files) without hardcoding them.
///
/// `filename` stays the **bare** slug (no `<id>-` prefix): it's a public/-facing logical name that the
/// frontend resolves via `entityPath` (which prepends `<id>-`). Only the index *file* is prefixed.
fn write_servicos_index(data_dir: &str, id: &str, ordem: &[auli_contract::Publico]) -> Result<()> {
    let entries: Vec<ServicoIndexEntry> = ordem
        .iter()
        .map(|p| ServicoIndexEntry {
            tipo: p.nome.clone(),
            filename: p.slug.clone(),
        })
        .collect();

    let json = serde_json::to_string_pretty(&entries)?;
    let out = raw_out(data_dir, id, "servicos-index.json");
    std::fs::write(&out, json)?;
    println!("Wrote {} ({} tabs)", out, entries.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_to_embed_is_breadcrumb_title_and_body_start() {
        let key = servico_text_to_embed("Empresas", "ICMS", "Emitir guia", "Passos para emitir.");
        assert_eq!(key, "Empresas | ICMS\nEmitir guia\nPassos para emitir.");
    }

    #[test]
    fn primary_ocorrencia_follows_publicos_ordem() {
        let ordem = vec![
            auli_contract::Publico {
                nome: "Cidadãos".into(),
                slug: "rs-c".into(),
            },
            auli_contract::Publico {
                nome: "Empresas".into(),
                slug: "rs-e".into(),
            },
        ];
        // ocorrências fora de ordem: o primário deve seguir publicos_ordem (Cidadãos), não a lista.
        let s = auli_contract::ServicoRaw {
            titulo: "T".into(),
            descricao: "corpo".into(),
            link: "l".into(),
            orgao: "O".into(),
            ocorrencias: vec![
                auli_contract::Ocorrencia {
                    publico: "Empresas".into(),
                    classe: "X".into(),
                },
                auli_contract::Ocorrencia {
                    publico: "Cidadãos".into(),
                    classe: "Y".into(),
                },
            ],
        };
        let oc = primary_ocorrencia(&s, &ordem).unwrap();
        assert_eq!((oc.publico.as_str(), oc.classe.as_str()), ("Cidadãos", "Y"));
    }

    /// O caso real do SP: o mesmo serviço (mesmo título, mesmo link) listado sob duas classes chega
    /// como DOIS itens. Sem fundir, os dois disputam o mesmo `.md` e a materialização da árvore erra.
    #[test]
    fn fundir_duplicatas_une_ocorrencias_e_preserva_ordem() {
        let raw =
            |titulo: &str, link: &str, classe: &str, descricao: &str| auli_contract::ServicoRaw {
                titulo: titulo.into(),
                descricao: descricao.into(),
                link: link.into(),
                orgao: "SEFAZ".into(),
                ocorrencias: vec![auli_contract::Ocorrencia {
                    publico: "Cidadão".into(),
                    classe: classe.into(),
                }],
            };
        let items = vec![
            raw("SIVEI", "https://x/sivei", "IPVA", "corpo"),
            raw("Outro", "https://x/outro", "ICMS", "corpo"),
            raw("SIVEI", "https://x/sivei", "ISENÇÕES", "corpo"),
            // Mesmo título em link diferente NÃO funde — são serviços distintos.
            raw("SIVEI", "https://x/sivei-2", "IPVA", "corpo"),
        ];
        let out = fundir_duplicatas(&items);
        assert_eq!(out.len(), 3, "só o par (SIVEI, /sivei) funde");
        // Ordem preservada: o primeiro de cada par vence a posição.
        let titulos: Vec<&str> = out.iter().map(|s| s.titulo.as_str()).collect();
        assert_eq!(titulos, vec!["SIVEI", "Outro", "SIVEI"]);
        let classes: Vec<&str> = out[0]
            .ocorrencias
            .iter()
            .map(|o| o.classe.as_str())
            .collect();
        assert_eq!(
            classes,
            vec!["IPVA", "ISENÇÕES"],
            "as duas classes sobrevivem"
        );

        // Ocorrência idêntica repetida não duplica.
        let repetido = vec![
            raw("A", "https://x/a", "ICMS", "corpo"),
            raw("A", "https://x/a", "ICMS", "corpo"),
        ];
        assert_eq!(fundir_duplicatas(&repetido)[0].ocorrencias.len(), 1);

        // Caixa/acento diferentes no MESMO link também fundem: o slug dobra os dois, então a árvore
        // os veria como o mesmo `.md` — é o segundo caso real do SP ("não" vs "Não").
        let caixa = vec![
            raw(
                "eCND - Débitos não Inscritos",
                "https://x/ecnd",
                "ICMS",
                "corpo",
            ),
            raw(
                "eCND - Débitos Não Inscritos",
                "https://x/ecnd",
                "CERTIDÕES",
                "corpo",
            ),
        ];
        let out = fundir_duplicatas(&caixa);
        assert_eq!(out.len(), 1, "diferença só de caixa colidiria na árvore");
        assert_eq!(out[0].ocorrencias.len(), 2);

        // Sem duplicatas, a lista sai intacta (o caso do RS — 586 itens, 0 pares).
        let intacto = vec![
            raw("A", "https://x/a", "ICMS", "corpo"),
            raw("B", "https://x/b", "IPVA", "corpo"),
        ];
        assert_eq!(fundir_duplicatas(&intacto).len(), 2);
    }

    #[test]
    fn contract_servico_bloco_matches_print_block() {
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
        let expected = "## pergunta\nEmpresas | ICMS\nEmitir guia\n\n## resposta\nPassos para emitir a guia.\nLink: https://x/svc/1";
        // Desde a B5 o serviço não renderiza bloco sozinho: ele se projeta no `Documento`, e quem
        // renderiza é o `bloco` único do contrato. O literal esperado NÃO mudou — é o mesmo bloco.
        let d = s.para_documento();
        assert_eq!(auli_contract::bloco(&d.pack(), &d.corpo), expected);
    }

    /// Equivalência golden (D-F2.7) — inerte sem `AULI_GOLDEN_DATA` (raiz do `data/` do repo). Lê os
    /// **snapshots v3 reais** (por coleção), roda as derivações do `process` num diretório temporário e compara com os
    /// artefatos golden em `data/rs/raw/`: agregados byte a byte (tolerando `\n` final); per-público por
    /// `(link, classe)` — incluindo os multi-classe restaurados; ordem/id fora do gate (D-S5).
    #[test]
    #[ignore = "gated por AULI_GOLDEN_DATA"]
    fn golden_rs_equivalence() {
        let Ok(root) = std::env::var("AULI_GOLDEN_DATA") else {
            return;
        };
        let rs_raw = format!("{}/rs/raw", root);

        let faqs =
            auli_contract::snapshot::load::<auli_contract::ColetaFaqs>("rs", &rs_raw, "faqs")
                .unwrap();
        let servicos = auli_contract::snapshot::load::<auli_contract::ColetaServicos>(
            "rs", &rs_raw, "servicos",
        )
        .unwrap()
        .expect("snapshot de serviços rs ausente — rode o scraper primeiro");

        // O temporário imita a forma real `<base>/raw`: o `process` materializa a árvore
        // `docs/servicos/` no PAI do `data_dir`, e ela precisa cair dentro do temporário.
        let base = std::env::temp_dir().join(format!("auli_golden_{}", std::process::id()));
        let out_dir = base.join("raw");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out = out_dir.to_str().unwrap();
        if let Some(snap) = &faqs {
            crate::derive_faqs::process("rs", out, &snap.coleta).unwrap();
        }
        process("rs", out, &servicos.coleta).unwrap();

        // Agregados: byte a byte (tolerando newline final que editores adicionam ao golden em disco).
        let trim = |v: Vec<u8>| {
            let mut e = v.len();
            while e > 0 && v[e - 1] == b'\n' {
                e -= 1;
            }
            v[..e].to_vec()
        };
        let mut checked = 0;
        for f in [
            "rs-faqs.json",
            "rs-servicos.json",
            "rs-portal-faqs.txt",
            "rs-portal-servicos.txt",
            "rs-servicos-index.json",
        ] {
            let golden = format!("{}/{}", rs_raw, f);
            if !std::path::Path::new(&golden).exists() {
                eprintln!("⏭️  golden ausente, pulando: {}", f);
                continue;
            }
            let got = trim(std::fs::read(format!("{}/{}", out, f)).unwrap());
            let want = trim(std::fs::read(&golden).unwrap());
            assert!(got == want, "artefato diverge do golden: {}", f);
            checked += 1;
        }

        // Per-público: multiset (link, classe, titulo, orgao) idêntico ao golden (multi-classe incl.).
        for pubx in &servicos.coleta.publicos_ordem {
            let key = |bytes: &[u8]| {
                let v: Vec<auli_contract::ServicoPerPublico> =
                    serde_json::from_slice(bytes).unwrap();
                let mut k: Vec<_> = v
                    .iter()
                    .map(|s| format!("{}|{}|{}|{}", s.link, s.classe, s.titulo, s.orgao))
                    .collect();
                k.sort();
                k
            };
            let g = format!("{}/rs-{}.json", rs_raw, pubx.slug);
            if !std::path::Path::new(&g).exists() {
                continue;
            }
            let got = key(&std::fs::read(format!("{}/rs-{}.json", out, pubx.slug)).unwrap());
            let want = key(&std::fs::read(&g).unwrap());
            assert!(
                got == want,
                "per-público diverge (link,classe): {}",
                pubx.slug
            );
        }

        eprintln!(
            "✅ golden RS: {} agregados + per-público conferidos",
            checked
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
