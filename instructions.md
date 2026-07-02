# Push instructions — `fix/consistency-audit`

The consistency-audit changes are **committed locally but not yet pushed** (the automated
environment couldn't complete GitHub authentication). Everything below is ready to go — you just need
to run the push from a terminal where you're logged into GitHub.

## Current state

- **Repo:** `https://github.com/oxschellen/auli.git` (remote `origin` already configured)
- **Branch:** `fix/consistency-audit` (created off `origin/main`)
- **Commit:** `c1340a8` — "Fix cross-cutting inconsistencies from full-codebase audit"
- **Contents:** 33 files (32 modified + `update.md` added). No build artifacts or secrets —
  `.gitignore` keeps `target/`, `node_modules/`, and `.env` out.
- **Author:** Carlos Schellenberger `<carlos.schellenberger@gmail.com>` (set repo-locally only;
  global git config was not touched).

## Push it

From the project root (`auli-main/`):

```bash
git push -u origin fix/consistency-audit
```

The first push will prompt for GitHub login (browser or Personal Access Token). If it hangs with no
prompt, your credential helper isn't set up — see Troubleshooting below.

## Open a pull request

Do **not** push straight to `main` — it has active PRs (#1–#3). Open a PR from the branch instead.

With the GitHub CLI:

```bash
gh pr create --fill --base main --head fix/consistency-audit
```

Or open it in the browser: after pushing, GitHub prints a "Create a pull request" URL, or go to
`https://github.com/oxschellen/auli/compare/main...fix/consistency-audit`.

## Verify before/after

```bash
git log --oneline -1                 # should show c1340a8
git diff --stat origin/main          # the 33 changed files
git ls-remote --heads origin fix/consistency-audit   # empty until pushed; shows a SHA after
```

## Re-run the checks (optional)

```bash
# Rust (from auli-server/)
cargo test -p auli-core -p auli-cli -p auli-collections -p auli-contract -p auli-scraper-kit

# Frontend (from auli-frontend/)
npm run typecheck
npm test
```

All of these passed at commit time (Rust: all green, 1 gated smoke test ignored by design;
frontend: type-check clean, 52 tests pass).

## Troubleshooting

- **Push hangs / no login prompt:** no credential helper is configured. Either:
  - install & auth the GitHub CLI: `gh auth login` (then retry the push), or
  - configure the manager: `git config --global credential.helper manager`, or
  - use a Personal Access Token as the password when prompted (create one at
    GitHub → Settings → Developer settings → Personal access tokens, scope `repo`).
- **`src refspec ... does not match`:** you're not on the branch — `git checkout fix/consistency-audit`.
- **Line endings:** git warned about LF→CRLF on Windows. Content is unchanged. If the repo standard
  is LF, consider adding a `.gitattributes` (`* text=auto eol=lf`) before merging.

## Notes carried over from the audit (`update.md`)

- Items still open (not in this commit): #10 (provisional service embed formula), #11 (duplicate RS
  scraper caches), plus sub-parts of #12/#16 and the broader #14 note about `data/prompts/*.txt`.
- Data: no pack regeneration needed for this repo (packs are generated, none committed). But any packs
  built **before** the `services -> servicos` rename need a one-time `auli update` re-run, since the
  server now loads `<id>-servicos.json`.
