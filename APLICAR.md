# Como aplicar as mudanças desta sessão

Dois artefatos foram produzidos:

1. **`auli-comment-cleanup.patch`** — corrige 3 comentários/docstrings desatualizados
   (`ratelimit.rs`: ngrok→Cloudflare Tunnel; `main.rs`: `portal-*.txt`→contrato tipado;
   `error.rs`: `DimensionMismatch` deixa de ser "Reserved for Phase 5").
2. **`rename-engine.sh`** — renomeia o workspace `auli/` → `auli-engine/` (com `git mv`,
   preservando histórico) e atualiza todas as referências (scripts + docs) na raiz.

> ⚠️ **A ORDEM IMPORTA.** Aplique o **patch ANTES** do rename. Depois do rename os arquivos
> mudam de pasta (`auli/...` → `auli-engine/...`) e o patch não os encontraria mais.

Rode tudo no **terminal**, na pasta-raiz do repositório (a que contém `auli/`,
`auli-frontend/` e os `auli_*.md` — provavelmente `~/Desktop/auli`).

---

## Passo 0 — Colocar os dois arquivos na raiz do repo
Baixe `auli-comment-cleanup.patch` e `rename-engine.sh` e mova-os para a raiz do repositório
(a mesma pasta dos passos abaixo).

## Passo 1 — Entrar na pasta e conferir que está tudo limpo
```bash
cd ~/Desktop/auli            # ajuste o caminho se for outro
git status
```
Espere ver "nothing to commit, working tree clean". Se houver alterações suas pendentes,
faça commit ou guarde antes de continuar.

## Passo 2 — Criar uma branch de segurança
Assim, se algo der errado, é só jogar a branch fora — o `main` fica intacto.
```bash
git switch -c chore/cleanup-and-engine-rename
```

## Passo 3 — Aplicar o patch dos comentários (PRIMEIRO)
```bash
git apply --check auli-comment-cleanup.patch   # ensaio: só verifica, não muda nada
git apply auli-comment-cleanup.patch           # aplica de verdade
```
Se o `--check` reclamar, **pare** e não rode o segundo comando.

## Passo 4 — Commit do patch
```bash
git add -A
git commit -m "docs: corrige comentários desatualizados (ngrok, portal-txt, Phase 5)"
```

## Passo 5 — Rodar o rename (DEPOIS do patch)
```bash
bash rename-engine.sh
```
Ele faz o `git mv auli auli-engine` e ajusta as referências. As mudanças já ficam preparadas
("staged") para commit.

## Passo 6 — Conferir e commitar o rename
```bash
git status                  # deve mostrar ~68 renomeados + 8 modificados
git commit -m "refactor: renomeia workspace auli/ -> auli-engine/ e atualiza referências"
```

## Passo 7 — Testar que ainda compila
```bash
cd auli-engine && cargo build --release --bin auli && cd ..
```

## Passo 8 — Levar para o `main` e enviar ao GitHub
```bash
git switch main                              # volta para o main
git merge chore/cleanup-and-engine-rename    # traz as mudanças
git push origin main                         # envia ao GitHub
```

---

## 🆘 Botão de pânico
A qualquer momento **antes do Passo 8**, para descartar tudo e voltar ao estado original:
```bash
git switch main
git branch -D chore/cleanup-and-engine-rename
```
Isso apaga a branch inteira; o `main` continua exatamente como estava.

## Sanidade (opcional, após o rename)
Não deve sobrar nenhuma referência ao diretório antigo:
```bash
grep -rn --include='*.sh' --include='*.md' -E 'ROOT/auli/|cd auli |\(auli/crates' . | grep -v auli-engine
```
(Saída vazia = tudo certo.)
