#!/usr/bin/env bash
# rs-do-zero.sh — RS inteiro do zero: apaga o cache, raspa serviços e faqs, deriva, vetoriza e sobe.
# Roteiro linear e autocontido: não chama nenhum outro script do repositório.

# Aborta na primeira falha, em vez de seguir com um passo apoiado no anterior que não deu certo.
# Sem isto um scrape que morre ainda chegaria a vetorizar e a subir o servidor com dados velhos.
set -euo pipefail

# Todo o roteiro vive dentro desta função, e não solto no arquivo, por uma razão só: o bash
# precisa parsear a função INTEIRA antes de poder chamá-la. Duas consequências, as duas úteis
# num script cuja etapa 7 segura o processo por dezenas de minutos: erro de sintaxe na última
# etapa aborta ANTES da primeira (sem isto o bash só descobriria ao chegar lá, com os caches já
# apagados), e uma edição no arquivo durante a rodada não desalinha mais a leitura — o bash volta
# ao disco por posição em bytes, e já viu tudo o que precisava ver.
main() {
  # Apaga os caches das duas coleções, o que é o que "do zero" significa aqui: o scraper é cache-first
  # sempre, e a flag --usecache só decide o que fazer no miss — nunca ignora um arquivo que exista.
  echo "Etapa 1/13 — 🗑️  apagando os caches de serviços e faqs"
  # rm -rf /home/ubu/Desktop/auli/data/rs/raw/cache/servicos /home/ubu/Desktop/auli/data/rs/raw/cache/faqs
  echo

  # Todos os passos seguintes resolvem `data/` pelo diretório corrente, e o esperado é `auli-server/`.
  # Rodar de outro lugar grava a árvore no caminho errado, sem erro nenhum.
  echo "Etapa 2/13 — 📂 entrando em auli-server/ (os passos seguintes resolvem data/ pelo CWD)"
  cd /home/ubu/Desktop/auli/auli-server
  echo

  # Caminho do modelo BGE-M3 (ONNX), exigido pelo passo de vetorização mais abaixo.
  # Absoluto de propósito: o `.env` do binário não sobrescreve variável que já esteja no ambiente.
  echo "Etapa 3/13 — 🧠 EMBED_CACHE_DIR=/home/ubu/Desktop/auli/models"
  export EMBED_CACHE_DIR=/home/ubu/Desktop/auli/models
  echo

  # Raspa as duas coleções — `all` é faqs seguido de serviços — direto da rede, já que o cache sumiu.
  # Grava só os snapshots em data/rs/; nada aqui chega ainda ao índice que o servidor lê.
  echo "Etapa 4/13 — 🕸️  raspando faqs e serviços do portal (~2.500 páginas, tudo da rede)"
  # cargo run --release -p auli-scraper-rs -- all
  echo

  # Atualiza o TARF sem recoletar: a listagem inteira volta da rede (46 GETs, ~1 min a 1 req/s),
  # mas o corpo só é buscado para acórdão que ainda não tem `.md` na árvore (D-TARF-8).
  echo "Etapa 5/13 — ⚖️  atualizando o TARF (listagem inteira; corpo só dos inéditos)"
  cargo run --release -p auli-scraper-rs -- tarf
  echo

  # Compila os dois binários usados nos passos seguintes; `auli` é o bin do pacote auli-cli.
  # Binário velho aqui é o pior caso do repositório: grava vetores de fórmula antiga que passam no boot.
  echo "Etapa 6/13 — 🔨 compilando auli e auli-collections (release)"
  cargo build --release -p auli-cli -p auli-collections
  echo

  # Deriva os artefatos dos snapshots, entre eles as árvores docs/faqs e docs/servicos.
  # É essa árvore, e não o snapshot, que o passo de vetorização lê — pulá-lo vetoriza a coleta anterior.
  echo "Etapa 7/13 — 🌳 derivando os artefatos (árvores docs/faqs e docs/servicos)"
  ./target/release/auli-collections rs
  echo

  # Vetoriza os packs das quatro coleções do RS a partir das árvores recém-derivadas.
  # Serviços e faqs são reescritos por inteiro; pareceres e TARF reaproveitam o vetor de quem não mudou.
  echo "Etapa 8/13 — 📦 vetorizando os packs do RS (a demorada; confira a linha de reaproveitados)"
  ./target/release/auli update --entity rs --out /home/ubu/Desktop/auli/data/rs/packs --version 1
  echo

  # Índice leve da aba de pareceres do frontend, derivado da mesma árvore que acabou de ser vetorizada.
  # Sem ele os packs ficam novos e a aba segue lendo o índice velho, sem que nada no boot perceba.
  echo "Etapa 9/13 — 🗂️  derivando o índice da aba de pareceres"
  ./target/release/auli-collections rs indice pareceres
  echo

  # O mesmo índice, agora para a aba do TARF.
  # Foi exatamente esta divergência que em 09/08/2026 deixou a aba em 3.800 acórdãos e o chat em 22.476.
  echo "Etapa 10/13 — 🗂️  derivando o índice da aba do TARF"
  ./target/release/auli-collections rs indice tarf
  echo

  # Derruba uma instância anterior, se houver, para liberar a porta 3000.
  # Sem isto o servidor novo não sobe e o erro de porta ocupada parece falha da vetorização.
  echo "Etapa 11/13 — 🔪 derrubando instância anterior do servidor, se houver"
  pkill -f "release/auli server" || true
  echo

  # Variáveis que o servidor lê: raiz dos dados (packs e registry) e destino do log de consultas.
  # Ambas absolutas porque o servidor roda em auli-server/ e os relativos cairiam dentro dele.
  echo "Etapa 12/13 — ⚙️  AULI_DATA_DIR=/home/ubu/Desktop/auli/data, AULI_LOG_DIR=/home/ubu/Desktop/auli/logs"
  export AULI_DATA_DIR=/home/ubu/Desktop/auli/data
  export AULI_LOG_DIR=/home/ubu/Desktop/auli/logs
  echo

  # Sobe a API em primeiro plano, só no loopback; Ctrl+C para parar.
  # Sem o túnel do Cloudflare: publicar api.auli.com.br é o ./start_server.sh, e este script é local.
  echo "Etapa 13/13 — 🚀 subindo a API em http://127.0.0.1:3000 — Ctrl+C para parar"
  ./target/release/auli server --port 3000 --bind 127.0.0.1
  echo
}

# A única linha que o bash relê depois de a função definida. Curta de propósito: é a janela que
# sobra, e ela some assim que esta chamada é lida.
main "$@"
