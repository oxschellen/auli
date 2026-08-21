//! Testes de ponta da rota `GET /v1/log/{uuid}` (D-LOG-3).
//!
//! Exercitam o Router de verdade — middleware de rate limit incluído —, sem socket e sem
//! `AppState`: a rota do log não precisa de estado, só do diretório de logs.
//!
//! **Cada requisição usa um IP diferente** no header `CF-Connecting-IP`. O limiter é 1 req/s com
//! burst 2 e é keyed por IP; sem isso o terceiro pedido de um teste viraria 429 e o teste passaria
//! a medir o limiter em vez da rota.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::LazyLock;

use auli_cli::api::{log_routes, ratelimit::question_rate_limiter};
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use tower::ServiceExt; // for `oneshot`
use uuid::Uuid;

/// Diretório de logs do teste, criado UMA vez por binário de teste.
///
/// `AULI_LOG_DIR` é estado global do processo, e os testes rodam em paralelo — daí o `LazyLock`,
/// que garante uma única escrita da variável antes de qualquer requisição.
static LOG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let dir = std::env::temp_dir().join(format!("auli-log-api-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    unsafe { std::env::set_var("AULI_LOG_DIR", &dir) };
    dir
});

/// Grava um arquivo de log na RAIZ plana do diretório, e devolve o UUID.
///
/// A raiz é deliberada: desde a subpasta mensal (D-LOG-6) a gravação de produção põe o arquivo em
/// `logs/AAAA-MM/`, então tudo que este helper produz é **legado**. Os testes que o usam seguem
/// verdes pelo fallback da D-LOG-7, e é isso que os torna o pino de integração dele — se o
/// fallback sumir, eles morrem juntos. O caminho novo tem teste próprio
/// (`log_na_subpasta_do_mes_e_servido_pela_rota`).
fn gravar_log(conteudo: &str) -> Uuid {
    let id = Uuid::now_v7();
    let nome = format!("2026-08-09_10-15-05_{id}.txt");
    std::fs::write(LOG_DIR.join(nome), conteudo).unwrap();
    id
}

/// Uma requisição à rota, com IP próprio para não disputar a cota com as outras.
async fn get(uri: &str, ip: &str) -> (StatusCode, String, String) {
    let app = log_routes(question_rate_limiter());
    let mut req = Request::builder()
        .uri(uri)
        .header("CF-Connecting-IP", ip)
        .body(Body::empty())
        .unwrap();
    // O middleware do rate limit extrai `ConnectInfo`; sem ele o extractor rejeita e vira 500.
    req.extensions_mut()
        .insert(ConnectInfo("127.0.0.1:0".parse::<SocketAddr>().unwrap()));

    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let corpo = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        content_type,
        String::from_utf8(corpo.to_vec()).unwrap(),
    )
}

/// Caminho feliz: o conteúdo volta **byte a byte**, sem filtro nem reformatação (D-LOG-5), e com
/// charset declarado — o registro tem acentuação (`ADERÊNCIA`) e sem isso o browser adivinha.
#[tokio::test]
async fn uuid_existente_devolve_o_arquivo_byte_a_byte() {
    LazyLock::force(&LOG_DIR);
    let conteudo =
        "----- PERGUNTA (ORIGINAL) -----\nquanto é a alíquota?\n\n----- ADERÊNCIA -----\n";
    let id = gravar_log(conteudo);

    let (status, tipo, corpo) = get(&format!("/v1/log/{id}"), "203.0.113.1").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(tipo, "text/plain; charset=utf-8");
    assert_eq!(corpo, conteudo, "o log tem de sair como está, sem edição");
}

/// UUID bem formado que não existe ⇒ 404 amigável.
#[tokio::test]
async fn uuid_valido_inexistente_da_404() {
    LazyLock::force(&LOG_DIR);
    let (status, _, corpo) = get(&format!("/v1/log/{}", Uuid::now_v7()), "203.0.113.2").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(corpo.contains("expirado ou inexistente"), "corpo: {corpo}");
}

