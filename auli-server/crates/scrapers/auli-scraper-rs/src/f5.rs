// As duas páginas que o F5 na frente do portal da SEFAZ-RS devolve no lugar do conteúdo (desde
// out/2026), ambas com HTTP 200 — por isso o status não basta, e sem esta checagem elas entravam no
// cache como página válida e envenenavam as rodadas seguintes (cache-first SEMPRE).
//
// - o desafio JS do Bot Defense: `window["bobcmn"] = …/TSPD/…` + "Please enable JavaScript";
// - o bloqueio do ASM: `<title>Request Rejected</title>` + "Your support ID is: <…>".
//
// Nenhuma das ~1.250 páginas legítimas do cache de 09/10/2026 contém essas marcas.

/// `true` se `body` é a página de desafio ou de bloqueio do F5, não o conteúdo pedido.
pub fn bloqueio_f5(body: &str) -> bool {
    body.contains("window[\"bobcmn\"]") || body.contains("<title>Request Rejected</title>")
}

/// Utilitários dos testes de "não grava o bloqueio no cache", compartilhados por `servicos` e `faqs`.
#[cfg(test)]
pub(crate) mod teste {
    use std::io::{Read, Write};
    use std::path::Path;

    /// O desafio JS do F5, encurtado mas com as marcas do real.
    pub const DESAFIO: &str = r#"<!DOCTYPE html><html><head><script type="text/javascript">(function(){window["bobcmn"] = "1011111/TSPD/3TSPD_101";})();</script></head><body><noscript>Please enable JavaScript to view the page content.</noscript></body></html>"#;

    /// Servidor HTTP local que responde 200 com `body` a toda conexão — o F5 também responde 200.
    /// Devolve a origem (`http://127.0.0.1:<porta>`).
    pub fn servidor_local(body: &'static str) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origem = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for conexao in listener.incoming() {
                let Ok(mut s) = conexao else { continue };
                let _ = s.read(&mut [0u8; 8192]);
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
            }
        });
        origem
    }

    /// Diretório temporário vazio e exclusivo do teste.
    pub fn dir_vazio(nome: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("auli-rs-{}-{}", nome, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Quantos arquivos há sob `dir`, recursivamente.
    pub fn arquivos(dir: &Path) -> usize {
        std::fs::read_dir(dir)
            .map(|it| {
                it.flatten()
                    .map(|e| {
                        let p = e.path();
                        if p.is_dir() { arquivos(&p) } else { 1 }
                    })
                    .sum()
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconhece_o_desafio_js() {
        assert!(bloqueio_f5(teste::DESAFIO));
    }

    #[test]
    fn reconhece_o_bloqueio_do_asm() {
        let bloqueio = "<html><head><title>Request Rejected</title></head><body>The requested URL \
                        was rejected. Please consult with your administrator.<br><br>Your support \
                        ID is: <17841629005646077394></body></html>";
        assert!(bloqueio_f5(bloqueio));
    }

    #[test]
    fn nao_confunde_pagina_nem_json_legitimos() {
        assert!(!bloqueio_f5(
            r#"<html><head><title>Serviços ao Cidadão</title></head><body><div class="artigo__texto">ok</div></body></html>"#
        ));
        assert!(!bloqueio_f5(r#"{"resultados":{},"maisResultados":false}"#));
    }
}
