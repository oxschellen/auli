# Grafos do acervo do RS — visualizadores

Dois visualizadores **self-contained** (HTML + CSS + JS + dados num único arquivo, sem
dependências, sem servidor). Abra no navegador — arraste para mover, scroll para zoom, clique
num nó para focar.

| arquivo | acervo | nó colorido | nó contornado | aresta |
|---|---|---|---|---|
| `grafo-rs.html` | 358 pareceres | dispositivo legal canônico (cor = família da norma) | tema tributário | co-citação e co-ocorrência |
| `grafo-rs-servicos.html` | 422 serviços | sistema/portal canônico (cor = público predominante) | assunto do serviço | co-menção e co-ocorrência |

O segundo é **derivado do primeiro**: mesma estética, mesmo layout force-directed, mesmo schema.
Só muda o que é nó — em vez de "quais normas a jurisprudência invoca junto", ele mostra "por quais
sistemas o contribuinte passa e o que os liga".

## Proveniência

Os dados vêm do pipeline offline do knowledge graph, sobre o acervo do RS:

```bash
# jurisprudência
auli-collections rs extrair              # LLM: {dispositivos, ncm, temas} por parecer
auli-collections rs canonizar            # determinístico: chave canônica dos dispositivos
auli-collections rs grafo                # determinístico: nós + arestas + layout -> grafo.json

# carta de serviços
auli-collections rs extrair-servicos     # LLM: {sistemas, temas} por serviço
auli-collections rs canonizar-servicos   # determinístico: chave canônica dos sistemas
auli-collections rs grafo-servicos       # determinístico: -> grafo-servicos.json
```

Os JSON resultantes (em `data/rs/extracao/`, fora do git) são **embutidos** no bloco
`<script id="graph-data" type="application/json">` de cada HTML.

O núcleo mostrado difere entre os dois porque os acervos têm densidades diferentes — os limiares
foram medidos, não copiados:

- **jurisprudência**: dispositivos citados em ≥3 pareceres, co-citação ≥2, 16 temas-topo
  → 97 dispositivos + 16 temas, 381 arestas.
- **serviços**: a malha é bem mais rala (415 menções para 133 sistemas; só 14 pares co-mencionados
  ≥2×). Com os limiares da jurisprudência sobrariam ~10 nós, então: sistemas em ≥2 serviços,
  co-menção ≥1, 24 assuntos-topo → 22 sistemas + 21 assuntos, 117 arestas. Note que **47% dos
  serviços não citam sistema nenhum** — o grafo cobre os 215 que citam.

## Regenerar

Depois de re-rodar o pipeline (ou para outro estado), reinjete o JSON no lugar do conteúdo do
bloco `graph-data`:

```bash
python3 - <<'PY'
import re
PARES = [('data/rs/extracao/grafo.json',          'tools/grafo/grafo-rs.html'),
         ('data/rs/extracao/grafo-servicos.json', 'tools/grafo/grafo-rs-servicos.html')]
for dados, html in PARES:
    d = open(dados).read().strip()
    h = open(html).read()
    h = re.sub(r'(<script id="graph-data" type="application/json">).*?(</script>)',
               lambda m: m.group(1) + d + m.group(2), h, count=1, flags=re.S)
    open(html, 'w').write(h)
PY
```

O schema que os visualizadores consomem é o do JSON: `nodes[{kind,id,label,fam?,val,x,y}]` e
`edges[{s,t,w,k}]` (`k`: 0 = entre nós coloridos, 1 = contornado↔colorido). `kind` é `disp`/`tema`
na jurisprudência e `sis`/`tema` nos serviços.
