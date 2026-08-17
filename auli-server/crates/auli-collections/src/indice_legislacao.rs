//! Subcomando `indice legislacao` — deriva o índice leve da coleção de **legislação**.
//!
//! Irmão do [`crate::indice`], separado por três diferenças que não cabiam num braço: a árvore tem
//! **subpastas** (uma por lei), o shape da entrada é outro (D-LEG-11) e a ordem é a do TEXTO LEGAL,
//! não a cronológica.
//!
//! **Shape novo, com nomes limpos** (D-LEG-11): `[{titulo, trilha, ementa, corpo, link}]`. O índice
//! de jurisprudência carrega `numero`/`assunto` por compatibilidade com uma tab que já existia; aqui
//! não há contrato legado a preservar, então os campos usam o vocabulário unificado. Não há
//! `resumo`: legislação não tem `## resumo` — a resposta É o `corpo`.
//!
//! **O `corpo` entra no índice** (D-LEG-11a), ao contrário do que acontece nas coleções de
//! jurisprudência. Lá ele fica de fora porque é a íntegra de um acórdão ou parecer: os 176 MB que já
//! foram servidos em `public/` uma vez. Aqui o corpo é a RESPOSTA de um par P/R — medidos, 187 corpos
//! somam 73 KB, média de 398 caracteres. Deixá-lo fora obrigaria a aba a mandar o usuário ao portal
//! para ler duas linhas que já estão prontas.
//!
//! **Ordem: por lei, e dentro da lei pelo dispositivo** (D-LEG-12) — a ordem em que se lê uma lei.
//! Ordenar pela trilha quebraria nos algarismos romanos ("Título IX" > "Título X" em lexicográfico);
//! o número do artigo é monotônico no texto. Ver [`chave_dispositivo`] para o que parseia e o que
//! vai para o fim do grupo.
//!
//! A chave desce até a alínea porque o artigo sozinho não basta: a entrega da Constituição são 41
//! pares TODOS do `Art. 155`, e com chave só de artigo eles empatavam em bloco e caíam na ordem de
//! caminho — que, depois do nome canônico, é alfabética pela pergunta. O grupo abria em
//! `§ 2º, IX, a` e o `caput` aparecia em 15º.

use std::path::Path;

use auli_contract::{Kind, lei_da_trilha, mddoc};
use serde::Serialize;

use crate::domain::entities::EntityConfig;
use crate::errors::Result;

