#!/usr/bin/env python3
"""Trava de paridade do contexto RAG: reenvia perguntas de logs antigos e compara o CONTEXTO RAG.

Para que serve: garantir que uma mudança no caminho de recuperação NÃO alterou *quais* documentos
entram no contexto do LLM, nem a ordem deles. É a verificação que nenhum teste unitário dá — os
testes de `montar_rag_*` pinam o FORMATO, e os de `bloco_parecer` pinam a leitura do corpo, mas
nada cobre o conjunto e a ordem dos documentos recuperados.

Como funciona: cada log de auditoria em `./logs` já contém a pergunta original e o contexto RAG
que foi montado. O script reenvia a mesma pergunta (mesma entidade, mesmo tipo) ao servidor
atual via `/v1/question` e compara o `CONTEXTO RAG` que o **próprio servidor** grava. Exato por
construção: nada é reconstruído aqui. A resposta do LLM é descartada — ela varia com a
temperatura; o contexto, não.

Custo: uma chamada de LLM por log (a rota `/v1/question` é a única que chama o modelo externo, e
o log só é gravado depois que ela retorna). Rode contra um servidor LOCAL, não o de produção.

Uso:
    # 1. suba um servidor com os logs indo para um diretório separado
    AULI_DATA_DIR=./data EMBED_CACHE_DIR=./models AULI_LOG_DIR=/tmp/parity-logs \
      ./auli-server/target/release/auli server --port 3111 --bind 127.0.0.1

    # 2. compare (o -u mostra o progresso ao vivo; sem ele o Python bufferiza)
    python3 -u scripts/parity-replay.py logs /tmp/parity-logs http://localhost:3111

Saída: uma linha por log e um resumo. Exit 0 se tudo idêntico, 1 se houver divergência (com o
primeiro diff impresso).

Histórico: usado no G2 (motor `auli-retrieval`) sobre 40 consultas reais de produção —
16 `pareceres` + 24 `servicos+faqs`, entidades rs/sp/sc — todas byte-idênticas.
"""

import json
import pathlib
import re
import sys
import time
import urllib.request

CAB = re.compile(r"^CONSULTA · .+ · entidade: (\w+) · tipo: (.+)$", re.M)
TIPO_COD = {"servicos+faqs": 1, "pareceres": 2}
SECAO_RAG = "CONTEXTO RAG (documentos recuperados)"


def secao(txt: str, titulo: str):
    """Conteúdo de uma seção do log, entre o cabeçalho `----- TITULO ---` e a próxima seção/régua."""
    m = re.search(rf"^-{{5}} {re.escape(titulo)} -+$\n(.*?)(?=\n^(?:-{{5}} |={{10,}}))", txt, re.M | re.S)
    return m.group(1) if m else None


def perguntar(base: str, entidade: str, cod: int, pergunta: str):
    req = urllib.request.Request(
        base + "/v1/question",
        data=json.dumps({"entity": entidade, "type": cod, "question": pergunta}).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=180) as r:
        return json.load(r)


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__)
        return 2
    base_dir, new_dir, base = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
    new_dir.mkdir(parents=True, exist_ok=True)

    baselines = sorted(base_dir.glob("*.txt"))
    print(f"{len(baselines)} logs de baseline\n", flush=True)

    iguais, difs, pulados = 0, [], []
    for p in baselines:
        txt = p.read_text()
        m = CAB.search(txt)
        pergunta = secao(txt, "PERGUNTA (ORIGINAL)")
        antes = secao(txt, SECAO_RAG)
        if not m or pergunta is None or antes is None:
            pulados.append((p.name, "seções ausentes (log truncado?)"))
            continue
        entidade, tipo = m.group(1), m.group(2).strip()
        cod = TIPO_COD.get(tipo)
        if cod is None:
            pulados.append((p.name, f"tipo desconhecido: '{tipo}'"))
            continue

        antes_dir = {f.name for f in new_dir.glob("*.txt")}
        try:
            resp = perguntar(base, entidade, cod, pergunta.strip("\n"))
        except Exception as e:  # rede/timeout: registra e segue, não derruba a bateria
            pulados.append((p.name, f"erro HTTP: {e}"))
            continue

        novos = [f for f in new_dir.glob("*.txt") if f.name not in antes_dir]
        if not novos:
            # Duas causas possíveis, e a primeira é a pegadinha:
            #  1. `new_dir` não é o AULI_LOG_DIR do servidor — ele gravou o log em outro lugar;
            #  2. o servidor devolveu aviso amigável (entidade/coleção ausente) em vez de
            #     responder, e o `log_question` só roda no caminho completo.
            pulados.append((
                p.name,
                f"nenhum log novo em {new_dir} — confira que é o AULI_LOG_DIR do servidor "
                f"(resposta recebida: {str(resp.get('answer'))[:60]!r})",
            ))
            continue
        depois = secao(max(novos, key=lambda f: f.stat().st_mtime).read_text(), SECAO_RAG) or ""

        if antes.strip("\n") == depois.strip("\n"):
            print(f"  ✅ {p.name} [{entidade} {tipo}] idêntico ({len(antes)} chars)", flush=True)
            iguais += 1
        else:
            print(f"  ❌ {p.name} [{entidade} {tipo}] DIVERGE "
                  f"(antes {len(antes)} / depois {len(depois)})", flush=True)
            difs.append((p.name, antes, depois))
        # >1s: respeita o limiter (1 req/s) E evita colisão de nome de log (timestamp por segundo,
        # aberto em modo append — dois logs no mesmo segundo virariam um arquivo só).
        time.sleep(1.3)

    print(f"\n{'=' * 60}\nRESUMO: {iguais} idênticos · {len(difs)} divergentes · {len(pulados)} pulados")
    for nome, motivo in pulados:
        print(f"  ⏭️  {nome}: {motivo}")

    if difs:
        import difflib

        nome, a, b = difs[0]
        print(f"\n--- primeiro diff ({nome}) ---")
        for linha in list(difflib.unified_diff(
            a.split("\n"), b.split("\n"), "antes", "depois", lineterm=""
        ))[:60]:
            print(linha)
    return 1 if difs else 0


if __name__ == "__main__":
    sys.exit(main())
