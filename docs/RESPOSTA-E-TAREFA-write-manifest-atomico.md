# RESPOSTA ao RELATORIO-POS-REVISAO-INCREMENTAL + TAREFA: atomicidade do `write_manifest`

**De:** Claude Fable · **Base:** `main` em `e3fedc1`, com os cinco itens na árvore de trabalho, não
commitados.

Relatório aceito integralmente. Os cinco itens estão bons e a verificação por mutação em cada um —
"um defeito, um teste" — é o que dá confiança de que os testes não são decorativos. Abaixo: onde **eu**
errei, o que fica decidido, e a TAREFA seguinte, que nasceu do seu próprio §9.

---

## 1 — Onde a TAREFA estava errada

Três, e a terceira é a séria.

**A asserção do teste (b) do item 1 era vazia** como especificada. Sua análise está certa, e a
estrutura que ela expôs é melhor que a correção: **a guarda e a remoção da linha defendem cenários
disjuntos** — `Some(h)` divergente só a guarda pega; `None` com árvore em disco só a remoção pega. Por
isso os dois testes ficam, e por isso a mutação derruba exatamente um cada. Registre essa partição no
doc comment se ela ainda não estiver lá: é o que impede alguém de "simplificar" um dos dois no futuro.

**`serde_json::value::IgnoredAny` não existe** — é `serde::de::IgnoredAny`. Erro meu, sem consequência.

**O item 3 estava errado na causa, e o argumento decisivo é estrutural, não empírico.** Numa
regeneração o `vetores_reaproveitaveis` retorna na primeira linha: cache desligado, arquivo nunca
aberto. Os 13,1 GB não podiam vir dali, e eu tirei o número justamente daquela rodada. Deveria ter
visto antes de escrever o item.

E a refutação vai além do item: **ela derruba também a atribuição que eu havia feito na véspera** — a
de que o RS picava em 13,1 GB contra 3,1 do SP por causa do `## resumo` do TARF com 27 mil caracteres.
Se o platô de 10,36 GiB se estabelece aos 21 segundos, nas FAQs, com 1.947 textos curtos, essa
história não se sustenta — e ainda precisaria explicar por que o SP, com número parecido de
documentos, ficou em 3,1 GB. **Não temos causa identificada.** A §10 do relatório está certa em mandar
isso para outra TAREFA, e eu acrescentaria a medição que discrimina (ver §4 aqui).

**Decisões que confirmo:** mantida a mudança do item 3 e, sobretudo, o comentário reescrito — deixar
a atribuição errada faria a próxima pessoa otimizar a coisa errada por anos. Mantidos os dois
comentários extra do §4: é a mesma frase morta que o item existe para matar, e reverter para "ficar
estrito à tabela" seria obedecer à lista em vez de ao propósito dela. E o `built_at`: um carimbo
fabricado seria mentira sobre o único campo que ainda era verdade.

---

## 2 — Antes de qualquer coisa: commite

O binário em `auli-server/target/release/auli` tem código que não corresponde a nenhum commit, e o
processo em execução roda um inode deletado de uma build anterior. Qualquer reinício hoje sobe algo
que não está no histórico. Os cinco itens estão revisados e verdes; commite antes de seguir.

---

## 3 — TAREFA (item 6): `write_manifest` atômico

O seu §9 não é um efeito colateral, é um **defeito encontrado**. A prova está no próprio incidente: na
árvore de hardlinks, os packs sobreviveram porque `write_collection_file` faz `rename`, e o manifesto
não sobreviveu porque `write_manifest` é `fs::write` direto. A assimetria existe, o boot depende dos
dois arquivos, e ela acabou de se manifestar.

```rust
pub fn write_manifest(path: impl AsRef<Path>, manifest: &Manifest) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(manifest)?;
    std::fs::write(path, bytes)?;   // ← não atômico
    Ok(())
}
```

