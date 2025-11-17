import { Parser, Tree, Input, PartialParse, TreeFragment, NodeSet, NodeType } from "@lezer/common";
import { defineLanguageFacet, Language, languageDataProp } from "@codemirror/language";

import { syntaxable_entites } from "./pkg/org_wasm"
import { styleTags, tags } from "@lezer/highlight";

export class ParserAdapter extends Parser {
  private buildTree(doc: string): Tree {
    const tokens = syntaxable_entites(doc);

    return Tree.build({
      buffer: Array.from(tokens),
      nodeSet: nodeSet,
      topID: 0,
    });
  }

  createParse(
    input: Input,
    fragments: readonly TreeFragment[],
    ranges: readonly { from: number; to: number }[]
  ): PartialParse {
    return this.startParse(input, fragments, ranges);
  }

  parse(input: string | Input,
    fragments?: readonly TreeFragment[],
    ranges?: readonly { from: number; to: number; }[]): Tree {
    const doc =
      typeof input === "string" ? input : input.read(0, input.length);
    return this.buildTree(doc);
  }

  startParse(
    input: string | Input,
    _fragments?: readonly TreeFragment[] | undefined,
    _ranges?: readonly { from: number; to: number }[] | undefined
  ): PartialParse {
    const doc =
      typeof input === "string" ? input : input.read(0, input.length);

    const tree = this.buildTree(doc);

    return {
      stoppedAt: input.length,
      parsedPos: input.length,
      stopAt: (_) => {},
      advance: () => tree,
    };
  }
}

const nodeSet: NodeSet = new NodeSet([NodeType.define({
  id: 0,
  name: "topNode",
  top: true,
  props: [
    [
      languageDataProp,
      defineLanguageFacet({
        commentTokens: { line: "#" },
      }),
    ],
  ],
}),
NodeType.define({ id: 1, name: "italic" }),
NodeType.define({ id: 2, name: "bold" }),
NodeType.define({ id: 3, name: "entity" }),
NodeType.define({ id: 4, name: "emoji" }),
NodeType.define({ id: 5, name: "target" }),
NodeType.define({ id: 6, name: "macro" }),
NodeType.define({ id: 7, name: "underline" }),
NodeType.define({ id: 8, name: "verbatim" }),
NodeType.define({ id: 9, name: "code" }),
NodeType.define({ id: 10, name: "comment" }),
NodeType.define({ id: 11, name: "inlinesrc" }),
NodeType.define({ id: 12, name: "strikethrough" }),
NodeType.define({ id: 13, name: "plainlink" }),
NodeType.define({ id: 14, name: "exportsnippet" }),
NodeType.define({ id: 15, name: "keyword" }),
NodeType.define({ id: 16, name: "block" }),
NodeType.define({ id: 17, name: "regularlink" }),
NodeType.define({ id: 18, name: "table" }),
NodeType.define({ id: 19, name: "paragraph" }),
NodeType.define({ id: 20, name: "plaintext" }),
NodeType.define({ id: 21, name: "list" }),
NodeType.define({ id: 22, name: "heading" }),
NodeType.define({ id: 23, name: "drawer" }),
NodeType.define({ id: 24, name: "fndef" }),
NodeType.define({ id: 25, name: "fnref" }),
NodeType.define({ id: 26, name: "nada" }),
]).extend(
  // ... means the highglighting applies to all child elements
  styleTags({
    "italic/...": tags.emphasis,
    "bold/...": tags.strong,
    emoji: tags.atom,
    inlinesrc: tags.className,
    comment: tags.comment,
    "strikethrough/...": tags.strikethrough,
    macro: tags.macroName,
    plainlink: tags.link,
    target: tags.labelName,
    heading: tags.heading,
    keyword: tags.annotation,
    block: tags.definitionKeyword,
    exportsnippet: tags.namespace,
    entity: tags.bool,
    "regularlink/...": tags.link,
    fndef: tags.typeName,
    fnref: tags.variableName,
  })
);

import { LanguageSupport } from "@codemirror/language";
import { Facet } from "@codemirror/state";

const newParser = new ParserAdapter();
export const orgLanguage = new Language(
  Facet.define(),
  newParser,
  [],
  "Org-Mode"
);

export const org = new LanguageSupport(orgLanguage);