/// Uma entrada do índice: o cabeçalho do `.md` menos o que só interessa ao servidor, mais o corpo.
/// É o vocabulário unificado caindo 1:1 — `titulo` é a pergunta, `ementa` é o dispositivo, `corpo`
/// é a resposta.
#[derive(Serialize)]
struct Entrada {
    titulo: String,
    trilha: String,
    ementa: String,
    corpo: String,
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
            Some(k) => (lei, 0u8, k),
            // `1` no segundo slot é o que empurra o sem-chave para o FIM DO GRUPO — do grupo da
            // própria lei, não da lista, porque a lei é o primeiro componente da chave.
            None => (lei, 1u8, (0, 0, 0, 0, 0, 0, 0)),
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

/// Chave de ordenação do dispositivo, tirada da `ementa`:
/// `(artigo, sufixo, parágrafo, sufixo do parágrafo, inciso, alínea, item)`. `None` quando não há
/// artigo reconhecível no começo — aí a entrada vai para o fim do grupo da lei, **nunca ganha uma
/// posição chutada**.
///
/// Os dois últimos níveis vieram com a LC 123/2006, que escreve o que a CF e a 6.537 não escreviam:
/// parágrafo acrescido (`§ 5º-M` — o art. 18 tem 17 deles) e item dentro da alínea (`XIII, g.1`).
///
/// O que parseia no ARTIGO, medido contra as 175 ementas reais (Lei 6.537/1973 + CF art. 155):
///
/// - **`Art. {N}`** — o caso comum: `Art. 28`, `Art. 1º`.
/// - **`Art. {N}-{L}`** — artigo acrescido por lei posterior (`Art. 17-A`, `Art. 136-F`). O sufixo
///   é o segundo componente, então `17` < `17-A` < `18`, como no texto.
/// - **`Arts. {N} …`** — o PLURAL (`Arts. 85 a 89`, `Arts. 32 e 33`): chaveia pelo PRIMEIRO artigo
///   da faixa, que está escrito ali. É leitura, não invenção.
///
/// E no SUBDISPOSITIVO, que a versão original jogava fora (ver [`subdispositivo`]): `caput`,
/// parágrafo, inciso e alínea. A regra de nível é **ausência = `caput` = menor valor**, então
/// `Art. 155` (sem nada), `Art. 155, caput` e `Art. 155 (escopo do acervo)` empatam no topo do
/// artigo, e o empate residual entre esses três é aceito de propósito: desempatá-los exigiria
/// ordenar por convenção de texto livre (o parêntese), que é frágil. O absurdo que a chave estendida
/// resolve é o `§ 2º, IX, a` abrindo o grupo.
///
/// O que NÃO parseia (e é o certo): ementa vazia, ou que comece por qualquer outra coisa — remissão
/// a outra norma, texto livre. Sem número no começo não há ordem defensável.
fn chave_dispositivo(ementa: &str) -> Option<(u32, u8, u16, u8, u16, u8, u8)> {
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
    let resto = &resto[digitos.len()..];

    // O ordinal do artigo (`Art. 1º`) é grafia, não conteúdo: sai da frente antes de procurar
    // sufixo ou subdispositivo. Sem isto, `Art. 1º, parágrafo único` não chegaria à vírgula.
    let resto = resto
        .strip_prefix('º')
        .or_else(|| resto.strip_prefix('°'))
        .unwrap_or(resto);

    // Sufixo de letra: só o `-A` colado ao número conta. `Art. 136 - A` (com espaços) não existe no
    // acervo e não é reconhecido — chave sem sufixo é a leitura conservadora.
    let (sufixo, resto) = match resto.strip_prefix('-').and_then(|r| {
        r.chars()
            .next()
            .filter(char::is_ascii_uppercase)
            .map(|c| (c, r))
    }) {
        Some((c, r)) => (c as u8, &r[c.len_utf8()..]),
        None => (0, resto),
    };

    let sub = subdispositivo(resto);
    Some((
        numero,
        sufixo,
        sub.paragrafo,
        sub.sufixo_paragrafo,
        sub.inciso,
        sub.alinea,
        sub.item,
    ))
}

/// Os cinco níveis abaixo do artigo. Tudo `0` é o nível do `caput`, e é o menor da chave.
///
/// Virou struct quando o art. 18 da LC 123 chegou: `(u16, u8, u16, u8, u8)` posicional já não se
/// lia, e trocar dois componentes de lugar por engano daria uma ordem errada que nenhum teste
/// pegaria de imediato.
#[derive(Default, PartialEq, Eq, Debug)]
struct Subdispositivo {
    paragrafo: u16,
    /// Letra do parágrafo acrescido (`§ 5º-M` → `M`). Vem DEPOIS do número e ANTES do inciso, então
    /// `§ 1º, I` < `§ 1º-A` — que é a ordem do texto: os incisos do § 1º vêm antes do § 1º-A.
    sufixo_paragrafo: u8,
    inciso: u16,
    alinea: u8,
    /// Item numerado dentro da alínea (`XIII, g.1` → `1`). A LC 123 usa; a CF e a 6.537 não.
    item: u8,
}

/// Lê o subdispositivo escrito depois do artigo.
///
/// **Corta por vírgula e ponto-e-vírgula, nunca por espaço**, e é essa escolha que faz o parser
/// funcionar: o acervo tem `Art. 138, II, e`, onde o `e` é a ALÍNEA, e tem `Art. 155, III e § 6º`,
/// onde o `e` é CONJUNÇÃO. O que os distingue é a vírgula. Cortando por espaço, um dos dois estaria
/// necessariamente errado.
///
/// O primeiro nível escrito manda, e o resto do segmento é ignorado: `caput e incisos I a III` é
/// caput; `§ 1º, IV; § 2º, IV e V; § 6º, I` é `§ 1º, IV`; `§§ 5º a 9º` é o `§ 5º`. Mesma leitura
/// conservadora do `Arts.` plural — chaveia pelo primeiro que está escrito, não pela faixa inteira.
fn subdispositivo(resto: &str) -> Subdispositivo {
    let vazio = Subdispositivo::default();
    let mut segs = resto
        .split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .peekable();

    match segs.peek() {
        None => return vazio,
        // `caput` explícito encerra a leitura: é o nível mais alto, e o que vier depois dele no
        // mesmo segmento é a companhia dele, não um nível mais fundo.
        Some(s) if primeira_palavra(s).eq_ignore_ascii_case("caput") => return vazio,
        Some(_) => {}
    }

    let (paragrafo, sufixo_paragrafo) = match segs.peek().and_then(|s| paragrafo_de(s)) {
        Some(p) => {
            segs.next();
            p
        }
        None => (0, 0),
    };

    let inciso = match segs.peek().and_then(|s| romano_de(primeira_palavra(s))) {
        Some(n) => {
            segs.next();
            n
        }
        // Nem parágrafo nem inciso no primeiro segmento: não há subdispositivo reconhecível —
        // texto livre como `(escopo do acervo)`, ou a faixa de artigos de `Arts. 85 a 89`.
        None if paragrafo == 0 => return vazio,
        None => 0,
    };

    // A alínea só existe pendurada num inciso. A guarda não é purismo: sem ela, o `a` de faixa em
    // `Arts. 85 a 89` seria lido como alínea `a`.
    let (alinea, item) = match inciso {
        0 => (0, 0),
        _ => segs
            .peek()
            .and_then(|s| alinea_de(primeira_palavra(s)))
            .unwrap_or((0, 0)),
    };
    Subdispositivo {
        paragrafo,
        sufixo_paragrafo,
        inciso,
        alinea,
        item,
    }
}

/// A primeira palavra de um segmento, sem aspas nem parênteses em volta. É ela que decide o nível;
/// o que vem depois é conjunção ou faixa (`XII` em `XII, a e b`, `IV` em `IV e V`).
fn primeira_palavra(seg: &str) -> &str {
    seg.split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '(' | ')'))
}

