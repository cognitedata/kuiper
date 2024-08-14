import { syntaxTree } from "@codemirror/language";
import { KuiperInput } from "./builtins";

import { Extension } from "@codemirror/state";
import { hoverTooltip } from "@codemirror/view";
import { dontComplete } from "./complete";

export function getHoverTooltip(builtIns: KuiperInput[]): Extension {
    const builtInsDict: { [label: string]: string } = {}
    for (const builtIn of builtIns) {
        builtInsDict[builtIn.label] = builtIn.description;
    }
    return hoverTooltip((view, pos, side) => {
        let inner = syntaxTree(view.state).resolveInner(pos, -1);
        if (dontComplete.indexOf(inner.name) > -1) return null;
        let isWord = inner.name == "Var" || inner.name == "PlainVar";
        if (!isWord) return null;

        let { from, to, text } = view.state.doc.lineAt(pos);
        let start = pos, end = pos
        while (start > from && /\w/.test(text[start - from - 1])) start--
        while (end < to && /\w/.test(text[end - from])) end++
        if (start == pos && side < 0 || end == pos && side > 0)
            return null;
        const key = text.slice(start - from, end - from);
        if (!(key in builtInsDict)) {
            return null;
        }
        const desc = builtInsDict[key];
        return {
            pos: start,
            end,
            above: true,
            create: (_v) => {
                let dom = document.createElement("div");
                dom.textContent = desc;
                dom.className = ".cm-tooltip.cm-tooltip-hover";
                return { dom };
            }
        }
    });
}