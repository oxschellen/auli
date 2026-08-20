//! `auli bundle` — empacota, por estado, as árvores `.md` que o pipeline gera em
//! `data/<id>/docs/<tipo>/` num `<id>.zip`, para quem quiser apontar a própria IA/RAG para as
//! pastas em vez de usar o chat.
//!
//! Os zips saem em `auli-frontend/public/downloads/` e sobem **no deploy do frontend**: o Vite copia
//! `public/` inteiro para `dist/`, então não existe caminho de publicação separado. O preço disso é
//! que o upload não é incremental — todo deploy reenvia o conjunto (hoje ~70 MB, dos quais o
//! `sp.zip` sozinho é 51 MB).
//!
//! **Determinismo é requisito, não acabamento.** mtime carimbado, compressão fixa e ordem estável
//! das entradas: duas execuções têm de produzir bytes idênticos. Sem isso o `sha256` do manifesto
//! vira ruído, e um deploy que não mudou conteúdo nenhum parece ter mudado tudo.
//!
//! O mtime das entradas **não** é a data real dos arquivos, e também não é uma constante fictícia:
//! é a data em que o conteúdo daquele estado mudou pela última vez. A data real não serve porque o
//! `auli-collections process` reescreve a árvore inteira a cada execução — hoje os 1.945 `.md` do
//! RS têm o mesmo mtime, embora 1.893 deles não tenham mudado uma vírgula. Usá-la faria o zip
//! mudar de bytes por causa de metadado, e o `sha256` deixaria de significar "o conteúdo mudou".
//!
//! Em vez disso, cada estado carrega no manifesto um `conteudo_sha256` — hash só de (caminho,
//! conteúdo), imune a mtime — e um `atualizado_em`. Quando o hash bate com o da execução anterior,
//! a data é reaproveitada; quando não bate, vira hoje. O determinismo se mantém porque a data só
//! anda quando o conteúdo anda, e os arquivos extraídos passam a dizer algo verdadeiro.
//!
//! A lista de estados vem do `registry.toml` — a MESMA fonte da verdade do server, do collections e
//! do frontend —, e não de uma varredura de `data/`. Um `data/<id>/` órfão no disco não vira
//! download; uma entidade no registry sem árvore `docs/` é simplesmente pulada.

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use auli_contract::Kind;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::error::Result;

/// Nível do deflate. Fixo por contrato: mudar aqui muda todos os `sha256` do manifesto de uma vez.
const NIVEL_COMPRESSAO: i64 = 6;

/// Formato do `atualizado_em` no manifesto e da data carimbada nas entradas do zip.
const FORMATO_DATA: &str = "%Y-%m-%d";

/// Permissão gravada em toda entrada. Fixa para o zip não herdar o umask de quem gerou.
const PERMISSAO: u32 = 0o644;

/// Um tipo de documento dentro de um estado (`servicos`, `faqs`, `pareceres`, …). `dir` é a raiz a
/// partir da qual o caminho dentro do zip é calculado — é ela que faz o nível `docs/` sumir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipoBundle {
    pub kind: String,
    pub dir: PathBuf,
    pub arquivos: Vec<PathBuf>,
}

/// Um estado com pelo menos um tipo que tem `.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstadoBundle {
    pub id: String,
    pub name: String,
    pub uf: String,
    pub state: String,
    pub tipos: Vec<TipoBundle>,
}