/// `(número, sufixo de letra)` do parágrafo, de um segmento que abre com `§`/`§§` — ou `(1, 0)` para
/// a grafia `parágrafo único`, que é o único parágrafo do artigo e portanto vem logo depois do caput
/// e dos incisos dele, como no texto.
///
/// O sufixo (`§ 5º-M`, `§ 15-A`) é o que a LC 123 trouxe: o art. 18 sozinho tem 17 parágrafos
/// acrescidos, e a série `§ 5º-A` a `§ 5º-M` empatava inteira sem ele. É o mesmo mecanismo do
/// sufixo do ARTIGO (`Art. 17-A`), um nível abaixo.
fn paragrafo_de(seg: &str) -> Option<(u16, u8)> {
    let Some(depois) = seg.strip_prefix("§§").or_else(|| seg.strip_prefix('§')) else {
        let p = primeira_palavra(seg).to_lowercase();
        return (p == "parágrafo" || p == "paragrafo").then_some((1, 0));
    };
    let depois = depois.trim_start();
    let digitos: String = depois.chars().take_while(char::is_ascii_digit).collect();
    let numero: u16 = digitos.parse().ok()?;

    // O ordinal é grafia (`5º`), o hífen é conteúdo (`5º-M`). Números altos vêm sem ordinal
    // (`§ 15-A`), então o `º` é opcional aqui.
    let resto = &depois[digitos.len()..];
    let resto = resto
        .strip_prefix('º')
        .or_else(|| resto.strip_prefix('°'))
        .unwrap_or(resto);
    let sufixo = resto
        .strip_prefix('-')
        .and_then(|r| r.chars().next())
        .filter(char::is_ascii_uppercase)
        .map_or(0, |c| c as u8);
    Some((numero, sufixo))
}

/// Numeral romano MAIÚSCULO (`I`, `IV`, `XII`) — o inciso. Exigir maiúscula é o que separa o inciso
/// da alínea e das preposições: o `a` de faixa e o `e` de ligação são minúsculos e nunca casam aqui.
fn romano_de(palavra: &str) -> Option<u16> {
    if palavra.is_empty() || !palavra.chars().all(|c| "IVXLCDM".contains(c)) {
        return None;
    }
    let valor = |c: char| match c {
        'I' => 1,
        'V' => 5,
        'X' => 10,
        'L' => 50,
        'C' => 100,
        'D' => 500,
        _ => 1000,
    };
    let letras: Vec<char> = palavra.chars().collect();
    // Em `i32` de propósito: a notação subtrativa (`IV`) passa por um acumulador negativo, e em
    // `u16` o primeiro `-1` estouraria por baixo.
    let mut total: i32 = 0;
    for (i, &c) in letras.iter().enumerate() {
        let v = valor(c);
        if letras.get(i + 1).is_some_and(|&p| valor(p) > v) {
            total -= v;
        } else {
            total += v;
        }
    }
    u16::try_from(total).ok()
}

