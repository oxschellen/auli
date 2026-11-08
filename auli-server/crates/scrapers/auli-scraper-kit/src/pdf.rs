//! O texto de um PDF pelo poppler, como processo: `pdftotext -layout` (o parágrafo começa recuado, o rodapé fica numa
//! linha, as páginas separadas por `\f`) e a data de criação pelo `pdfinfo -isodates`. As FAQs do PI e da BA
//! (`docs/faqs/ESTUDO-RO-PI-Faqs.md` §3.1, `docs/faqs/ESTUDO-BA-Faqs.md`). Quem coleta precisa do poppler-utils; os testes usam o
//! texto extraído como fixture.

use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::{Context, Result, bail};

/// Um PDF lido.
#[derive(Debug, Clone, PartialEq)]
pub struct Pdf {
    pub texto: String,
    /// `(ano, mês)` da criação, quando o PDF a declara.
    pub criado: Option<(u32, u32)>,
}

static SEQ: AtomicU32 = AtomicU32::new(0);

/// O texto e a data de criação de um PDF. Erro se o poppler não está instalado ou não lê o arquivo.
pub fn extrair(pdf: &[u8]) -> Result<Pdf> {
    if !pdf.starts_with(b"%PDF") {
        bail!("o corpo não é um PDF");
    }
    let caminho = std::env::temp_dir().join(format!(
        "auli-pdf-{}-{}.pdf",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&caminho, pdf).with_context(|| format!("gravar {}", caminho.display()))?;
    let texto = rodar("pdftotext", &["-layout", "-enc", "UTF-8"], &caminho, true);
    let info = rodar("pdfinfo", &["-isodates"], &caminho, false);
    let _ = std::fs::remove_file(&caminho);
    Ok(Pdf {
        texto: texto?,
        criado: criado(&info?),
    })
}

fn rodar(programa: &str, args: &[&str], pdf: &Path, para_stdout: bool) -> Result<String> {
    let mut cmd = Command::new(programa);
    cmd.args(args).arg(pdf);
    if para_stdout {
        cmd.arg("-");
    }
    let saida = cmd
        .output()
        .with_context(|| format!("{programa} não rodou — instale o poppler-utils"))?;
    if !saida.status.success() {
        bail!(
            "{programa} falhou: {}",
            String::from_utf8_lossy(&saida.stderr).trim()
        );
    }
    String::from_utf8(saida.stdout)
        .with_context(|| format!("{programa} devolveu texto fora de UTF-8"))
}

/// O `(ano, mês)` da linha "CreationDate: 2020-04-29T15:23:23-03" do `pdfinfo -isodates`.
fn criado(info: &str) -> Option<(u32, u32)> {
    let data = info
        .lines()
        .find_map(|l| l.strip_prefix("CreationDate:"))?
        .trim();
    let ano = data.get(0..4)?.parse().ok()?;
    let mes = data.get(5..7)?.parse().ok()?;
    (1..=12).contains(&mes).then_some((ano, mes))
}

/// "abr/2020".
pub fn mes_ano((ano, mes): (u32, u32)) -> String {
    const MESES: [&str; 12] = [
        "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
    ];
    format!("{}/{ano}", MESES[(mes - 1) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_de_criacao_do_pdfinfo() {
        let info =
            "Producer: x\nCreationDate:    2020-04-29T15:23:23-03\nModDate: 2020-04-30T00:00:00Z\n";
        assert_eq!(criado(info), Some((2020, 4)));
        assert_eq!(criado("Producer: x\n"), None);
        assert_eq!(criado("CreationDate: lixo\n"), None);
        assert_eq!(mes_ano((2020, 4)), "abr/2020");
        assert_eq!(mes_ano((2019, 12)), "dez/2019");
    }

    #[test]
    fn corpo_que_nao_e_pdf_e_erro() {
        assert!(extrair(b"<html>").is_err());
    }
}