/// Uma linha do `downloads.json`.
#[derive(Debug, Clone, Serialize)]
pub struct EntradaManifesto {
    pub estado: String,
    pub uf: String,
    pub nome: String,
    pub arquivo: String,
    pub tamanho_bytes: u64,
    /// O mesmo tamanho já legível ("51,2 MB"), para a página de download não ter de formatar —
    /// e para ninguém baixar 51 MB sem saber.
    pub tamanho: String,
    pub sha256: String,
    /// Hash só de (caminho, conteúdo) das fontes — imune a mtime. É ele que decide se o
    /// `atualizado_em` anda; o `sha256` acima não serve, porque muda também quando muda a
    /// embalagem (README, nível de compressão).
    pub conteudo_sha256: String,
    /// Data em que o conteúdo deste estado mudou pela última vez (`YYYY-MM-DD`). É também o mtime
    /// carimbado em toda entrada do zip.
    pub atualizado_em: String,
    /// Quando este zip foi *gerado* — anda a cada execução, ao contrário do `atualizado_em`.
    pub gerado_em: String,
    pub tipos: Vec<ContagemTipo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContagemTipo {
    pub tipo: String,
    pub arquivos: usize,
}

/// Só os campos que o bundle usa; serde ignora o resto do registry (prompt, collections…).
#[derive(Deserialize)]
struct RegistroRaiz {
    #[serde(default)]
    entities: Vec<RegistroEntidade>,
}

#[derive(Deserialize)]
struct RegistroEntidade {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    uf: String,
    #[serde(default)]
    state: String,
}

/// Percorre o registry e devolve os estados que têm conteúdo, ordenados por `id`, com os arquivos
/// de cada tipo ordenados por caminho. **A ordenação aqui é a pré-condição do determinismo do zip**
/// — `read_dir` não promete ordem nenhuma.
pub fn descobrir(data_root: &Path) -> Result<Vec<EstadoBundle>> {
    let caminho = auli_contract::config::registry_path(data_root);
    let texto = fs::read_to_string(&caminho)
        .map_err(|e| format!("não consegui ler {}: {e}", caminho.display()))?;
    let registro: RegistroRaiz =
        toml::from_str(&texto).map_err(|e| format!("{} inválido: {e}", caminho.display()))?;

    let mut estados = Vec::new();
    for ent in registro.entities {
        let docs = data_root.join(&ent.id).join("docs");
        if !docs.is_dir() {
            continue;
        }

        let mut subpastas: Vec<PathBuf> = fs::read_dir(&docs)
            .map_err(|e| format!("não consegui ler {}: {e}", docs.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        subpastas.sort();

        let mut tipos = Vec::new();
        for dir in subpastas {
            let kind = dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if kind_segurado(&kind) {
                println!(
                    "⏸️  {}/{kind}: segurado fora do zip (coleta incompleta).",
                    ent.id
                );
                continue;
            }
            let mut arquivos = Vec::new();
            coletar_md(&dir, &mut arquivos)?;
            if arquivos.is_empty() {
                continue;
            }
            arquivos.sort();
            tipos.push(TipoBundle {
                kind,
                dir,
                arquivos,
            });
        }

        if tipos.is_empty() {
            continue;
        }
        estados.push(EstadoBundle {
            id: ent.id,
            name: ent.name,
            uf: ent.uf,
            state: ent.state,
            tipos,
        });
    }

    estados.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(estados)
}

/// Coleta os `.md` sob `dir`, recursivamente. Hoje as árvores são planas; a recursão existe para o
/// dia em que deixarem de ser, e não para ser exercitada agora.
fn coletar_md(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entrada in
        fs::read_dir(dir).map_err(|e| format!("não consegui ler {}: {e}", dir.display()))?
    {
        let caminho = entrada
            .map_err(|e| format!("entrada ilegível em {}: {e}", dir.display()))?
            .path();
        if caminho.is_dir() {
            coletar_md(&caminho, out)?;
        } else if caminho.extension().is_some_and(|e| e == "md") {
            out.push(caminho);
        }
    }
    Ok(())
}

/// Hash das FONTES de um estado: (caminho dentro do zip, conteúdo), na ordem da descoberta. Não
/// entra mtime, nem os README gerados, nem nada da embalagem — é o que permite responder "o acervo
/// mudou?" sem confundir com "o pacote mudou?".
pub fn hash_fontes(estado: &EstadoBundle) -> Result<String> {
    let mut hasher = Sha256::new();
    for tipo in &estado.tipos {
        for arquivo in &tipo.arquivos {
            let relativo = arquivo.strip_prefix(&tipo.dir).unwrap_or(arquivo);
            hasher.update(tipo.kind.as_bytes());
            hasher.update(b"/");
            hasher.update(relativo.to_string_lossy().replace('\\', "/").as_bytes());
            hasher.update(b"\0");
            let conteudo = fs::read(arquivo)
                .map_err(|e| format!("não consegui ler {}: {e}", arquivo.display()))?;
            hasher.update(&conteudo);
            hasher.update(b"\0");
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// A data `YYYY-MM-DD` como `zip::DateTime` (meia-noite), para carimbar as entradas.
fn data_do_zip(iso: &str) -> Result<DateTime> {
    let d = chrono::NaiveDate::parse_from_str(iso, FORMATO_DATA)
        .map_err(|e| format!("data inválida {iso:?} (esperado YYYY-MM-DD): {e}"))?;
    use chrono::Datelike;
    DateTime::from_date_and_time(d.year() as u16, d.month() as u8, d.day() as u8, 0, 0, 0)
        .map_err(|e| format!("data {iso:?} fora do que o formato ZIP representa: {e:?}").into())
}

/// Monta o `.zip` de um estado **em memória** e devolve os bytes — é essa forma que deixa o teste de
/// determinismo comparar duas execuções sem passar por disco. `data` é carimbada em toda entrada.
pub fn gerar_zip(estado: &EstadoBundle, data: DateTime) -> Result<Vec<u8>> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let opcoes = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(NIVEL_COMPRESSAO))
        .last_modified_time(data)
        .unix_permissions(PERMISSAO);

    escrever(
        &mut writer,
        "README.md",
        readme_estado(estado).as_bytes(),
        opcoes,
    )?;

    for tipo in &estado.tipos {
        escrever(
            &mut writer,
            &format!("{}/README.md", tipo.kind),
            readme_tipo(estado, tipo).as_bytes(),
            opcoes,
        )?;
        for arquivo in &tipo.arquivos {
            let relativo = arquivo.strip_prefix(&tipo.dir).unwrap_or(arquivo);
            let nome = format!(
                "{}/{}",
                tipo.kind,
                relativo.to_string_lossy().replace('\\', "/")
            );
            let conteudo = fs::read(arquivo)
                .map_err(|e| format!("não consegui ler {}: {e}", arquivo.display()))?;
            escrever(&mut writer, &nome, &conteudo, opcoes)?;
        }
    }

    let cursor = writer
        .finish()
        .map_err(|e| format!("fechando o zip de '{}': {e}", estado.id))?;
    Ok(cursor.into_inner())
}

fn escrever(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    nome: &str,
    conteudo: &[u8],
    opcoes: SimpleFileOptions,
) -> Result<()> {
    writer
        .start_file(nome, opcoes)
        .map_err(|e| format!("abrindo '{nome}' no zip: {e}"))?;
    writer
        .write_all(conteudo)
        .map_err(|e| format!("gravando '{nome}' no zip: {e}"))?;
    Ok(())
}

/// Rótulo de exibição do tipo. Tipo desconhecido cai no próprio nome — a rotina não hardcoda a
/// lista de coleções, só sabe apresentar melhor as que existem hoje.
/// Coleções que existem na árvore mas **não entram nos zips públicos**, por estarem incompletas.
///
/// Um zip é material de download com o site apontando para ele: publicar um recorte parcial sob o
/// rótulo da coleção o apresenta como se fosse o acervo. Enquanto a coleta não fecha, é melhor não
/// publicar do que publicar pela metade sem o leitor ter como saber.
///
/// **Vazia desde 08/08/2026.** O `tarf` esteve aqui enquanto a coleta era pausada para a migração
/// de formato; a decisão passou a ser publicar o que já existe e seguir coletando, com o README da
/// pasta dizendo que a coleta estava em andamento — o leitor ficava sabendo, que era o ponto. A
/// coleta fechou em 09/08/2026 (22.476 acórdãos) e o aviso saiu junto: agora a contagem É o acervo.
///
/// A lista filtra o BUNDLE, não o dado: a árvore segue no disco e dentro do mapa `docs_hashes` do
/// manifesto. Segurar uma coleção movendo o diretório mudaria esse hash e faria o servidor recusar o
/// boot — por isso o filtro mora aqui.
const KINDS_SEGURADOS: &[&str] = &[];

fn kind_segurado(kind: &str) -> bool {
    KINDS_SEGURADOS.contains(&kind)
}

/// Rótulo de exibição da pasta. As pastas do bundle vêm do **disco** (`read_dir` sobre `docs/`),
/// não do enum — daí o `&str` na entrada e o fallback: um diretório que não seja coleção conhecida
/// aparece pelo próprio nome, em vez de sumir do README.
fn titulo_tipo(kind: &str) -> String {
    Kind::parse(kind).map_or_else(|| kind.to_string(), |k| k.titulo().to_string())
}

/// O schema real de cada tipo, tirado do módulo `mddoc` do contrato — hoje um só, para todas as
/// coleções. É o que permite a quem baixou escrever o próprio parser sem abrir 500 arquivos.
///
/// Todas as coleções compartilham o mesmo frontmatter desde o formato v2 (ago/2026) — o que muda
/// entre elas é o que cada campo carrega, e é isso que estes textos explicam. Campo que não se
/// aplica vem presente e vazio.
fn schema_tipo(kind: &str) -> &'static str {
    const COMUM: &str = "Frontmatter YAML entre `---`, com os campos `titulo`, `trilha`, `ementa` e \
                         `link`, seguido do texto sob `## corpo`.";
    let Some(kind) = Kind::parse(kind) else {
        return COMUM;
    };
    match kind {
        Kind::Servicos => {
            "Frontmatter YAML entre `---`, com os campos `titulo`, `trilha` (o público e a seção\n\
             do portal, no formato `Público | Seção`), `ementa` (vazia nos serviços), `link` e\n\
             `orgao`. A descrição do serviço vem depois, sob `## corpo`."
        }
        Kind::Faqs => {
            "Frontmatter YAML entre `---`, com os campos `titulo` (a pergunta), `trilha` (a trilha\n\
             de navegação no portal, do tipo `Inicial | Tema | Subtema`), `ementa` (vazia nas\n\
             perguntas) e `link`. A resposta é o corpo do arquivo, sob `## corpo`."
        }
        Kind::Legislacao => {
            "Frontmatter YAML entre `---`, com os campos `titulo` (a pergunta), `trilha` (a\n\
             hierarquia da lei, do tipo `Lei | Título | Capítulo`, cujo primeiro segmento é\n\
             também o nome da subpasta), `ementa` (o dispositivo — ex.: `Art. 21, I`) e `link`\n\
             (a página oficial da lei). A resposta é o corpo do arquivo, sob `## corpo`.\n\
             Esta é a única coleção cuja pasta tem um nível a mais: uma subpasta por lei."
        }
        Kind::Pareceres | Kind::Tarf => {
            "Frontmatter YAML entre `---`, com os campos `titulo` (o número do documento),\n\
             `trilha` (vazia aqui), `ementa` (o assunto) e `link`. O corpo traz duas seções:\n\
             `## resumo`, uma síntese curta, e `## corpo`, o texto integral."
        }
    }
}

fn readme_tipo(estado: &EstadoBundle, tipo: &TipoBundle) -> String {
    format!(
        "# {titulo} — {uf}\n\
         \n\
         {n} documentos, um por arquivo `.md`. Dados públicos da {nome}; cada arquivo traz no\n\
         frontmatter o link para a fonte oficial.\n\
         \n\
         ## Como usar com sua IA\n\
         \n\
         Aponte seu assistente / RAG para **esta pasta**. Cada `.md` é um documento independente —\n\
         mantenha os arquivos separados (não concatene) para o seu mecanismo chunkar por documento.\n\
         \n\
         ## Estrutura de cada arquivo\n\
         \n\
         {schema}\n\
         \n\
         Formato v2 (ago/2026): todas as coleções compartilham o mesmo frontmatter. A versão\n\
         anterior usava chaves próprias por coleção — ver histórico do repositório.\n\
         \n\
         ## Origem\n\
         \n\
         Conteúdo derivado de dados públicos da {nome}. Os resumos, quando presentes, são gerados\n\
         automaticamente — a fonte oficial é o link no frontmatter.\n",
        titulo = titulo_tipo(&tipo.kind),
        uf = estado.uf,
        n = tipo.arquivos.len(),
        nome = estado.name,
        schema = schema_tipo(&tipo.kind),
    )
}

fn readme_estado(estado: &EstadoBundle) -> String {
    let mut pastas = String::new();
    for tipo in &estado.tipos {
        pastas.push_str(&format!(
            "- `{}/` — {} documentos ({})\n",
            tipo.kind,
            tipo.arquivos.len(),
            titulo_tipo(&tipo.kind)
        ));
    }
    format!(
        "# {nome} — {state}\n\
         \n\
         Acervo em Markdown do portal da {nome}, um documento por arquivo.\n\
         \n\
         ## O que tem aqui\n\
         \n\
         {pastas}\n\
         Cada pasta tem o próprio `README.md` com o schema dos arquivos e como apontar sua IA\n\
         para ela.\n\
         \n\
         ## Origem\n\
         \n\
         Conteúdo derivado de dados públicos da {nome}. Cada arquivo traz no frontmatter o link\n\
         para a fonte oficial, que prevalece sobre esta cópia.\n",
        nome = estado.name,
        state = estado.state,
        pastas = pastas,
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Tamanho legível em pt-BR (vírgula decimal), para o manifesto poder ser exibido cru.
pub fn formatar_tamanho(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB).replace('.', ",")
    } else {
        format!("{:.1} MB", b / MB).replace('.', ",")
    }
}

/// O que a execução anterior registrou, para decidir se a data do conteúdo anda ou fica.
#[derive(Deserialize)]
struct EntradaAnterior {
    estado: String,
    #[serde(default)]
    conteudo_sha256: String,
    #[serde(default)]
    atualizado_em: String,
}

/// Lê o `downloads.json` da execução anterior. Ausente ou ilegível não é erro — significa apenas
/// que não há histórico e todas as datas serão de hoje.
fn manifesto_anterior(caminho: &Path) -> Vec<EntradaAnterior> {
    fs::read_to_string(caminho)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Gera `<out>/<id>.zip` para todo estado com conteúdo, mais o `downloads.json`.
pub fn run_bundle(data_root: PathBuf, out: PathBuf) -> Result<()> {
    let estados = descobrir(&data_root)?;
    if estados.is_empty() {
        return Err(format!(
            "nenhuma entidade do registry tem árvore `docs/` com `.md` sob {}",
            data_root.display()
        )
        .into());
    }

    fs::create_dir_all(&out)?;
    let caminho_manifesto = out.join("downloads.json");
    let anteriores = manifesto_anterior(&caminho_manifesto);
    let agora = chrono::Utc::now();
    let gerado_em = agora.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let hoje = agora.format(FORMATO_DATA).to_string();

    let mut manifesto = Vec::with_capacity(estados.len());
    let mut total = 0u64;
    for estado in &estados {
        // A data só anda quando as FONTES andam — daí o hash próprio, imune a mtime e à embalagem.
        let conteudo_sha256 = hash_fontes(estado)?;
        let atualizado_em = anteriores
            .iter()
            .find(|a| a.estado == estado.id)
            .filter(|a| a.conteudo_sha256 == conteudo_sha256 && !a.atualizado_em.is_empty())
            .map(|a| a.atualizado_em.clone())
            .unwrap_or_else(|| hoje.clone());

        let bytes = gerar_zip(estado, data_do_zip(&atualizado_em)?)?;
        let sha256 = sha256_hex(&bytes);
        let arquivo = format!("{}.zip", estado.id);
        let destino = out.join(&arquivo);

        // O zip é determinístico, então um conteúdo igual não precisa ser reescrito. Poupa o disco
        // e, sobretudo, mantém o mtime estável — o upload é que não tem como ser incremental.
        let inalterado = fs::read(&destino)
            .map(|antigo| sha256_hex(&antigo) == sha256)
            .unwrap_or(false);
        if !inalterado {
            fs::write(&destino, &bytes)?;
        }

        let tamanho_bytes = bytes.len() as u64;
        total += tamanho_bytes;
        println!(
            "  {} {:>10}  {}  {}{}",
            arquivo,
            formatar_tamanho(tamanho_bytes),
            atualizado_em,
            estado
                .tipos
                .iter()
                .map(|t| format!("{}={}", t.kind, t.arquivos.len()))
                .collect::<Vec<_>>()
                .join(" "),
            if inalterado { "  (inalterado)" } else { "" },
        );

        manifesto.push(EntradaManifesto {
            estado: estado.id.clone(),
            uf: estado.uf.clone(),
            nome: estado.name.clone(),
            arquivo,
            tamanho_bytes,
            tamanho: formatar_tamanho(tamanho_bytes),
            sha256,
            conteudo_sha256,
            atualizado_em,
            gerado_em: gerado_em.clone(),
            tipos: estado
                .tipos
                .iter()
                .map(|t| ContagemTipo {
                    tipo: t.kind.clone(),
                    arquivos: t.arquivos.len(),
                })
                .collect(),
        });
    }

    let json = serde_json::to_string_pretty(&manifesto)?;
    fs::write(&caminho_manifesto, json)?;

    println!(
        "✅ {} zips ({}) + downloads.json em {}",
        manifesto.len(),
        formatar_tamanho(total),
        out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// Monta uma `data/` de teste espelhando o layout real e devolve a raiz. Inclui as três
    /// armadilhas do enunciado: entidade sem `docs/`, entidade fora do registry e tipo vazio.
    fn data_de_teste(tag: &str) -> PathBuf {
        // Espelha o layout REAL do repo: `config/` é irmã de `data/`, não filha. Sem os dois
        // níveis, o teste escreveria o registry no `/tmp` compartilhado — e uma execução
        // atropelaria a outra.
        let base = std::env::temp_dir().join(format!("auli-bundle-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let raiz = base.join("data");
        fs::create_dir_all(&raiz).unwrap();
        fs::create_dir_all(base.join("config")).unwrap();

        fs::write(
            auli_contract::config::registry_path(&raiz),
            r#"
[[entities]]
id = "zz"
name = "SEFAZ-ZZ"
uf = "ZZ"
state = "Estado ZZ"

[[entities]]
id = "yy"
name = "SEFAZ-YY"
uf = "YY"
state = "Estado YY"

[[entities]]
id = "ww"
name = "SEFAZ-WW"
uf = "WW"
state = "Estado WW"
"#,
        )
        .unwrap();

        let escrever_md = |uf: &str, tipo: &str, nomes: &[&str]| {
            let dir = raiz.join(uf).join("docs").join(tipo);
            fs::create_dir_all(&dir).unwrap();
            for n in nomes {
                fs::write(dir.join(format!("{n}.md")), format!("conteúdo de {n}\n")).unwrap();
            }
        };

        // zz: os três tipos, com um deles gravado fora de ordem alfabética de propósito.
        escrever_md("zz", "servicos", &["b-dois", "a-um", "c-tres"]);
        escrever_md("zz", "faqs", &["p2", "p1"]);
        escrever_md("zz", "pareceres", &["n1", "n2"]);
        // zz também tem um tipo com pasta criada e vazia — deve sumir do zip.
        fs::create_dir_all(raiz.join("zz").join("docs").join("notas")).unwrap();

        // yy: só pareceres.
        escrever_md("yy", "pareceres", &["y1", "y2"]);

        // ww: está no registry mas nunca foi coletado — só `packs/`, sem `docs/`. É o caso do `ap`.
        fs::create_dir_all(raiz.join("ww").join("packs")).unwrap();

        // xx: tem conteúdo no disco mas NÃO está no registry — não pode virar download.
        escrever_md("xx", "servicos", &["intruso"]);

        raiz
    }

    /// Data arbitrária mas fixa para os testes que não são sobre datas.
    fn data_de_teste_zip() -> DateTime {
        data_do_zip("2026-07-29").unwrap()
    }

    /// A data carimbada numa entrada do zip, como `YYYY-MM-DD`.
    fn data_da_entrada(bytes: &[u8], nome: &str) -> String {
        let mut arquivo = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
        let entrada = arquivo.by_name(nome).unwrap();
        let d = entrada.last_modified().unwrap();
        format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
    }

    fn nomes_no_zip(bytes: &[u8]) -> Vec<String> {
        let mut arquivo = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
        (0..arquivo.len())
            .map(|i| arquivo.by_index(i).unwrap().name().to_string())
            .collect()
    }

    fn conteudo_no_zip(bytes: &[u8], nome: &str) -> String {
        use std::io::Read;
        let mut arquivo = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
        let mut entrada = arquivo.by_name(nome).unwrap();
        let mut texto = String::new();
        entrada.read_to_string(&mut texto).unwrap();
        texto
    }

    #[test]
    fn descobre_so_o_que_o_registry_lista_e_tem_md() {
        let raiz = data_de_teste("descoberta");
        let estados = descobrir(&raiz).unwrap();

        let ids: Vec<&str> = estados.iter().map(|e| e.id.as_str()).collect();
        // `ww` sai por não ter docs/, `xx` por não estar no registry; a ordem é por id.
        assert_eq!(ids, vec!["yy", "zz"]);

        let zz = estados.iter().find(|e| e.id == "zz").unwrap();
        let tipos: Vec<&str> = zz.tipos.iter().map(|t| t.kind.as_str()).collect();
        // `notas` existe como pasta mas está vazia — não vira tipo.
        assert_eq!(tipos, vec!["faqs", "pareceres", "servicos"]);

        let servicos = zz.tipos.iter().find(|t| t.kind == "servicos").unwrap();
        let nomes: Vec<String> = servicos
            .arquivos
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(nomes, vec!["a-um.md", "b-dois.md", "c-tres.md"]);
    }

    #[test]
    fn zip_tem_readmes_e_descarta_o_nivel_docs() {
        let raiz = data_de_teste("estrutura");
        let estados = descobrir(&raiz).unwrap();
        let zz = estados.iter().find(|e| e.id == "zz").unwrap();
        let nomes = nomes_no_zip(&gerar_zip(zz, data_de_teste_zip()).unwrap());

        assert!(nomes.contains(&"README.md".to_string()));
        for tipo in ["servicos", "faqs", "pareceres"] {
            assert!(nomes.contains(&format!("{tipo}/README.md")));
        }
        assert!(nomes.contains(&"servicos/a-um.md".to_string()));
        // O nível `docs/` não aparece em lugar nenhum do zip.
        assert!(!nomes.iter().any(|n| n.contains("docs/")));
        // E o tipo de pasta vazia não virou pasta.
        assert!(!nomes.iter().any(|n| n.starts_with("notas/")));
    }

    #[test]
    fn estado_com_um_tipo_so_leva_so_aquele_tipo() {
        let raiz = data_de_teste("um-tipo");
        let estados = descobrir(&raiz).unwrap();
        let yy = estados.iter().find(|e| e.id == "yy").unwrap();
        let nomes = nomes_no_zip(&gerar_zip(yy, data_de_teste_zip()).unwrap());

        assert_eq!(
            nomes,
            vec![
                "README.md",
                "pareceres/README.md",
                "pareceres/y1.md",
                "pareceres/y2.md",
            ]
        );
    }

    #[test]
    fn md_no_zip_e_byte_identico_a_entrada() {
        let raiz = data_de_teste("verbatim");
        let estados = descobrir(&raiz).unwrap();
        let zz = estados.iter().find(|e| e.id == "zz").unwrap();
        let bytes = gerar_zip(zz, data_de_teste_zip()).unwrap();

        let original = fs::read_to_string(raiz.join("zz/docs/servicos/a-um.md")).unwrap();
        assert_eq!(conteudo_no_zip(&bytes, "servicos/a-um.md"), original);
    }

    #[test]
    fn geracao_e_deterministica() {
        let raiz = data_de_teste("determinismo");
        let estados = descobrir(&raiz).unwrap();

        // Duas descobertas independentes, para o teste também cobrir a ordem vinda do `read_dir`.
        let primeira: Vec<Vec<u8>> = estados
            .iter()
            .map(|e| gerar_zip(e, data_de_teste_zip()).unwrap())
            .collect();
        let segunda: Vec<Vec<u8>> = descobrir(&raiz)
            .unwrap()
            .iter()
            .map(|e| gerar_zip(e, data_de_teste_zip()).unwrap())
            .collect();

        for (a, b) in primeira.iter().zip(segunda.iter()) {
            assert_eq!(sha256_hex(a), sha256_hex(b), "zip deve ser byte-idêntico");
        }
    }

    #[test]
    fn manifesto_bate_com_os_arquivos_gerados() {
        let raiz = data_de_teste("manifesto");
        let out = raiz.join("downloads");
        run_bundle(raiz.clone(), out.clone()).unwrap();

        let texto = fs::read_to_string(out.join("downloads.json")).unwrap();
        let entradas: Vec<serde_json::Value> = serde_json::from_str(&texto).unwrap();

        let ids: Vec<&str> = entradas
            .iter()
            .map(|e| e["estado"].as_str().unwrap())
            .collect();
        assert_eq!(ids, vec!["yy", "zz"], "manifesto ordenado por estado");

        for entrada in &entradas {
            let nome = entrada["arquivo"].as_str().unwrap();
            let bytes = fs::read(out.join(nome)).unwrap();
            assert_eq!(
                entrada["sha256"].as_str().unwrap(),
                sha256_hex(&bytes),
                "sha256 do manifesto tem de conferir com o arquivo em disco"
            );
            assert_eq!(
                entrada["tamanho_bytes"].as_u64().unwrap(),
                bytes.len() as u64
            );
            assert!(entrada["tamanho"].as_str().unwrap().ends_with("B"));
        }

        let zz = entradas.iter().find(|e| e["estado"] == "zz").unwrap();
        let contagens: BTreeMap<String, u64> = zz["tipos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                (
                    t["tipo"].as_str().unwrap().to_string(),
                    t["arquivos"].as_u64().unwrap(),
                )
            })
            .collect();
        assert_eq!(contagens["servicos"], 3);
        assert_eq!(contagens["faqs"], 2);
        assert_eq!(contagens["pareceres"], 2);
        assert!(!contagens.contains_key("notas"));

        // `ww` (sem docs/) e `xx` (fora do registry) não podem ter virado arquivo.
        assert!(!out.join("ww.zip").exists());
        assert!(!out.join("xx.zip").exists());
    }

    #[test]
    fn hash_das_fontes_ignora_mtime() {
        // O caso real: o `process` reescreve a árvore inteira e carimba mtime novo em todos os
        // arquivos, mesmo nos que não mudaram. O hash não pode enxergar isso.
        let raiz = data_de_teste("hash-mtime");
        let antes = hash_fontes(&descobrir(&raiz).unwrap()[0]).unwrap();

        let alvo = raiz.join("yy/docs/pareceres/y1.md");
        let conteudo = fs::read(&alvo).unwrap();
        fs::write(&alvo, &conteudo).unwrap(); // reescreve idêntico: mtime novo, conteúdo igual

        assert_eq!(antes, hash_fontes(&descobrir(&raiz).unwrap()[0]).unwrap());
    }

    #[test]
    fn hash_das_fontes_muda_com_o_conteudo() {
        let raiz = data_de_teste("hash-conteudo");
        let antes = hash_fontes(&descobrir(&raiz).unwrap()[0]).unwrap();
        fs::write(raiz.join("yy/docs/pareceres/y1.md"), "outro texto\n").unwrap();
        assert_ne!(antes, hash_fontes(&descobrir(&raiz).unwrap()[0]).unwrap());
    }

    #[test]
    fn data_do_conteudo_fica_quando_nada_muda_e_anda_quando_muda() {
        let raiz = data_de_teste("data-conteudo");
        let out = raiz.join("downloads");
        run_bundle(raiz.clone(), out.clone()).unwrap();

        // Reescreve o manifesto com uma data antiga, simulando um pacote gerado há tempos.
        let ler = |out: &PathBuf| -> Vec<serde_json::Value> {
            serde_json::from_str(&fs::read_to_string(out.join("downloads.json")).unwrap()).unwrap()
        };
        let mut entradas = ler(&out);
        for e in &mut entradas {
            e["atualizado_em"] = serde_json::json!("2020-03-15");
        }
        fs::write(
            out.join("downloads.json"),
            serde_json::to_string_pretty(&entradas).unwrap(),
        )
        .unwrap();

        // Conteúdo intocado: a data tem de ser preservada, e chegar carimbada nos arquivos.
        run_bundle(raiz.clone(), out.clone()).unwrap();
        let depois = ler(&out);
        for e in &depois {
            assert_eq!(e["atualizado_em"], "2020-03-15");
        }
        let bytes = fs::read(out.join("yy.zip")).unwrap();
        assert_eq!(data_da_entrada(&bytes, "pareceres/y1.md"), "2020-03-15");

        // Agora um documento muda: só o estado afetado ganha data nova.
        fs::write(raiz.join("yy/docs/pareceres/y1.md"), "texto novo\n").unwrap();
        run_bundle(raiz.clone(), out.clone()).unwrap();
        let final_ = ler(&out);
        let yy = final_.iter().find(|e| e["estado"] == "yy").unwrap();
        let zz = final_.iter().find(|e| e["estado"] == "zz").unwrap();
        assert_ne!(yy["atualizado_em"], "2020-03-15", "yy mudou, data anda");
        assert_eq!(zz["atualizado_em"], "2020-03-15", "zz não mudou, data fica");
    }

    #[test]
    fn tamanho_formatado_em_pt_br() {
        assert_eq!(formatar_tamanho(512), "512 B");
        assert_eq!(formatar_tamanho(2048), "2,0 KB");
        assert_eq!(formatar_tamanho(53_477_376), "51,0 MB");
    }

    #[test]
    fn nenhuma_colecao_esta_segurada_hoje() {
        // A lista está vazia desde 08/08/2026 (ver `KINDS_SEGURADOS`). O teste existe para o dia em
        // que alguém puser algo lá: segurar uma coleção é decisão de publicação, não detalhe de
        // implementação, e some do zip sem deixar rastro se ninguém estiver olhando.
        for k in ["servicos", "faqs", "pareceres", "tarf"] {
            assert!(!kind_segurado(k), "{k} deixou de ser publicado sem aviso");
        }
    }

    #[test]
    fn todo_kind_publicado_tem_titulo_e_schema_humanos() {
        // Guarda contra o defeito real de 08/08/2026: o `tarf` entrou nos zips sem caso em
        // `titulo_tipo`, e o README saiu com "tarf/ — 2600 documentos (tarf)". Kind novo tem que
        // ganhar rótulo e schema ANTES de ser publicado.
        for k in ["servicos", "faqs", "pareceres", "tarf"] {
            assert_ne!(titulo_tipo(k), k, "`{k}` caiu no rótulo cru");
            assert!(
                schema_tipo(k).contains("titulo"),
                "`{k}` sem schema próprio no README"
            );
        }
    }
}
