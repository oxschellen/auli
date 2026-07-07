# Descoberta — SEFAZ-PB (Paraíba), 24ª entidade

## Fonte
- Portal institucional: `https://www.sefaz.pb.gov.br/` = **Joomla** (generator "Envolute", `com_content`).
- **Carta de Serviços** (fonte real): `https://cartaservico.sefaz.pb.gov.br/` — app **PHP** server-rendered
  (Apache, PHP/7.4). `servicos.php` = listagem; `saibamais.php?id=N` = ficha do serviço.

## Listagem (`servicos.php`)
- Accordion aninhado: **categoria de topo → público ("Para Empresa"/"Para o Cidadão") → subcategoria →
  serviço** (`<li><a href="saibamais.php?id=N">Título</a></li>`).
- **101 serviços** (id 1..101). Cada id aparece **2×** na listagem (uma árvore por público) → dedup por id.
- `classe` = a subcategoria imediata = o botão de accordion mais próximo ANTES do link que **não** é
  rótulo de público. ~51 classes distintas (fino mas fiel, como DF/142).

## Ficha (`saibamais.php?id=N`) — rica
Ficha estruturada com pares `<h3>Rótulo:</h3><h6 class="h6">Valor</h6>`:
**O que é o serviço**, **Público-alvo**, Forma de prestação, Taxa, Agendamento, Exigências, Quanto
tempo leva, Etapas do serviço, Documentação necessária, Unidades Físicas, Horário de Atendimento,
Contato, Informações adicionais. Além disso:
- **título** em `<div class="inputbutton01" title="…">` (nome completo do serviço);
- **link real** do serviço em `onclick="redireciona('id','URL')"` (o botão ACESSAR SERVIÇO).

## Modelagem (molde TO/DF, rico)
- `titulo` e `descricao` vêm do detalhe; descrição = os pares (menos o Público-alvo) + "Acessar o
  serviço: {URL}"; campos vazios ("-") descartados.
- **público** = campo "Público-alvo" da ficha (lista por vírgula → Cidadão/Empresa, per-serviço).
- `classe` = subcategoria da listagem; `link` = `saibamais.php?id=N` (único, identidade); órgão SEFAZ-PB.
- `ocorrencias` = público-alvo × classe (1–2 por serviço).

## Rede / TLS
- `cartaservico.sefaz.pb.gov.br` responde 200 em HTTPS padrão (Apache); UA AuliBot aceito. Sem gotcha
  de fingerprint (ureq funciona, diferente de GO/DF).

## Decisão
Sem fork relevante: catálogo genuinamente rico (101 fichas estruturadas) → **buscar os 101 detalhes**
(molde TO/DF), público vindo do próprio dado. Sem headless.
