# Grafo de jurisprudência — visualizador

`grafo-rs.html` é um visualizador **self-contained** (HTML + CSS + JS + dados num
único arquivo, sem dependências, sem servidor) da malha de citações do ICMS gaúcho:
cada nó colorido é um **dispositivo legal** canônico, cada nó contornado é um **tema
tributário**, e as ligações são co-citação (dispositivo↔dispositivo) e co-ocorrência
(tema↔dispositivo). Abra no navegador — arraste para mover, scroll para zoom, clique
num tema para acender as normas que a jurisprudência invoca ao seu redor.

## Proveniência

Os dados vêm do pipeline offline do knowledge graph, sobre o acervo do RS:

```
auli-collections rs extrair    # LLM: {dispositivos, ncm, temas} por parecer
auli-collections rs canonizar  # determinístico: chave canônica dos dispositivos
auli-collections rs grafo      # determinístico: nós + arestas + layout -> grafo.json
```

O `grafo.json` resultante (em `data/rs/extracao/`, fora do git) é **embutido** no bloco
`<script id="graph-data" type="application/json">` deste HTML. O núcleo mostrado são os
dispositivos citados em ≥3 pareceres (co-citação ≥2) + os 16 temas mais frequentes.

## Regenerar

Depois de re-rodar o pipeline (ou para outro estado), reinjete o JSON no lugar do
conteúdo do bloco `graph-data`:

```bash
python3 - <<'PY'
import re
data = open('data/rs/extracao/grafo.json').read()
h = open('tools/grafo/grafo-rs.html').read()
h = re.sub(r'(<script id="graph-data" type="application/json">).*?(</script>)',
           lambda m: m.group(1) + data + m.group(2), h, count=1, flags=re.S)
open('tools/grafo/grafo-rs.html', 'w').write(h)
PY
```

O schema que o visualizador consome é o do `grafo.json`: `nodes[{kind,id,label,fam?,val,x,y}]`
e `edges[{s,t,w,k}]` (`k`: 0 = co-citação, 1 = tema↔dispositivo).
