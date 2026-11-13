import { Fragment } from "react";
import { Link } from "@chakra-ui/react";

// Capturing group so String.prototype.split keeps the URLs as separate parts.
const urlSplitRegex = /(https?:\/\/[^\s]+)/g;
// Separate, non-global matcher for classification — a /g regex is stateful
// across .test() calls and would misclassify parts.
const isUrl = (part: string) => /^https?:\/\//.test(part);

// Sentence punctuation that commonly trails a URL but isn't part of it.
const trailingPunctuation = /[.,;:!?'"]+$/;

/** Split trailing sentence punctuation off a matched URL so e.g. "x.com." links
 *  only "x.com" and renders the period as text. A closing ) is only stripped
 *  when the URL has no opening ( (so balanced-paren URLs stay intact). */
function splitTrailing(part: string): { url: string; trailing: string } {
  let url = part;
  let trailing = "";

  if (url.endsWith(")") && !url.includes("(")) {
    url = url.slice(0, -1);
    trailing = ")";
  }

  const match = url.match(trailingPunctuation);
  if (match) {
    trailing = match[0] + trailing;
    url = url.slice(0, -match[0].length);
  }

  return { url, trailing };
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
