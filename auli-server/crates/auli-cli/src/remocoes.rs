//! `auli remover` — aplica as remoções que um humano aprovou. Fase 4 da TAREFA-UPDATE-INCREMENTAL.
//!
//! **É a única parte do pipeline que apaga documento do acervo**, e por isso nada aqui roda por
//! acidente. O `auli update` nunca remove: ele escreve um log de órfãos e segue (D-INC-9). Remover
//! exige rodar este subcomando, de propósito.
//!
//! **Subcomando e não flag** (D-INC-10). Uma `--aplicar-remocoes` acabaria dentro do `update-*.sh`
//! "para não rodar duas vezes", e o portão humano desapareceria sem ninguém decidir removê-lo. Um
//! subcomando exige invocação deliberada — e é assimétrico de propósito: escrever o log é barato e
//! automático, apagar não.
//!
//! **A regra:** aplica exatamente `aprovados ∩ órfãos_atuais`. Os dois lados importam:
//!
//! - a interseção com os **órfãos atuais** garante que um documento que voltou para a árvore entre a
//!   revisão e a aplicação **não seja removido** — o log envelheceu, o disco não;
//! - a interseção com os **aprovados** garante que só sai o que foi lido por alguém. Órfão novo, que
//!   apareceu depois da revisão, não entra de carona.
//!
//! Não recusa a rodada porque o diff mudou em algo irrelevante — se recusasse, um documento novo
//! chegando entre a revisão e a aplicação obrigaria a revisar tudo de novo, e a válvula viraria
//! obstáculo. Recusa apenas **remover o que não foi lido**.
//!
//! **Idempotente:** rodar duas vezes não faz mal. Na segunda, os aprovados já não são órfãos (nem
//! estão no pack), a interseção é vazia e nada acontece.
//!
//! A aprovação é **seletiva por linha**: o humano apaga do log as linhas que não quer autorizar.
//! Elas voltam no log da próxima rodada do `update`, porque não aprovar hoje é "não agora", não uma
//! decisão permanente.
//!
//! **O log é consumido aqui, e regenerado só pelo `update`.** Regenerá-lo neste subcomando, com os
//! órfãos que sobraram, devolveria ao arquivo exatamente as linhas que o humano apagou para NÃO
//! aprovar — e a próxima invocação as leria como aprovação. O "não" viraria "sim" sozinho, sem
//! ninguém decidir. Defeito encontrado pelo teste de ponta a ponta da Fase 4, não por leitura.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use auli_contract::Kind;
use auli_core::manifest::{self, Manifest};
use vector_store::Writer;

use crate::error::Result;
use crate::update;

/// O que aconteceu com uma coleção, nas três classes que o D-INC-10 exige relatar.
struct Relatorio {
    aplicados: Vec<String>,
    /// Órfãos de hoje que o log não aprovou — continuam no log, para a próxima revisão.
    nao_aprovados: usize,
    /// Aprovados que deixaram de ser órfãos: o documento reapareceu na árvore. Descartados.
    reapareceram: Vec<String>,
}

