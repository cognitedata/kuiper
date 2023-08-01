import { Completion, CompletionContext, CompletionResult, CompletionSource } from "@codemirror/autocomplete";
import { syntaxTree } from "@codemirror/language";
import { SyntaxNode, SyntaxNodeRef } from "@lezer/common";

const scopeNodes = new Set([
    "Lambda"
]);

const gatherCompletions: {
    [node: string]: (node: SyntaxNodeRef, def: (node: SyntaxNodeRef, type: string) => void) => void | boolean
} = {
    
}

export const dontComplete = [
    "String", "BlockComment", "."
]

const Identifier = /^[\w$\xa1-\uffff][\w$\d\xa1-\uffff]*$/

function isInSelector(node: SyntaxNode): boolean {
    if (node == null) return false;
    if (node.name == "Selector") return true;
    if (node.name == "Term" || node.name == "Var" || node.name == "PlainVar") {
        return isInSelector(node.parent);
    }
    return false;
}

export function varCompletionSource(sources: Completion[]): CompletionSource {
    return (context: CompletionContext): CompletionResult | null => {
        let inner = syntaxTree(context.state).resolveInner(context.pos, -1);
        if (dontComplete.indexOf(inner.name) > -1) return null;
        if (isInSelector(inner)) return null;
        let isWord = inner.name == "Var" || inner.name == "PlainVar"
            || inner.to - inner.from < 20 && Identifier.test(context.state.sliceDoc(inner.from, inner.to));
        if (!isWord && !context.explicit) return null;
        let options: Completion[] = [];
        for (let pos: SyntaxNode | null = inner; pos; pos = pos.parent) {
            if (pos.name == "Lambda") {
                for (let child of pos.getChildren("Var", null, null)) {
                    let text = context.state.sliceDoc(child.from, child.to);
                    if (text.startsWith("`")) text = text.slice(1, -1);
                    options.push({ label: text, type: "variable" });
                }
            }
        }
        return {
            options: options.concat(sources),
            from: isWord ? inner.from : context.pos,
            validFor: Identifier
        }
    }
}