/// Alínea e o item dentro dela: `(letra, item)`. Uma letra minúscula sozinha no seu próprio segmento
/// (`…, XII, a`) vale `(1, 0)`; com o item numerado da LC 123 (`…, XIII, g.1`) vale `(7, 1)`.
///
/// Sem o item, `g.1` e `g.2` não casavam como alínea nenhuma e caíam em `0` — ou seja, ANTES da
/// alínea `a`, que é o oposto da ordem do texto.
fn alinea_de(palavra: &str) -> Option<(u8, u8)> {
    let mut letras = palavra.chars();
    let c = letras.next().filter(char::is_ascii_lowercase)?;
    let letra = c as u8 - b'a' + 1;
    let resto: String = letras.collect();
    if resto.is_empty() {
        return Some((letra, 0));
    }
    // Só `.{dígitos}` continua sendo alínea; qualquer outra coisa é palavra comum ("aos", "e").
    let digitos = resto.strip_prefix('.')?;
    if digitos.is_empty() || !digitos.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((letra, digitos.parse().ok()?))
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
        let (header, _resumo, corpo) = mddoc::parse_doc(&texto).map_err(|e| {
            format!(
                "`{}` não parseia ({e}) — corrija antes de derivar o índice",
                caminho.display()
            )
        })?;
        out.push(Entrada {
            titulo: header.titulo,
            trilha: header.trilha,
            ementa: header.ementa,
            corpo: corpo.trim().to_string(),
            link: header.link,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os formatos REAIS da ementa, tirados das 288 do acervo (6.537 + CF + LC 123) — um por linha,
    /// com a chave `(artigo, sufixo, parágrafo, sufixo do parágrafo, inciso, alínea, item)`.
    #[test]
    fn chave_cobre_os_formatos_reais_da_ementa() {
        let casos = [
            // O comum, e o ordinal `º` — que não pode virar parte do número nem cortar o parse.
            ("Art. 28", (28, 0, 0, 0, 0, 0, 0)),
            ("Art. 1º", (1, 0, 0, 0, 0, 0, 0)),
            ("Art. 104", (104, 0, 0, 0, 0, 0, 0)),
            // Nível de caput: ausência, palavra explícita e companhia dos incisos — tudo no menor
            // valor, e por isso empatados entre si de propósito.
            ("Art. 155, caput", (155, 0, 0, 0, 0, 0, 0)),
            ("Art. 155, caput e incisos I a III", (155, 0, 0, 0, 0, 0, 0)),
            ("Art. 155 (escopo do acervo)", (155, 0, 0, 0, 0, 0, 0)),
            // Inciso do caput: depois do caput, antes de qualquer §.
            ("Art. 21, I", (21, 0, 0, 0, 1, 0, 0)),
            ("Art. 155, II", (155, 0, 0, 0, 2, 0, 0)),
            ("Art. 11, I, a", (11, 0, 0, 0, 1, 1, 0)),
            // Parágrafo, inciso e alínea, o caso fundo.
            ("Art. 121, § 2º", (121, 0, 2, 0, 0, 0, 0)),
            ("Art. 155, § 1º, IV", (155, 0, 1, 0, 4, 0, 0)),
            ("Art. 155, § 2º, IX, a", (155, 0, 2, 0, 9, 1, 0)),
            ("Art. 155, § 2º, XII, i", (155, 0, 2, 0, 12, 9, 0)),
            ("Art. 1º, parágrafo único", (1, 0, 1, 0, 0, 0, 0)),
            // Faixa e plural, em cada nível: chaveiam pelo PRIMEIRO escrito.
            ("Art. 96, §§ 5º a 9º", (96, 0, 5, 0, 0, 0, 0)),
            ("Art. 155, § 2º, VII e VIII", (155, 0, 2, 0, 7, 0, 0)),
            ("Art. 155, § 2º, XII, a e b", (155, 0, 2, 0, 12, 1, 0)),
            (
                "Art. 155, § 1º, IV; § 2º, IV e V; § 6º, I",
                (155, 0, 1, 0, 4, 0, 0),
            ),
            // Artigo acrescido: o sufixo é o segundo componente, e ainda aceita subdispositivo.
            ("Art. 17-A", (17, b'A', 0, 0, 0, 0, 0)),
            ("Art. 136-F", (136, b'F', 0, 0, 0, 0, 0)),
            ("Art. 136-A, §§ 1º e 2º", (136, b'A', 1, 0, 0, 0, 0)),
            // Plural e faixa de ARTIGOS: o ` a ` / ` e ` aqui não é alínea nem inciso.
            ("Arts. 85 a 89", (85, 0, 0, 0, 0, 0, 0)),
            ("Arts. 32 e 33", (32, 0, 0, 0, 0, 0, 0)),
            ("Arts. 136-C a 136-E", (136, b'C', 0, 0, 0, 0, 0)),
            // LC 123: PARÁGRAFO acrescido — o art. 18 tem 17, e a série `§ 5º-A` a `§ 5º-M`
            // empatava inteira antes deste componente.
            ("Art. 18, § 5º-E", (18, 0, 5, b'E', 0, 0, 0)),
            ("Art. 18, § 5º-M", (18, 0, 5, b'M', 0, 0, 0)),
            ("Art. 18, § 1º-B", (18, 0, 1, b'B', 0, 0, 0)),
            ("Art. 18, §§ 5º-B, 5º-D e 5º-F", (18, 0, 5, b'B', 0, 0, 0)),
            ("Art. 18, §§ 5º-J, 5º-K e 5º-M", (18, 0, 5, b'J', 0, 0, 0)),
            ("Art. 16, §§ 1º-A a 1º-D", (16, 0, 1, b'A', 0, 0, 0)),
            // Número alto vem SEM ordinal no texto legal — o `º` é opcional antes do hífen.
            ("Art. 18, § 15-A", (18, 0, 15, b'A', 0, 0, 0)),
            ("Art. 18, § 22-C", (18, 0, 22, b'C', 0, 0, 0)),
            // LC 123: ITEM dentro da alínea.
            ("Art. 13, § 1º, XIII, g.1", (13, 0, 1, 0, 13, 7, 1)),
            (
                "Art. 13, § 1º, XIII, g.2 e h, e § 5º",
                (13, 0, 1, 0, 13, 7, 2),
            ),
            ("Art. 13, § 1º, XIII, a a h", (13, 0, 1, 0, 13, 1, 0)),
        ];
        for (ementa, esperada) in casos {
            assert_eq!(
                chave_dispositivo(ementa),
                Some(esperada),
                "ementa: {ementa:?}"
            );
        }
    }

    /// O sufixo do parágrafo vem DEPOIS do número e ANTES do inciso, e essa posição é a regra:
    /// no texto legal, os incisos do `§ 1º` vêm antes do `§ 1º-A`, que é parágrafo próprio.
    #[test]
    fn o_paragrafo_acrescido_vem_depois_dos_incisos_do_paragrafo_dele() {
        let mut ementas = [
            "Art. 13, § 1º-A",
            "Art. 13, § 2º",
            "Art. 13, § 1º, XIII",
            "Art. 13, § 1º",
        ];
        ementas.sort_by_key(|e| chave_dispositivo(e).unwrap());
        assert_eq!(
            ementas,
            [
                "Art. 13, § 1º",
                "Art. 13, § 1º, XIII", // inciso do § 1º...
                "Art. 13, § 1º-A",     // ...antes do § 1º-A, que é outro parágrafo
                "Art. 13, § 2º",
            ]
        );
    }

    /// A vírgula é o que separa a alínea da conjunção — o par que justifica cortar por pontuação e
    /// não por espaço. Os dois existem no acervo, e um parser por espaço erraria necessariamente um.
    #[test]
    fn a_virgula_separa_a_alinea_da_conjuncao() {
        // `e` sozinho no seu segmento é a ALÍNEA `e` (5ª).
        assert_eq!(
            chave_dispositivo("Art. 138, II, e"),
            Some((138, 0, 0, 0, 2, 5, 0))
        );
        // O mesmo `e` colado ao inciso, sem vírgula, é CONJUNÇÃO — e não vira alínea nenhuma.
        assert_eq!(
            chave_dispositivo("Art. 155, III e § 6º"),
            Some((155, 0, 0, 0, 3, 0, 0))
        );
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
            Some(k) => (0u8, k),
            None => (1u8, (0, 0, 0, 0, 0, 0, 0)),
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

    /// Dentro de UM artigo — o caso da entrega da CF, em que os 41 pares são todos do `Art. 155` —
    /// a ordem é a da leitura do dispositivo: caput, incisos do caput, e então os parágrafos, cada
    /// um com seus incisos e alíneas. É este teste que falha se a chave voltar a ser só o artigo.
    #[test]
    fn dentro_do_artigo_a_ordem_desce_caput_inciso_paragrafo_alinea() {
        let mut ementas = [
            "Art. 155, § 2º, XII, g",
            "Art. 155, § 1º, I",
            "Art. 155, II",
            "Art. 155, § 2º, IX, a",
            "Art. 155, caput",
            "Art. 155, § 6º, I",
            "Art. 155, § 2º, XII",
            "Art. 155, § 2º, I",
            "Art. 155, III",
            "Art. 155, § 2º, XII, a",
        ];
        ementas.sort_by_key(|e| chave_dispositivo(e).unwrap());
        assert_eq!(
            ementas,
            [
                "Art. 155, caput",
                "Art. 155, II",  // incisos do caput: depois do caput...
                "Art. 155, III", // ...e antes de qualquer parágrafo
                "Art. 155, § 1º, I",
                "Art. 155, § 2º, I",
                "Art. 155, § 2º, IX, a", // IX antes de XII: romano por VALOR, não lexicográfico
                "Art. 155, § 2º, XII",   // o inciso sem alínea abre...
                "Art. 155, § 2º, XII, a", // ...e as alíneas vêm em ordem
                "Art. 155, § 2º, XII, g",
                "Art. 155, § 6º, I",
            ]
        );
    }

    /// O caso do art. 18 da LC 123, que motivou os dois níveis novos: uma série inteira de
    /// parágrafos acrescidos e um item dentro da alínea. Sem eles, os cinco `§ 5º-x` empatariam e
    /// cairiam na ordem alfabética da pergunta — o problema do art. 155 um nível abaixo.
    #[test]
    fn a_serie_de_paragrafos_acrescidos_da_lc123_ordena_pela_letra() {
        let mut ementas = [
            "Art. 18, §§ 5º-J, 5º-K e 5º-M",
            "Art. 18, § 5º-I",
            "Art. 18, §§ 5º-C e 5º-H",
            "Art. 18, caput e §§ 1º e 1º-A",
            "Art. 18, §§ 5º-B, 5º-D e 5º-F",
            "Art. 18, § 5º-E",
            "Art. 18, § 1º-B",
            "Art. 18, § 3º",
            "Art. 18, §§ 12 a 14",
        ];
        ementas.sort_by_key(|e| chave_dispositivo(e).unwrap());
        assert_eq!(
            ementas,
            [
                "Art. 18, caput e §§ 1º e 1º-A", // `caput` manda: nível mais alto
                "Art. 18, § 1º-B",
                "Art. 18, § 3º",
                "Art. 18, §§ 5º-B, 5º-D e 5º-F", // B < C < E < I < J, pela LETRA
                "Art. 18, §§ 5º-C e 5º-H",
                "Art. 18, § 5º-E",
                "Art. 18, § 5º-I",
                "Art. 18, §§ 5º-J, 5º-K e 5º-M",
                "Art. 18, §§ 12 a 14", // 12 > 5: por VALOR, não lexicográfico
            ]
        );
    }

    /// O item da alínea (`g.1`) vem depois das alíneas anteriores, não antes de todas. Sem o
    /// componente, `g.1` não casava como alínea e caía em `0` — ou seja, na frente do `a`.
    #[test]
    fn o_item_da_alinea_ordena_dentro_da_letra_dele() {
        let mut ementas = [
            "Art. 13, § 1º, XIII, g.2 e h, e § 5º",
            "Art. 13, § 1º, XIII, a a h",
            "Art. 13, § 1º, XIII, g.1",
            "Art. 13, § 1º, XIII, b a f",
        ];
        ementas.sort_by_key(|e| chave_dispositivo(e).unwrap());
        assert_eq!(
            ementas,
            [
                "Art. 13, § 1º, XIII, a a h",
                "Art. 13, § 1º, XIII, b a f",
                "Art. 13, § 1º, XIII, g.1", // g depois de b, e g.1 antes de g.2
                "Art. 13, § 1º, XIII, g.2 e h, e § 5º",
            ]
        );
    }
}
