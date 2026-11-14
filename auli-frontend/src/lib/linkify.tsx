import { Fragment } from "react";
import { Link } from "@chakra-ui/react";

// Capturing group so String.prototype.split keeps the URLs as separate parts. A URL também para
// na aspa (reta ou curva): aspa não codificada não pertence a URL, e as descrições de Serviços
// citam endereço assim — `clique aqui "https://…").` (docs/Estudo-servicos-descricao.md, §5.3).
const urlSplitRegex = /(https?:\/\/[^\s"“”]+)/g;
// Separate, non-global matcher for classification — a /g regex is stateful
// across .test() calls and would misclassify parts.
const isUrl = (part: string) => /^https?:\/\//.test(part);

// Sentence punctuation that commonly trails a URL but isn't part of it.
const trailingPunctuation = /[.,;:!?'"]+$/;

const contar = (s: string, c: string) => s.split(c).length - 1;

/** Split trailing sentence punctuation off a matched URL so e.g. "x.com." links
 *  only "x.com" and renders the period as text. A closing ) is only stripped
 *  when it has no matching ( in the URL (so balanced-paren URLs stay intact).
 *  Repete até estabilizar: o fim `);` de `(http://x.gov.br);` precisa das duas
 *  regras alternadas, e uma passada só deixava o `)` no link. */
function splitTrailing(part: string): { url: string; trailing: string } {
  let url = part;
  for (;;) {
    const antes = url;
    if (url.endsWith(")") && contar(url, ")") > contar(url, "(")) url = url.slice(0, -1);
    url = url.replace(trailingPunctuation, "");
    if (url === antes) break;
  }
  return { url, trailing: part.slice(url.length) };
}

export function editLinks(text: string) {
  return text.split(urlSplitRegex).map((part: string, index: number) => {
    if (!isUrl(part)) return part;

    const { url, trailing } = splitTrailing(part);
    const link = (
      <Link
        key={`${index}-${part}`}
        href={url}
        color="accent"
        target="_blank"
        rel="noopener noreferrer"
      >
        {url}
      </Link>
    );

    // Only wrap when there's trailing punctuation to render alongside the link.
    if (!trailing) return link;
    return (
      <Fragment key={`${index}-${part}`}>
        {link}
        {trailing}
      </Fragment>
    );
  });
}
