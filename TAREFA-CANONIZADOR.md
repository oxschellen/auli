# TAREFA-CANONIZADOR — subcomando `canonizar` (chave canônica de dispositivos)

## Contexto

Segundo incremento do knowledge graph do Auli, direto sobre a saída do `extrair`
(TAREFA-EXTRACAO). O `extrair` copiou os dispositivos **literais** de propósito — trilha de
auditoria — sabendo que a canonização seria um passo FUTURO, **determinístico e sem LLM**. Este é
esse passo.

O `canonizar` lê `data/<id>/extracao/extracao.jsonl`, parseia cada `dispositivo.texto` numa **chave
canônica hierárquica** e colapsa variantes. Nada de rede, nada de LLM: pode ser re-rodado sobre o
JSONL existente quantas vezes for preciso, a custo zero. O literal **nunca** é destruído — ele fica
em cada linha ao lado da chave (a mesma disciplina do `extrair`).

**Grounding empírico (RS, 1ª rodada — 372/372 pareceres, 1.502 literais distintos / 1.903 citações):**

- Citações são **regulares e hierárquicas**: artigo (1.130), Livro (985), inciso (595),
  item/subitem (223), Seção (189), alínea (186), § (181), Apêndice (166), Capítulo (139).
  Uma citação é um caminho: `norma › livro/anexo/apêndice › artigo › §/inciso/alínea/item`.
- **RICMS domina** (56% das citações). Depois: Instrução Normativa (7,7%), Decreto (5,4%),
  Lei Complementar (4,8%), Lei (3,9%), Convênio ICMS (3,5%), Resolução, Protocolo, CF, EC, Portaria.
- **A poeira de variantes é mecânica.** Protótipo determinístico colapsou **1.123 literais em 607
  chaves (~46%)** com regras de pontuação/alias. Pares reais que DEVEM colapsar:
  - `inciso X do artigo 27 do Livro I do RICMS` (18×) ≡ `... do Regulamento do ICMS` (2×)
  - `Emenda Constitucional nº 87/15` (11×) ≡ `... n.º 87/15` (5×)
  - `§ 4.º do artigo 46 do Livro I do RICMS` ≡ `§ 4º do artigo 46 ...` ≡ `... Regulamento do ICMS`
- **A cauda dura é ~25% (379 literais)** e é genuinamente não-canonizável sem contexto:
  - anáfora (117): `§ 4º do **referido** artigo 31`, `desse artigo 38-A`;
  - sem norma identificável (233): `inciso II do artigo 25-B` (a que regulamento?);
  - despejo de texto de artigo (parte dos "longos", 29): `Art. 32 - Assegura-se direito a crédito...`.

## Decisões fechadas (não rediscutir)

| # | Decisão |
|---|---------|
| K1 | Fonte = `data/<id>/extracao/extracao.jsonl` (NÃO os `.md`). Determinístico, sem rede, sem LLM, idempotente e re-rodável. |
| K2 | **Cauda dura = `canonizavel: false` + `canon_key: null`, `texto` literal preservado.** Nada é descartado nem "chutado"; o grafo só omite o nó não-resolvido. (Mesma filosofia de trilha de auditoria do `extrair`.) |
| K3 | **RS primeiro** (one-shot; a análise do colapso decide a v2 das regras). A tabela de aliases de norma nasce RS mas estruturada para generalizar (`ricms-rs` hoje, `ricms-sc`/`ricms-pr` depois). |
| K4 | Chave = slug hierárquico determinístico `norma[:livro][:apêndice][:artigo][:§][:inciso][:alínea][:item]`. Ordinais SEM pontuação (`2º`=`2.º`=`2`); incisos em romano MAIÚSCULO; sufixo de artigo preservado (`38-A`, `1-K`); alínea minúscula. |
| K5 | Norma canonizada por **tabela de aliases** (`RICMS`/`Regulamento do ICMS`/`(RICMS)`→`ricms-rs`; `CTN`↔`Código Tributário Nacional`; `LC`↔`Lei Complementar`; `EC`↔`Emenda Constitucional`; `CF`↔`Constituição Federal`; `Convênio ICMS`; `IN DRP`; `Decreto`; `Lei`). Número/ano com separadores normalizados; **dígitos verbatim** (nunca inventar/completar). |
| K6 | Saídas (irmãs em `data/<id>/extracao/`): `dispositivos.jsonl` (1 linha por OCORRÊNCIA: `{numero, texto, canon_key, canon_display, canonizavel}`) + `dispositivos-index.json` (semente do grafo: `canon_key → {display, ocorrencias, variantes[], pareceres[]}`). |
| K7 | `canon_display` = UMA forma humana normalizada por chave (a variante mais frequente, ou reconstruída da chave — decidir na C1). O literal fica em `variantes[]` do índice. |
| K8 | Regravação integral atômica (`.tmp`+rename): a saída é derivada e determinística, reescrita inteira a cada rodada (sem append incremental — difere do `extrair`, cuja rodada é cara/LLM). |

## Fases

- **C1** — módulo novo `auli-collections/src/canonizar.rs` (parser + chave + índice) + fiação no `main.rs`
  (subcomando `canonizar`, mesma família OFFLINE do `indice`; sem flags de LLM).
