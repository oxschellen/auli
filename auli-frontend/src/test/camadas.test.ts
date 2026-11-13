import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

/**
 * As regras da árvore de `src/` (docs/ESTUDO-frontend-organizacao.md, "As três regras"), como
 * teste, para que a estrutura não se desfaça com o tempo:
 *
 * 1. A base (`ui/`, `lib/`, `api/`, `contexto/`, `theme/`) não importa de `secoes/` nem de `app/`.
 * 2. Uma seção não importa de outra. O que duas seções precisam sobe para a base. A exceção é a
 *    `home/`, que existe para compor as outras, e os arquivos soltos em `secoes/`.
 *
 * Lê os imports relativos de todo arquivo que não é de teste. O projeto não usa alias: um import
 * entre pastas é sempre relativo, então é tudo o que há para ler.
 */
const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");
const BASE = ["ui", "lib", "api", "contexto", "theme"];
const SECOES = "secoes";
/** Seções que existem para compor as outras. */
const COMPOE = ["home"];

function arquivos(dir: string): string[] {
  return readdirSync(dir).flatMap((nome) => {
    const caminho = join(dir, nome);
    if (statSync(caminho).isDirectory()) return arquivos(caminho);
    return /\.(tsx?|jsx?)$/.test(nome) && !/\.test\./.test(nome) ? [caminho] : [];
  });
}

/** `[arquivo, alvo]` de cada import relativo (estático, `export … from` ou `import()`), os dois
 *  relativos a `src/`. */
function imports(): [string, string][] {
  return arquivos(SRC).flatMap((arq) =>
    [
      ...readFileSync(arq, "utf-8").matchAll(
        /(?:from\s+|import\s*\(\s*)["'](\.{1,2}\/[^"']*)["']/g,
      ),
    ].map(([, alvo]): [string, string] => [
      relative(SRC, arq),
      relative(SRC, join(dirname(arq), alvo)),
    ]),
  );
}

const pasta = (p: string) => p.split(sep)[0];
const secao = (p: string) => (pasta(p) === SECOES ? p.split(sep)[1] : null);

describe("camadas de src/", () => {
  it("lê os imports da árvore (o teste não passa por estar vazio)", () => {
    const todos = imports();
    expect(todos.length).toBeGreaterThan(100);
    expect(todos.some(([de]) => pasta(de) === SECOES)).toBe(true);
    expect(todos.some(([de]) => BASE.includes(pasta(de)))).toBe(true);
  });

  it("a base não importa de seções nem de app/", () => {
    const fora = imports().filter(
      ([de, para]) => BASE.includes(pasta(de)) && [SECOES, "app"].includes(pasta(para)),
    );
    expect(fora).toEqual([]);
  });

  it("uma seção não importa de outra", () => {
    const fora = imports().filter(([de, para]) => {
      const a = secao(de);
      const b = secao(para);
      // `length > 2` deixa de fora os arquivos soltos em secoes/ (um registro de seções, por ex.).
      return a && b && a !== b && !COMPOE.includes(a) && de.split(sep).length > 2;
    });
    expect(fora).toEqual([]);
  });
});
