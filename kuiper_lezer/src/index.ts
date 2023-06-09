import { LRLanguage, LanguageSupport, bracketMatching } from "@codemirror/language";
import {parser} from "./kuiper.grammar"
import {styleTags, tags} from "@lezer/highlight"
import {Completion, completeFromList} from "@codemirror/autocomplete";

export const kuiperLanguage = LRLanguage.define({
    parser: parser.configure({
        props: [styleTags({
            'Integer Float UInteger': tags.number,
            'Var PlainVar': tags.variableName,
            'String': tags.string,
            'Null': tags.null,
            '"{" "}"': tags.brace,
            '"[" "]"': tags.bracket,
            '"(" ")"': tags.paren,
            '"." ":" ","': tags.punctuation,
            '"||" "&&" "==" "!=" ">" "<" ">=" "is" "+" "-" "*" "/" "%" "!" "=>"': tags.operator,
            "BlockComment": tags.blockComment
        })]
    }),
    languageData: {
        closeBrackets: { brackets: ["(", "[", "{", "'", '"', "`"] },
    }
})

const builtIns: KuiperInput[] = [
    { label: "pow", description: "`pow(x, y)`: Return x to the power y" },
    { label: "log", description: "`log(x, y)`: Return the base y logarithm of x" },
    { label: "atan2", description: "`atan2(x, y)`: Return the four quadrant arctangent of (x, y)" },
    { label: "floor", description: "`floor(x)`: Return x rounded down the nearest integer" },
    { label: "ceil", description: "`ceil(x)`: Return x rounded up to the nearest integer" },
    { label: "round", description: "`round(x)`: Round x to the nearest integer. 0.5 is rounded up" },
    { label: "concat", description: "`concat(x, y, ...)`: Concatenate any number of strings" },
    { label: "string", description: "`string(x)`: Convert x to a string, if possible" },
    { label: "int", description: "`int(x)`: Convert x to an integer, if possible" },
    { label: "float", description: "`float(x)`: Convert x to a floating point number, if possible" },
    { label: "if", description: "`if(x, y, (z))`: If x evaluates to true, return y, otherwise return z or null if z is not given" },
    { label: "to_unix_timestamp", description: "`to_unix_timestamp(x, f)`: Convert the string x to a unix timestamp using the format string f" },
    { label: "case", description: "`case(x, c1, r1, c2, r2, ..., (default))`: Compare x to each of c1, c2, etc. and return the matching r1, r2 of the first match. If no entry matches, a final optional expression can be returned as default" },
    { label: "pairs", description: "`pairs(x)`: Convert the object x into a list of objects `{\"key\": ..., \"value\": ...}`" },
    { label: "map", description: "`map(x, it => ...)`: Return the result of the lambda passed as second argument for each entry in the array x" },
    { label: "flatmap", description: "`flatmap(x, it => ...)`: Return the result of the lambda passed as the second argument for each entry in the array x. If the result of the lambda is an array, return each element of that array instead" },
    { label: "filter", description: "`filter(x, it => ...)`: Return only elements where the lambda returns a truthy value" },
    { label: "zip", description: "`zip(x, y, z, ..., (i1, i2, i3, ...) => ...)`: Take a number of arrays, call the given lambda on each entry, and return a single array from the result of each call. The returned array will be as long as the longest argument, null will be given for the shorter input arrays when they run out" },
    { label: "length", description: "`length(x)`: Return the length of the array, string, or object x" },
    { label: "chunk", description: "`chunk(x, s)`: Convert the array x into chunks of at most length s" },
    { label: "now", description: "`now()`: Return the current time in milliseconds since 1/1/1970" },
];

export type KuiperInput = {
    label: string,
    description: string,
};

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
    const completion = kuiperLanguage.data.of({
        autocomplete: completeFromList(
            builtInCompletions.concat(inputCompletions)
        )
    });
    return new LanguageSupport(
        kuiperLanguage,
        [completion]
    )
}