- **C2** — testes embutidos: o parser sobre os **pares reais do RS** (as variantes acima DEVEM cair na
  mesma chave; a cauda dura DEVE virar `canonizavel:false`).
- **C3** — execução no RS + análise do colapso (roteiro no fim; Carlos roda, fora do aceite).

---

## Parser — gramática (a partir dos literais reais)

Ordem de reconhecimento (primeira que casa vence), sobre o texto com espaços colapsados:

1. **Descarte para a cauda (K2)**, nesta ordem:
   - anáfora: contém `referido|referida|desse|dessa|deste|desta|seu|sua|mesmo|citad[oa]` → `anafora`;
   - despejo de artigo: casa `^art\.?\s*\d+\s*[-–—]\s` (ex.: `Art. 32 - Assegura-se...`) → `texto-artigo`;
   - resíduo longo (> ~120 chars sem estrutura clara) → `longo`.
2. **Norma** (tabela K5). Para normas com identificador (LC/EC/Convênio/IN/Decreto/Lei), extrair
   `número/ano` → normalizar separadores, manter dígitos. **Lei precisa de parser próprio** (o
   protótipo falhou aqui, mashando tudo em `lei:?`): `Lei (Federal|Estadual)? n[º.°]?\s*([\d.]+)[,/]?\s*(?:de\s+[\d.]+|/\s*(\d+))` → `lei:<numero>/<ano?>`. Sem norma identificável → `norma-desconhecida` (cauda).
3. **Componentes hierárquicos** (todos opcionais, extraídos independentes):
   - `livro (I|II|III)`; `apêndice (romano)`; `anexo`; `seção`; `capítulo`;
   - `art(igo)? (\d+(-[A-Z])?)`; `§|parágrafo (\d+|único)`; `inciso (romano)`; `alínea ([a-z])`;
     `(sub)?item ([\d.]+)`.
4. **Chave** = junção com `:` na ordem de K4, prefixos: `l`, `ap`, `ax`, `art`, `§`, `inc`, `al`, `it`.
   Ex.: `inciso X do artigo 27 do Livro I do RICMS` → `ricms-rs:lI:art27:incX`.

Regra de ouro (K5): quando em dúvida entre canonizar e preservar, **preserva** (cauda). Um merge
errado polui o grafo; um literal a mais no `variantes[]` não custa nada.

## Esquema de saída

`dispositivos.jsonl` (uma linha por ocorrência, ordem estável por `numero` depois por texto):
```json
{"numero":"Parecer nº 15006","texto":"inciso IX do artigo 4.º ... do Livro I do RICMS","canon_key":"ricms-rs:lI:art4:incIX","canon_display":"art. 4º, inc. IX, Livro I do RICMS/RS","canonizavel":true}
{"numero":"...","texto":"§ 4º do referido artigo 31","canon_key":null,"canon_display":null,"canonizavel":false}
```

`dispositivos-index.json` (semente do grafo, ordenado por `ocorrencias` desc):
```json
{
  "ricms-rs:lI:art27:incX": {
    "display": "art. 27, inc. X, Livro I do RICMS/RS",
    "ocorrencias": 20,
    "variantes": ["inciso X do artigo 27 do Livro I do RICMS", "inciso X do artigo 27 do Livro I do Regulamento do ICMS"],
    "pareceres": ["Parecer nº 15006", "..."]
  }
}
```

## C4 — Aceite

1. `cargo build -p auli-collections` limpo; `cargo clippy -- -D warnings` limpo; `cargo fmt` (stable).
2. `cargo test -p auli-collections` — testes novos do `canonizar.rs` passam E a regressão do resto.
   Testes-chave (pares reais): as variantes RICMS/`Regulamento do ICMS`, `nº`/`n.º`, `§ 2º`/`§ 2.º`
   colapsam na mesma `canon_key`; anáfora e `Art. N -` viram `canonizavel:false`.
3. Fumaça: `auli-collections rs canonizar` sobre a `extracao.jsonl` real → `dispositivos.jsonl` +
   `dispositivos-index.json` parseáveis; re-rodar produz saída byte-idêntica (idempotência).

## C5 — Roteiro de análise (Carlos roda; fora do aceite)

```bash
cd data/rs/extracao
# Taxa de colapso e cobertura
jq -r 'select(.canonizavel).canon_key' dispositivos.jsonl | sort -u | wc -l        # chaves distintas
jq -r 'select(.canonizavel|not).texto' dispositivos.jsonl | wc -l                  # cauda dura
# Chaves com MAIS variantes literais (mede o ganho do canonizador)
jq -r 'to_entries[] | "\(.value.variantes|length)\t\(.key)"' dispositivos-index.json | sort -rn | head
# Auditoria da cauda: por que não canonizou (revisar → v2 das regras)
jq -r 'select(.canonizavel|not).texto' dispositivos.jsonl | sort | uniq -c | sort -rn | head -40
```

## Fora de escopo (explícito)

Montagem/visualização do grafo (parecer ↔ dispositivo ↔ tema); resolução de anáfora (precisa do
corpo do parecer — passo futuro); vocabulário controlado de temas (tarefa irmã); outros estados além
do RS; mudanças no `extrair`/`auli-contract`/`update`/pack.