/// **A validação de formato é a defesa contra traversal.** Nenhuma destas entradas é um UUID, então
/// nenhuma chega a virar caminho — a resposta é 400, e não 404, porque a recusa acontece antes de
/// qualquer olhada no disco.
///
/// Um `..%2f` que porventura escapasse daria 404 (não existe arquivo assim) e o teste passaria
/// enganado; por isso o que se afirma aqui é o **400**, que só a peneira de formato produz.
#[tokio::test]
async fn entrada_que_nao_e_uuid_da_400_antes_de_tocar_o_disco() {
    LazyLock::force(&LOG_DIR);
    let hostis = [
        "..%2f..%2f..%2fetc%2fpasswd",
        "..%2F..%2Fetc%2Fshadow",
        "%2e%2e%2f%2e%2e%2fetc%2fhosts",
        "nao-e-uuid",
        "00000000-0000-0000-0000-00000000000", // um dígito a menos
        "0000000g-0000-0000-0000-000000000000", // 'g' não é hex
    ];

    for (i, entrada) in hostis.iter().enumerate() {
        let ip = format!("198.51.100.{}", i + 1);
        let (status, _, corpo) = get(&format!("/v1/log/{entrada}"), &ip).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "entrada: {entrada}");
        assert!(
            !corpo.contains(entrada),
            "o 400 não deve ecoar a entrada do cliente: {corpo}"
        );
    }
}

/// A mensagem do 404 é a MESMA para "nunca existiu" e para "podado pela retenção" — quem chuta
/// UUIDs não descobre por diferença de resposta qual dos dois aconteceu.
#[tokio::test]
async fn inexistente_e_podado_respondem_identico() {
    LazyLock::force(&LOG_DIR);

    // "Podado": o arquivo existiu e foi apagado — exatamente o que a retenção manual faz.
    let podado = gravar_log("registro que será apagado");
    std::fs::remove_file(LOG_DIR.join(format!("2026-08-09_10-15-05_{podado}.txt"))).unwrap();

    let (s1, t1, c1) = get(&format!("/v1/log/{podado}"), "203.0.113.10").await;
    let (s2, t2, c2) = get(&format!("/v1/log/{}", Uuid::now_v7()), "203.0.113.11").await;

    assert_eq!((s1, t1, c1), (s2, t2, c2));
}

/// D-LOG-6 de ponta a ponta: arquivo na subpasta do mês derivada do UUID é servido pela rota.
///
/// O mês está **hardcoded** de propósito (1788224400 = 2026-09-01T01:00:00Z ⇒ "2026-09" em UTC):
/// é uma fonte independente da derivação de produção, então se ela mudar — para horário local,
/// por exemplo —, este teste morre em vez de acompanhar a mudança.
#[tokio::test]
async fn log_na_subpasta_do_mes_e_servido_pela_rota() {
    LazyLock::force(&LOG_DIR);
    let ts = uuid::Timestamp::from_unix(uuid::NoContext, 1_788_224_400, 0);
    let id = Uuid::new_v7(ts);
    let mes_dir = LOG_DIR.join("2026-09");
    std::fs::create_dir_all(&mes_dir).unwrap();
    // Prefixo LOCAL de 31/08 com subpasta UTC de 09: é exatamente o descompasso de até 3h que a
    // D-LOG-6 aceita na virada do mês, e a rota tem de achar assim mesmo.
    std::fs::write(
        mes_dir.join(format!("2026-08-31_22-00-00_{id}.txt")),
        "registro novo",
    )
    .unwrap();

    let (status, _, corpo) = get(&format!("/v1/log/{id}"), "203.0.113.20").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(corpo, "registro novo");
}

/// Não existe rota de listagem (D-LOG-3): sem o UUID não há como chegar a log nenhum.
#[tokio::test]
async fn nao_ha_rota_de_listagem() {
    LazyLock::force(&LOG_DIR);
    for uri in ["/v1/log", "/v1/log/", "/v1/logs"] {
        let (status, _, _) = get(uri, "192.0.2.1").await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{uri} não pode responder nada útil"
        );
    }
}
