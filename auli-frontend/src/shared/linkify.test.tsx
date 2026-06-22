import { describe, it, expect } from "vitest";
import { isValidElement, Fragment, type ReactElement, type ReactNode } from "react";
import { editLinks } from "./linkify";

/** First link element in the output, unwrapping a Fragment (link + trailing). */
function firstLink(parts: ReactNode[]): ReactElement<{ href: string }> | undefined {
  for (const p of parts) {
    if (!isValidElement(p)) continue;
    if (p.type === Fragment) {
      const kids = (p.props as { children?: ReactNode }).children;
      const arr = Array.isArray(kids) ? kids : [kids];
      const link = arr.find((k) => isValidElement(k));
      if (link) return link as ReactElement<{ href: string }>;
    } else {
      return p as ReactElement<{ href: string }>;
    }
  }
  return undefined;
}

describe("editLinks", () => {
  it("returns plain text untouched as a single string part", () => {
    const parts = editLinks("nenhum link aqui");
    expect(parts).toEqual(["nenhum link aqui"]);
  });

  it("splits a URL out into a React link element, keeping the surrounding text", () => {
    const parts = editLinks("veja https://exemplo.com agora");
    expect(parts).toHaveLength(3);
    expect(parts[0]).toBe("veja ");
    expect(parts[2]).toBe(" agora");

    const link = parts[1] as ReactElement<{
      href: string;
      target: string;
      rel: string;
      children: string;
    }>;
    expect(isValidElement(link)).toBe(true);
    expect(link.props.href).toBe("https://exemplo.com");
    expect(link.props.target).toBe("_blank");
    expect(link.props.rel).toBe("noopener noreferrer");
    expect(link.props.children).toBe("https://exemplo.com");
  });

  it("links multiple URLs in the same text", () => {
    const parts = editLinks("a http://x.com b https://y.com") as ReactNode[];
    const links = parts.filter(
      (p): p is ReactElement<{ href: string }> => isValidElement(p),
    );
    expect(links.map((l) => l.props.href)).toEqual(["http://x.com", "https://y.com"]);
  });

  it("does not treat bare www/text as a link", () => {
    const parts = editLinks("contato www.exemplo.com") as ReactNode[];
    expect(parts.some((p) => isValidElement(p))).toBe(false);
  });

  it("strips a trailing period from the linked URL", () => {
    const parts = editLinks("veja https://exemplo.com.") as ReactNode[];
    expect(firstLink(parts)?.props.href).toBe("https://exemplo.com");
  });

  it("strips a trailing comma", () => {
    const parts = editLinks("a https://x.com, b") as ReactNode[];
    expect(firstLink(parts)?.props.href).toBe("https://x.com");
  });

  it("strips a trailing close paren only when the URL has no open paren", () => {
    expect(firstLink(editLinks("(veja https://x.com)") as ReactNode[])?.props.href).toBe(
      "https://x.com",
    );
    // Balanced-paren URL (e.g. Wikipedia) keeps its closing paren.
    expect(
      firstLink(editLinks("ref https://w.org/wiki/Foo_(bar)") as ReactNode[])?.props.href,
    ).toBe("https://w.org/wiki/Foo_(bar)");
  });
});
