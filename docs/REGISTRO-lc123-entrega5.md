# REGISTRO — LC 123/2006, 5ª entrega (arts. 18-A a 18-F — MEI, bloco C1 da Fase C)

Data: 19/08/2026. Pares 73–90 (numeração própria da lei). Pasta (D-LEG-4): "Lei Complementar 123-2006 — Simples Nacional". Nomes provisórios com prefixo do nº do par; rename canônico via `nome_faq_de` na integração (§3.6 do REGISTRO-legislacao-perguntas).

## Fonte — Rota A da Fase C

- **Fonte:** TXT extraído do PDF consolidado da Câmara/LEGIN "normaatualizada" (`pdftotext -layout`, 3.613 linhas), fornecido por **upload** — a rota de extração direta do arquivo, porque o fetch não lê o PDF da Câmara (stream comprimido) e o `lcp123.htm` do Planalto trunca no art. 18, § 15-A.
- Bloco extraído: **linhas 1247–1532** do TXT (do "Art. 18-A." até a linha anterior ao "Art. 19.").
- Os **links** dos pares seguem nas âncoras do Planalto por artigo (`#art18a` … `#art18f`), convenção do acervo.

## Corte

Regra mantida: **última alteração anterior à LC 214, de 16/1/2025**. Verificado por grep no bloco: **9 marcas da LC 214 e 1 da LC 227/2026 (§ 7º, I), todas "(Vide ...)"** — efeito futuro com **texto inalterado**. Zero redações novas e zero acréscimos da 214 no bloco ⇒ o texto pré-corte é **idêntico** ao consolidado da fonte, gerado direto (conforme o LEIA-ME da pasta de insumo e a auditoria de 18/08: C1 limpo). Última alteração pré-corte no bloco: **LC 188/2021** (art. 18-A, § 1º e incisos; art. 18-F inteiro).

## Formato — mudança desta entrega

Pares emitidos **já no formato canônico** do `render_doc` (`auli-contract/src/mddoc.rs`): frontmatter plano `chave: valor` sem aspas, quatro chaves sempre presentes, `## corpo` como única seção. Fonte do formato: o próprio código no repo (main @ #153) + convenções do `REGISTRO-legislacao-perguntas.md` §3 — substitui o gabarito combinado e **dispensa o script de normalização** usado nas integrações anteriores.

## Armadilhas do bloco

- § 3º, IV e V, "a"–"e": as alíneas marcadas "(Vide LC 214)" são anotações de efeito futuro — dentro do corte com o texto atual.
- § 4º, I: a remissão a "Anexos V **ou VI**" permanece no texto, mas o **Anexo VI foi revogado** pela LC 155/2016 (nota no par 77).
- § 4º, IV: **revogado** (LC 155/2016).
- R$ 45,65 (§ 3º, V, "a") é **valor nominal de referência reajustável** (§ 11) — os pares não congelam valor em reais; a nota remete aos 5% do salário-mínimo (Lei 8.212, art. 21, § 2º, II, "b").
- § 7º, I: "(Vide LC 227/2026)" — efeito futuro, texto inalterado no corte.

## Mapa dos pares

- **73** — 18-A, caput e § 1º: o que é MEI, limite 81.000, quem pode (art. 966 CC; extrativista/CGSN/rural).
- **74** — § 2º: limite proporcional de início (6.750/mês, fração = mês inteiro).
- **75** — § 3º, I–III e VI: o que não se aplica na vigência + isenção dos federais (ressalva 18-C).
- **76** — § 3º, IV–V, §§ 11 e 15-A: DAS-MEI (INSS 5% SM + R$ 1 ICMS + R$ 5 ISS), reajuste, remissão de débitos estaduais/municipais.
- **77** — §§ 4º, 4º-A e 4º-B: vedações (Anexos V/VI, 2º estabelecimento, sócio de outra empresa, startup) e autorizações.
- **78** — § 5º: opção (irretratável; janeiro; início de atividade).
- **79** — §§ 6º–8º e 16: desenquadramento por opção/obrigatório/de ofício, regra dos 20%, multa do art. 36-A.
- **80** — §§ 9º–10: efeitos — regra geral do Simples; excesso ≤ 20% paga diferença sem acréscimos em janeiro.
- **81** — §§ 12 e 15: sem aposentadoria por tempo de contribuição salvo complementação; inadimplência não conta carência.
- **82** — §§ 13, 15-B e 16-A: dispensas de acessórias; cancelamento automático após 12 meses; baixa via portal.
- **83** — § 17: alterações no CNPJ que equivalem a desenquadramento.
- **84** — §§ 18–19-B: cancelamento municipal condicionado; conselhos profissionais (vedações e dispensas).
- **85** — §§ 20–25: nota fiscal gratuita pela internet, guia de turismo, tarifas de concessionárias, trava anti-pejotização (§ 24), residência como sede.
- **86** — 18-B: CPP de 20% do contratante de MEI (rol do § 1º); relação de emprego afasta o regime.
- **87** — 18-C: MEI com um único empregado (SM/piso; retenção; CPP 3%; substituto no afastamento; declaração única).
- **88** — 18-D: IPTU pela menor alíquota no local de residência/atividade.
- **89** — 18-E: política pública; MEI é modalidade de ME; extensão de benefícios; vedação de restrições; MEI rural segurado especial.
- **90** — 18-F: MEI caminhoneiro (251.600; 20.966,67/mês; INSS 12% do SM).

Sem par sentinela novo — o corte da lei já é anunciado pelo par 2 (1ª entrega).

## Notas infraconstitucionais/jurisprudência

- Par 73: ocupações permitidas no Anexo XI da Res. CGSN nº 140/2018.
- Pares 76 e 81: Lei nº 8.212/1991, art. 21, §§ 2º e 3º (alíquotas de 5%/11% e complementação).
- Par 86: Lei nº 8.212/1991, art. 22, III e § 1º (CPP do contratante de contribuinte individual).

## ref/

`lc-123-2006-arts-18a-18f-texto-vigente.md` — texto do bloco por **extração direta do TXT** (reflow mecânico de parágrafos, zero redigitação), atribuições da Câmara preservadas, nota de corte no cabeçalho. Destino: `data/rs/ref/` (família `<id>-*-texto-vigente.md`, pulada pelo copy_ref).

## Blocos restantes do escopo (Fase C)

C2: arts. 19–20 (limpo, gerar direto). C3: arts. 21–27 (7 alterações reais — recuperar redação anterior do art. 26 e excluir 25-A/25-B). C4: arts. 28–32 (limpo). C5: arts. 33–41 (7 alterações, todas no 38-A). C6: Anexos I–V — **desta mesma fonte TXT** (o LEIA-ME confirma: os cinco carregam a redação da LC 155/2016, que é exatamente a pré-corte; Anexo VI = registro de revogação; Anexo VII = pós-reforma, ignorar).
