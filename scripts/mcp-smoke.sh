#!/usr/bin/env bash
# Smoke do endpoint MCP (streamable HTTP): initialize → initialized → tools/list → tools/call.
#
# Valida a camada de PROTOCOLO: handshake, sessão, schemas das ferramentas e chamadas reais.
# É o degrau 1 da verificação do G4 (os degraus 2 e 3 usam clientes MCP de verdade — Claude Code
# em localhost e o conector remoto via tunnel).
#
# Uso: ./scripts/mcp-smoke.sh [http://localhost:3000/mcp]
set -euo pipefail
URL="${1:-http://localhost:3000/mcp}"
H=(-H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream')
HDRS=$(mktemp)
trap 'rm -f "$HDRS"' EXIT

# Com streamable HTTP a resposta chega como SSE — e o stream começa com um `data:` VAZIO antes do
# frame real, então não basta pegar a primeira linha `data:`. Aceita também JSON puro.
extrai_json() {
  sed -n 's/^data: //p' | grep -v '^[[:space:]]*$' | tail -1
}

rpc() { # rpc <json-body>
  curl -s "${H[@]}" -H "Mcp-Session-Id: $SID" "$URL" -d "$1" | extrai_json
}

echo "== initialize =="
curl -s -D "$HDRS" "${H[@]}" "$URL" -d '{
  "jsonrpc":"2.0","id":1,"method":"initialize",
  "params":{"protocolVersion":"2025-03-26","capabilities":{},
            "clientInfo":{"name":"mcp-smoke","version":"0.1"}}}' \
  | extrai_json | python3 -m json.tool 2>/dev/null | head -20

SID=$(grep -i '^mcp-session-id:' "$HDRS" | tr -d '\r' | awk '{print $2}')
if [ -z "$SID" ]; then
  echo "FALHA: servidor não devolveu Mcp-Session-Id. Se veio 406, confira o header Accept duplo." >&2
  exit 1
fi
echo "session: $SID"

echo
echo "== notifications/initialized =="
curl -s "${H[@]}" -H "Mcp-Session-Id: $SID" "$URL" \
  -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' >/dev/null
echo "ok"

echo
echo "== tools/list =="
rpc '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  | python3 -c 'import sys,json
ts = json.load(sys.stdin)["result"]["tools"]
print(len(ts), "ferramentas:")
for t in ts:
    nome = t["name"]
    desc = t["description"][:80]
    args = sorted((t.get("inputSchema") or {}).get("properties", {}).keys())
    print("  -", nome + ":", desc + "...")
    print("    args:", args)'

echo
echo "== tools/call listar_entidades =="
rpc '{"jsonrpc":"2.0","id":3,"method":"tools/call",
      "params":{"name":"listar_entidades","arguments":{}}}' \
  | python3 -c 'import sys,json
print(json.load(sys.stdin)["result"]["content"][0]["text"])' \
  | head -20

echo
echo "== tools/call buscar_pareceres (sc) =="
rpc '{"jsonrpc":"2.0","id":4,"method":"tools/call",
      "params":{"name":"buscar_pareceres","arguments":{
        "uf":"sc","pergunta":"crédito de ICMS na aquisição de energia elétrica","top_k":3}}}' \
  | python3 -c 'import sys,json
ps = json.loads(json.load(sys.stdin)["result"]["content"][0]["text"])
print(len(ps), "pareceres:")
for p in ps:
    print("  - [%.4f] %s" % (p["score"], p["numero"]))
    assert "corpo" not in p, "corpo NUNCA vai na busca"
print("ok: nenhum corpo no resultado da busca")'

echo
echo "== tools/call obter_parecer (corpo integral) =="
rpc '{"jsonrpc":"2.0","id":5,"method":"tools/call",
      "params":{"name":"obter_parecer","arguments":{
        "uf":"sc","numero":"CONSULTA COPAT nº 0091/17"}}}' \
  | python3 -c 'import sys,json
p = json.loads(json.load(sys.stdin)["result"]["content"][0]["text"])
corpo = p.get("corpo") or ""
print("numero:", p["numero"])
print("corpo: ", len(corpo), "chars —", repr(corpo[:90]))'

echo
echo "== tools/call buscar_pareceres (UF sem acervo) — erro guiando ao listar_entidades =="
rpc '{"jsonrpc":"2.0","id":6,"method":"tools/call",
      "params":{"name":"buscar_pareceres","arguments":{"uf":"zz","pergunta":"x"}}}' \
  | python3 -c 'import sys,json
e = json.load(sys.stdin)["error"]
print("code", e["code"], "-", e["message"])'

echo
echo "✅ smoke MCP concluído"
