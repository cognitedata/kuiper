import { Completion, completeFromList, ifNotIn } from "@codemirror/autocomplete";
import { LRLanguage, LanguageSupport, delimitedIndent, foldInside, foldNodeProp, indentNodeProp } from "@codemirror/language";
import { styleTags, tags } from "@lezer/highlight";
import { KuiperInput, builtIns } from "./builtins";
import { dontComplete, varCompletionSource } from "./complete";
import { parser } from "./kuiper.grammar";

export { KuiperInput };

export const kuiperLanguage = LRLanguage.define({
    name: "kuiper",
    parser: parser.configure({
        props: [styleTags({
            'Number': tags.number,
            'Variable/...': tags.variableName,
            'FunctionName/...': tags.function(tags.variableName),
            'String': tags.string,
            'Null': tags.null,
            '\"{\" \"}\"': tags.brace,
            '\"[\" \"]\"': tags.bracket,
            '\"(\" \")\"': tags.paren,
            '\":\"': tags.punctuation,
            "\".\"": tags.derefOperator,
            "\",\"": tags.separator,
            "Boolean/...": tags.bool,
            "BlockComment": tags.blockComment,
            "LineComment": tags.lineComment,
            "CompareOp/...": tags.compareOperator,
            "ArithOp/...": tags.arithmeticOperator,
            "LogicOp/...": tags.logicOperator,
            "Arrow": tags.function(tags.punctuation),
            "Type/...": tags.typeName,
        }), indentNodeProp.add({
            Object: delimitedIndent({ closing: "}" }),
            Array: delimitedIndent({ closing: "]" }),
            Lambda: cx => cx.baseIndent + cx.unit
        }), foldNodeProp.add({
            "Object Array": foldInside,
            BlockComment(tree) { return { from: tree.from + 2, to: tree.to - 2 } }
        })]
    }),
    languageData: {
        closeBrackets: { brackets: ["(", "[", "{", "'", '"', "`"] },
        commentTokens: { block: { open: "/*", close: "*/" } }
    }
})

export function kuiper(inputs: KuiperInput[] = []) {
    const builtInCompletions: Completion[] = builtIns.map(func => ({
        label: func.label,
        detail: func.description,
        type: "function"
    }));
    const inputCompletions: Completion[] = inputs.map(inp => ({
        label: inp.label,
        detail: inp.description,
        type: "variable"
    }));
    const buildInCompletion = kuiperLanguage.data.of({
        autocomplete: ifNotIn(dontComplete, completeFromList(
            builtInCompletions
        ))
    });
    const varCompletion = kuiperLanguage.data.of({
        autocomplete: varCompletionSource(inputCompletions)
    });

    return new LanguageSupport(
        kuiperLanguage,
        [buildInCompletion, varCompletion]
    )
}