**O que fazer:** `.tmp` no **mesmo diretório** do destino (o `rename` precisa do mesmo sistema de
arquivos) + `rename`, na mesma disciplina do `write_collection_file` (D-INC-5). Se a forma do
`write_collection_file` servir tal e qual, reaproveite-a em vez de reescrever a sequência — mas **não
crie uma abstração** para dois usos; duplicar quatro linhas é melhor que inventar um `escrever_atomico`
genérico em `auli-core` para isto.

**Três ganhos, e vale registrá-los no doc comment:**

1. fecha a janela do "manifesto pela metade" que o comentário do `run_remover` já mencionava como
   possibilidade;
2. transforma a não-atomicidade **entre** pack e manifesto (§5 do relatório) numa janela entre **dois
   estados válidos**, em vez de um estado possivelmente truncado — e é o que faz a recuperação por
   `auli update` ser confiável;
3. torna a isolação por hardlink uma técnica **segura** para o diretório de packs inteiro. Isso é
   utilidade direta: foi a técnica que você precisou usar para medir sem copiar 700 MB, e ela volta a
   ser precisável.

**Teste:** o arquivo final aparece por `rename` e o `.tmp` não sobrevive — espelhando
`escrita_e_atomica_e_nao_deixa_tmp_para_tras` do `vector-store`. E, se for barato de montar, o teste
que reproduz o incidente: **escrever o manifesto através de um hardlink não altera o inode original.**
Esse é o teste que descreve o defeito real; o do `.tmp` descreve a implementação.

**Auditoria delimitada, na mesma leva.** Não saia atrás de todo `fs::write` do repo — há dezenas, e a
maioria é cache de scraper ou fixture de teste, onde escrita parcial é irrelevante. Verifique **só os
arquivos de que o boot ou o `update` dependem para estar íntegros**, e reporte o que achar sem
corrigir: pelo menos `snapshot.rs` (`snapshot.json`) e o `dispositivos-index.json` do `grafo.rs`
merecem um olhar — o segundo já escreve em `.tmp`, o primeiro parece não. Se algum deles for lido no
boot ou consumido pelo `rag`, entra na lista da próxima; se for artefato reconstruível por um comando,
não entra.

**Invariante de escopo:** depois deste item, `auli update` de qualquer entidade continua produzindo
pack **byte a byte idêntico** ao de produção, e o manifesto difere apenas no `built_at`. Nada mais
pode mudar.

---

## 4 — A próxima, para não se perder: a memória do RS

Não é para agora, mas registre o desenho enquanto está fresco, porque a §3 do relatório é o melhor
dado que existe sobre isso.

O que sabemos: platô de **10,36 GiB aos 21 s**, durante a vetorização das FAQs (1.947 textos), que não
cede até o fim; o pipeline inteiro do TARF acrescenta ~620 MiB por cima; e o SP, com número parecido
de documentos, fica em 3,1 GB. Nenhuma hipótese explica os três fatos juntos — nem "resumo grande do
TARF" (o platô o precede), nem "modelo + arena constante" (o SP seria igual).

**A medição que discrimina: rodar cada coleção do `rs` isoladamente**, com a mesma amostragem de
`VmRSS` a cada 200 ms. Se `rs-faqs` sozinha atingir os 10,36 GiB, a causa está no caminho ou nos dados
das FAQs e o TARF é inocente; se não atingir, o platô depende de algo acumulado ao longo da rodada e a
**ordem** das coleções entra como suspeita. Sem isso, qualquer teoria sobre arena do ONNX é narrativa,
e eu já gastei crédito demais nesta conversa com narrativa sobre memória.

---

## 5 — O que continua fora

O desempate do `dedup_por_slug`: intocado, por decisão. A investigação de 10/08 mostrou que o empate de
`dataDocumento` é sinal de **duplicata ou erro de metadado**, não de revisão — trocar o critério
resolveria um problema que não se manifestou e mascararia o sinal. A TAREFA do scraper que existe é
outra: `existe E mesmo GUID ⇒ pula`, mais a verificação "número do corpo × título da listagem" como
anomalia. Ela vem depois da memória do RS.
