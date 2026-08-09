#!/usr/bin/env python3
"""Resumo de uma linha por entidade dos packs em disco — o que o `publicar.sh` mostra no portão.

Arquivo próprio, e não um `python3 -c` embutido no shell: a versão inline precisava de três níveis
de aspas aninhadas (bash → python → f-string) e quebrava ao menor ajuste. Aqui dá para ler e testar.

Só lê; nunca escreve. Falha de leitura em uma entidade não derruba as outras — o portão existe para
informar a decisão, então um manifesto ilegível vira uma linha dizendo isso, não um traceback.
"""

import json
import pathlib
import sys


def main() -> int:
    raiz = pathlib.Path(__file__).resolve().parent.parent / "data"
    manifestos = sorted(raiz.glob("*/packs/*.manifest.json"))
    if not manifestos:
        print("   packs: nenhum manifesto em data/*/packs/")
        return 0

    for m in manifestos:
        entidade = m.parent.parent.name
        try:
            d = json.loads(m.read_text(encoding="utf-8"))
            partes = " ".join(f"{c['kind']}={c['count']}" for c in d["collections"])
            print(f"   packs {entidade}: {partes} · {d['built_at'][:10]}")
        except (OSError, ValueError, KeyError) as e:
            print(f"   packs {entidade}: ilegível ({type(e).__name__})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