/// Aplica as remoções aprovadas de uma entidade. A regra e o portão estão no doc do módulo.
///
/// **Não há atomicidade entre o pack e o manifesto.** São dois arquivos: cada um é escrito de forma
/// atômica (D-INC-5), os dois juntos não. Morrer na janela entre as duas escritas deixa o pack novo
/// com o manifesto velho, e o boot recusa por hash de pack divergente — sem meio de adivinhar que a
/// causa foi esta. A recuperação é `auli update`: ele reconstrói o pack a partir da árvore e
/// recarimba tudo. As remoções já aplicadas **permanecem**, porque os órfãos não estão na árvore de
/// qualquer forma; e o log, consumido antes da falha, é regenerado por lá com os órfãos que
/// sobraram (D-INC-9). Recuperável e limpo, portanto — mas só para quem sabe disto.
pub fn run_remover(entity: String, out: PathBuf) -> Result<()> {
    let docs_dir = out
        .parent()
        .ok_or_else(|| format!("diretório de packs sem pai: {}", out.display()))?
        .join("docs");
    let writer = Writer::new(&out);

    // O manifesto é lido ANTES de qualquer escrita: sem ele não há o que recarimbar, e um pack
    // alterado com manifesto velho faz o boot recusar sem dizer por quê (D-INC-11).
    let mpath = manifest::manifest_path(&out, &entity);
    let mut manifesto: Manifest = manifest::read_manifest(&mpath).map_err(|e| {
        format!(
            "não consegui ler o manifesto de '{entity}' em {}: {e}. \
             Rode `auli update` antes — remover sem recarimbar deixaria o boot recusando.",
            mpath.display()
        )
    })?;

    // A árvore tem de estar em sincronia com o pack ANTES de qualquer escrita. Se ela andou desde o
    // último `auli update` — um acórdão novo do scraper, por exemplo —, o pack não viu o que chegou:
    // decidir remoções aqui seria decidir sobre o diff errado, o mesmo raciocínio do `ids_na_arvore`
    // logo abaixo. E recarimbar o manifesto no fim faria pior que isso: apagaria o carimbo que faz o
    // boot recusar um pack atrasado, deixando o documento novo ausente do índice em silêncio.
    // Por isso a recusa vem antes de escrever pack, manifesto ou de consumir log.
    //
    // **Nota histórica (D-DH-2):** até o mapa por diretório, esta guarda e a preservação do carimbo
    // lá embaixo defendiam cenários DISJUNTOS — o manifesto sem `docs_hash` com árvore em disco só
    // era pego pela preservação, porque a guarda devolvia `Ok` para o campo ausente. O D-DH-2
    // fechou esse braço: mapa ausente com árvore presente é erro alto AQUI, na guarda. A
    // preservação lá embaixo continua correta (o `remover` não toca a árvore e escreve de volta o
    // que leu), mas o papel dela mudou: é round-trip, não defesa independente.
    manifest::validate_docs_hashes(&docs_dir, &manifesto).map_err(|e| {
        format!("{e}\nA árvore está à frente do pack: rode `auli update` antes do `remover`.")
    })?;

    let mut mexeu = false;
    for kind in Kind::TODOS {
        // Só a jurisprudência acumula órfãos: serviços e faqs são reescritos por inteiro a cada
        // rodada (D-INC-8), então o que sumiu da árvore já saiu do pack sozinho.
        if !kind.pack_incremental() {
            continue;
        }
        let name = format!("{}-{}", entity, kind.as_str());
        let Some(aprovados) = ler_aprovados(&update::caminho_log_remocoes(&out, &name))? else {
            continue; // sem log, nada foi aprovado para esta coleção
        };

        let na_arvore = ids_na_arvore(&entity, &docs_dir, kind)?;
        let orfaos = update::orfaos_do_pack(&writer.path_for(&name), &na_arvore)?;
        let ids_orfaos: HashSet<&str> = orfaos.iter().map(|(id, _, _)| id.as_str()).collect();

        let relatorio = Relatorio {
            aplicados: aprovados
                .iter()
                .filter(|id| ids_orfaos.contains(id.as_str()))
                .cloned()
                .collect(),
            nao_aprovados: orfaos
                .iter()
                .filter(|(id, _, _)| !aprovados.contains(id))
                .count(),
            reapareceram: aprovados
                .iter()
                .filter(|id| !ids_orfaos.contains(id.as_str()))
                .cloned()
                .collect(),
        };

        relatar(&name, &relatorio);
        if relatorio.aplicados.is_empty() {
            continue;
        }

        let total = writer.apply::<String>(&name, vec![], &relatorio.aplicados)?;
        // O carimbo é a MESMA função do `update` (D-INC-11) — não uma segunda cópia que envelhece.
        let entrada = update::carimbar_colecao(&out, kind, &name, total)?;
        match manifesto
            .collections
            .iter_mut()
            .find(|c| c.kind == entrada.kind)
        {
            Some(c) => *c = entrada,
            None => manifesto.collections.push(entrada),
        }
        mexeu = true;

        // O log é CONSUMIDO, não regenerado. Regenerá-lo aqui com os órfãos que sobraram seria
        // devolver ao arquivo justamente as linhas que o humano APAGOU para não aprovar — e a
        // próxima rodada do `remover` as leria como aprovação. O "não" viraria "sim" sozinho.
        //
        // Quem regenera é o `auli update`, do diff da rodada seguinte (D-INC-9). O órfão não
        // aprovado volta ao log por lá, que é o que faz "não aprovar hoje" significar "não agora".
        std::fs::remove_file(update::caminho_log_remocoes(&out, &name))?;
        println!(
            "   📋 {name}: log consumido. O que não foi aprovado volta no próximo `auli update`."
        );
    }

    if !mexeu {
        println!("✅ Nada a remover — nenhum pack e nenhum manifesto foi tocado.");
        return Ok(());
    }

    // O mapa `docs_hashes` lido é escrito de volta INTACTO. O `remover` não toca a árvore, e a
    // guarda no início já provou que o carimbo lido descreve a árvore atual — recalcular aqui
    // seria um no-op comprado ao preço de uma leitura da árvore inteira. Round-trip, portanto
    // (ver a nota histórica na guarda: o cenário que fazia desta linha uma defesa independente
    // foi fechado pelo D-DH-2).
    manifest::write_manifest(&mpath, &manifesto)?;
    println!("📝 Manifesto recarimbado em {:?}", mpath);
    Ok(())
}

/// Lê os `doc_path` aprovados de um log. `None` se o arquivo não existe — nada foi aprovado.
///
/// Ignora comentários (`#`) e linhas vazias, e lê **só o primeiro campo**: o resto da linha é para o
/// humano. Um log editado à mão que perdeu as colunas de título e link continua válido, porque a
/// chave é a única coisa que o programa precisa.
fn ler_aprovados(caminho: &Path) -> Result<Option<HashSet<String>>> {
    if !caminho.exists() {
        return Ok(None);
    }
    let texto = std::fs::read_to_string(caminho)?;
    Ok(Some(
        texto
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.split('\t').next().unwrap_or(l).trim().to_string())
            .filter(|id| !id.is_empty())
            .collect(),
    ))
}

/// Os `doc_path` que a árvore tem hoje para uma coleção.
///
/// Deriva do MESMO `preparar` que o `update` usa, e não de um `read_dir` próprio: o `doc_path` de
/// serviços e faqs leva hash do link, então recompor o nome à mão aqui abriria a chance de divergir
/// do que o pack gravou — e divergir aqui significa apagar documento bom.
///
/// Herda daí a guarda de resumo pendente, e isso é desejável: um `.md` sem `## resumo` faz o
/// `auli update` recusar, logo o pack está atrás da árvore — e decidir remoções a partir de uma
/// árvore que o pack ainda não viu é decidir sobre o diff errado. A recusa vem com o remédio certo
/// porque a entidade é repassada.
fn ids_na_arvore(entity: &str, docs_dir: &Path, kind: Kind) -> Result<HashSet<String>> {
    Ok(update::preparar(entity, docs_dir, kind)?
        .map(|docs| docs.iter().map(|d| d.doc_path()).collect())
        .unwrap_or_default())
}

fn relatar(name: &str, r: &Relatorio) {
    if !r.aplicados.is_empty() {
        println!("🗑️  {name}: removendo {} registro(s):", r.aplicados.len());
        for id in r.aplicados.iter().take(10) {
            println!("      {id}");
        }
        if r.aplicados.len() > 10 {
            println!("      … e mais {}", r.aplicados.len() - 10);
        }
    }
    if r.nao_aprovados > 0 {
        println!(
            "   ⏸  {name}: {} órfão(s) NÃO aprovados — seguem no log para a próxima revisão",
            r.nao_aprovados
        );
    }
    if !r.reapareceram.is_empty() {
        println!(
            "   ↩️  {name}: {} aprovado(s) deixaram de ser órfãos (o documento voltou para a \
             árvore) — NÃO removidos:",
            r.reapareceram.len()
        );
        for id in r.reapareceram.iter().take(10) {
            println!("      {id}");
        }
    }
    if r.aplicados.is_empty() && r.nao_aprovados == 0 && r.reapareceram.is_empty() {
        println!("   ✅ {name}: log vazio, nada a fazer");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auli_contract::{Documento, Embeddable, mddoc};
    use auli_core::embed::EMBED_DIM;
    use auli_core::manifest::{CollectionEntry, Manifest};
    use vector_store::{CollectionData, Record};

    /// Monta um cenário completo: árvore com `na_arvore`, pack com `no_pack`, e manifesto.
    /// Devolve `(packs_dir, entity)`.
    fn cenario(tag: &str, na_arvore: &[&str], no_pack: &[&str]) -> (PathBuf, String) {
        let base = std::env::temp_dir().join(format!("auli-remover-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let docs = base.join("docs").join("pareceres");
        let packs = base.join("packs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::create_dir_all(&packs).unwrap();

        let doc_de = |numero: &str| -> Documento {
            let header = mddoc::cabecalho_jurisprudencia(
                numero,
                &format!("assunto de {numero}"),
                &format!("http://x/{numero}"),
            );
            crate::update::documento_de(
                Kind::Pareceres,
                header,
                "resumo".into(),
                format!("corpo de {numero}"),
            )
        };

        for numero in na_arvore {
            escrever_md(&docs, numero);
        }

        let records: Vec<Record<String>> = no_pack
            .iter()
            .map(|n| {
                let d = doc_de(n);
                Record {
                    id: d.doc_path(),
                    embedding: vec![1.0; 2],
                    payload: d.stored_repr(),
                    key_hash: Some("h".into()),
                }
            })
            .collect();
        let total = records.len();
        vector_store::write_collection_file(
            packs.join("ent-pareceres.json"),
            &CollectionData { records },
        )
        .unwrap();

        let m = Manifest {
            entity: "ent".into(),
            version: "1".into(),
            built_at: "now".into(),
            embed_model_id: manifest::EMBED_MODEL_ID.into(),
            embed_dim: EMBED_DIM,
            strategy_version: manifest::STRATEGY_VERSION,
            pack_format: manifest::PACK_FORMAT,
            collections: vec![CollectionEntry {
                kind: "pareceres".into(),
                count: total,
                dim: EMBED_DIM,
                file: "ent-pareceres.json".into(),
                bytes: 0,
                hash: "0".into(),
            }],
            docs_hashes: None,
        };
        manifest::write_manifest(manifest::manifest_path(&packs, "ent"), &m).unwrap();
        (packs, "ent".to_string())
    }

    /// Escreve na árvore o `.md` de um número de documento, como o scraper o materializaria.
    fn escrever_md(docs: &Path, numero: &str) {
        let header = mddoc::cabecalho_jurisprudencia(
            numero,
            &format!("assunto de {numero}"),
            &format!("http://x/{numero}"),
        );
        std::fs::write(
            docs.join(format!("{}.md", mddoc::slug(numero))),
            mddoc::render_doc(&header, Some("resumo"), &format!("corpo de {numero}")),
        )
        .unwrap();
    }

    /// Carimba no manifesto o mapa `docs_hashes` da árvore como ela está agora — é o que o
    /// `auli update` faz ao fim de uma rodada. Os cenários que precisam PASSAR pela guarda de
    /// entrada do `remover` chamam isto antes.
    fn carimbar_docs_hashes(packs: &Path) {
        let p = manifest::manifest_path(packs, "ent");
        let mut m = manifest::read_manifest(&p).unwrap();
        m.docs_hashes = manifest::hash_docs_map(packs.parent().unwrap().join("docs")).unwrap();
        assert!(m.docs_hashes.is_some());
        manifest::write_manifest(&p, &m).unwrap();
    }

    fn ids_do_pack(packs: &Path) -> Vec<String> {
        let d: CollectionData<String> =
            vector_store::read_collection_file(packs.join("ent-pareceres.json")).unwrap();
        d.records.into_iter().map(|r| r.id).collect()
    }

    fn log_de(packs: &Path) -> PathBuf {
        crate::update::caminho_log_remocoes(packs, "ent-pareceres")
    }

    /// Escreve um log de aprovação com os `doc_path` dados (é o que o humano deixa depois de editar).
    fn aprovar(packs: &Path, ids: &[&str]) {
        let mut t = String::from("# doc_path\ttitulo\tlink\n");
        for id in ids {
            t.push_str(&format!("{id}\tT\tL\n"));
        }
        std::fs::write(log_de(packs), t).unwrap();
    }

    #[test]
    fn remove_exatamente_os_aprovados_e_consome_o_log() {
        // No pack: A, B, C. Na árvore: só A. Órfãos: B e C. Aprovado: só B.
        let (packs, ent) = cenario("basico", &["A 1"], &["A 1", "B 2", "C 3"]);
        // Pós-D-DH-2: um manifesto de verdade sempre traz o mapa (o `auli update` o carimba),
        // e sem ele a guarda de entrada recusa antes de qualquer escrita. Este cenário é de
        // REMOÇÃO, não de migração — daí o carimbo aqui.
        carimbar_docs_hashes(&packs);
        aprovar(&packs, &["docs/pareceres/b-2.md"]);

        run_remover(ent.clone(), packs.clone()).unwrap();

        assert_eq!(
            ids_do_pack(&packs),
            vec!["docs/pareceres/a-1.md", "docs/pareceres/c-3.md"],
            "só o aprovado sai; o órfão não aprovado FICA"
        );
        // O log é CONSUMIDO. Regenerá-lo aqui com o órfão que sobrou devolveria ao arquivo a linha
        // que o humano apagou para não aprovar, e a próxima rodada a leria como aprovação.
        assert!(
            !log_de(&packs).exists(),
            "o log tem de ser consumido; quem o regenera é o `auli update`"
        );

        // D-INC-11: o manifesto foi recarimbado, com a contagem nova e o hash do arquivo real.
        let m = manifest::read_manifest(manifest::manifest_path(&packs, "ent")).unwrap();
        let c = m
            .collections
            .iter()
            .find(|c| c.kind == "pareceres")
            .unwrap();
        assert_eq!(c.count, 2);
        let bytes = std::fs::read(packs.join("ent-pareceres.json")).unwrap();
        assert_eq!(c.hash, manifest::hash_hex(&bytes));
        assert_eq!(c.bytes, bytes.len() as u64);

        let _ = std::fs::remove_dir_all(packs.parent().unwrap());
    }

    #[test]
    fn rodar_duas_vezes_nao_faz_mal() {
        let (packs, ent) = cenario("idem", &["A 1"], &["A 1", "B 2"]);
        // Pós-D-DH-2: um manifesto de verdade sempre traz o mapa (o `auli update` o carimba),
        // e sem ele a guarda de entrada recusa antes de qualquer escrita. Este cenário é de
        // REMOÇÃO, não de migração — daí o carimbo aqui.
        carimbar_docs_hashes(&packs);
        aprovar(&packs, &["docs/pareceres/b-2.md"]);

        run_remover(ent.clone(), packs.clone()).unwrap();
        let depois_da_primeira = ids_do_pack(&packs);
        // Reaprovar o MESMO id: na segunda ele já não é órfão (nem está no pack), então a
        // interseção é vazia e nada acontece.
        aprovar(&packs, &["docs/pareceres/b-2.md"]);
        run_remover(ent, packs.clone()).unwrap();

        assert_eq!(ids_do_pack(&packs), depois_da_primeira);
        assert_eq!(ids_do_pack(&packs), vec!["docs/pareceres/a-1.md"]);
        let _ = std::fs::remove_dir_all(packs.parent().unwrap());
    }

    #[test]
    fn documento_que_reapareceu_na_arvore_nao_e_removido() {
        // O caso que a interseção existe para cobrir: o log foi revisado quando B era órfão, e
        // entre a revisão e a aplicação o documento VOLTOU para a árvore.
        let (packs, ent) = cenario("voltou", &["A 1", "B 2"], &["A 1", "B 2"]);
        // Pós-D-DH-2: um manifesto de verdade sempre traz o mapa (o `auli update` o carimba),
        // e sem ele a guarda de entrada recusa antes de qualquer escrita. Este cenário é de
        // REMOÇÃO, não de migração — daí o carimbo aqui.
        carimbar_docs_hashes(&packs);
        aprovar(&packs, &["docs/pareceres/b-2.md"]);

        run_remover(ent, packs.clone()).unwrap();

        assert_eq!(
            ids_do_pack(&packs),
            vec!["docs/pareceres/a-1.md", "docs/pareceres/b-2.md"],
            "aprovado que deixou de ser órfão NÃO pode ser removido"
        );
        let _ = std::fs::remove_dir_all(packs.parent().unwrap());
    }

    #[test]
    fn sem_log_nada_acontece() {
        // O `update` só escreve log quando há órfão; sem ele, o subcomando é inócuo.
        let (packs, ent) = cenario("semlog", &["A 1"], &["A 1", "B 2"]);
        // Pós-D-DH-2: um manifesto de verdade sempre traz o mapa (o `auli update` o carimba),
        // e sem ele a guarda de entrada recusa antes de qualquer escrita. Este cenário é de
        // REMOÇÃO, não de migração — daí o carimbo aqui.
        carimbar_docs_hashes(&packs);
        assert!(!log_de(&packs).exists());
        run_remover(ent, packs.clone()).unwrap();
        assert_eq!(ids_do_pack(&packs).len(), 2, "sem aprovação, nada sai");
        let _ = std::fs::remove_dir_all(packs.parent().unwrap());
    }

    #[test]
    fn recusa_quando_a_arvore_esta_a_frente_do_pack() {
        // O cenário que o mapa existe para pegar: o pack está carimbado com a árvore de
        // ontem, o scraper trouxe um documento novo, e o boot HOJE recusaria por hash divergente.
        // Se o `remover` rodasse e recarimbasse, o boot passaria a aceitar um pack que não tem esse
        // documento — a proteção apagada justamente pelo subcomando que só devia apagar registro.
        let (packs, ent) = cenario("frente", &["A 1"], &["A 1", "B 2"]);
        carimbar_docs_hashes(&packs);
        aprovar(&packs, &["docs/pareceres/b-2.md"]);
        escrever_md(
            &packs.parent().unwrap().join("docs").join("pareceres"),
            "C 3",
        );

        let pack_antes = std::fs::read(packs.join("ent-pareceres.json")).unwrap();
        let manifesto_antes = std::fs::read(manifest::manifest_path(&packs, "ent")).unwrap();

        let e = run_remover(ent, packs.clone()).unwrap_err().to_string();
        assert!(e.contains("à frente do pack"), "erro: {e}");
        assert!(
            e.contains("auli update"),
            "o erro tem de trazer o remédio: {e}"
        );

        assert_eq!(
            std::fs::read(packs.join("ent-pareceres.json")).unwrap(),
            pack_antes,
            "a recusa vem ANTES de escrever o pack"
        );
        assert_eq!(
            std::fs::read(manifest::manifest_path(&packs, "ent")).unwrap(),
            manifesto_antes,
            "a recusa vem ANTES de escrever o manifesto"
        );
        assert!(
            log_de(&packs).exists(),
            "rodada recusada não consome o log — a aprovação continua valendo depois do `update`"
        );
        let _ = std::fs::remove_dir_all(packs.parent().unwrap());
    }

    #[test]
    fn remover_recusa_manifesto_sem_mapa_com_arvore_em_disco() {
        // D-DH-2 visto do `remover`: o manifesto anterior à migração não passa da guarda — antes
        // do mapa, este cenário só era defendido pela preservação (ver nota histórica na guarda).
        // Verificação por mutação: devolver `Ok` no braço (None, Some) do `validate_docs_hashes`
        // mata este teste.
        let (packs, ent) = cenario("sem-mapa", &["A 1"], &["A 1", "B 2"]);
        aprovar(&packs, &["docs/pareceres/b-2.md"]);
        let mpath = manifest::manifest_path(&packs, "ent");
        assert_eq!(manifest::read_manifest(&mpath).unwrap().docs_hashes, None);

        let e = run_remover(ent, packs.clone()).unwrap_err().to_string();
        assert!(e.contains("docs_hashes"), "erro: {e}");
        assert!(e.contains("auli update"), "o remédio está na mensagem: {e}");
        assert_eq!(
            ids_do_pack(&packs),
            vec!["docs/pareceres/a-1.md", "docs/pareceres/b-2.md"],
            "recusou ANTES de escrever pack"
        );
        let _ = std::fs::remove_dir_all(packs.parent().unwrap());
    }

    #[test]
    fn mapa_e_escrito_de_volta_intacto_apos_remocao() {
        // Round-trip: depois de uma remoção aplicada com sucesso, o mapa no manifesto é byte a
        // byte o que foi lido — o `remover` não recarimba, porque não toca a árvore.
        let (packs, ent) = cenario("mapa-roundtrip", &["A 1"], &["A 1", "B 2"]);
        carimbar_docs_hashes(&packs);
        aprovar(&packs, &["docs/pareceres/b-2.md"]);
        let mpath = manifest::manifest_path(&packs, "ent");
        let antes = manifest::read_manifest(&mpath).unwrap().docs_hashes;
        assert!(antes.is_some(), "o cenário precisa do mapa carimbado");

        run_remover(ent, packs.clone()).unwrap();

        assert_eq!(
            ids_do_pack(&packs),
            vec!["docs/pareceres/a-1.md"],
            "a rodada não foi inócua"
        );
        assert_eq!(manifest::read_manifest(&mpath).unwrap().docs_hashes, antes);
        let _ = std::fs::remove_dir_all(packs.parent().unwrap());
    }

    #[test]
    fn o_log_e_lido_pelo_primeiro_campo_e_ignora_comentario() {
        let dir = std::env::temp_dir().join(format!("auli-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("l.tsv");
        std::fs::write(
            &p,
            "# cabeçalho\n\ndocs/x/a.md\tTítulo com espaço\thttp://l\ndocs/x/b.md\n",
        )
        .unwrap();
        let ids = ler_aprovados(&p).unwrap().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("docs/x/a.md") && ids.contains("docs/x/b.md"));
        // Linha sem as colunas do humano continua válida: a chave é o que o programa precisa.
        assert!(
            ler_aprovados(&dir.join("nao-existe.tsv"))
                .unwrap()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
