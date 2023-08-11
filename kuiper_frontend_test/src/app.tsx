import * as React from 'react'
import CodeMirror from '@uiw/react-codemirror';
import { createRoot } from 'react-dom/client';
import { okaidia } from '@uiw/codemirror-theme-okaidia';
import { createTheme } from '@uiw/codemirror-themes'
import { tags as t } from '@lezer/highlight';
import {linter, Diagnostic} from "@codemirror/lint"
import { EditorView } from "@codemirror/view"
import { syntaxTree } from "@codemirror/language"
import { json } from "@codemirror/lang-json"

const kuiperTheme = createTheme({
    theme: 'dark',
    settings: {
        background: '#333333',
        foreground: '#ffffff',
        caret: '#5d00ff',
        selection: '#036dd626',
        selectionMatch: '#036dd626',
        lineHighlight: '#8a91991a',
        gutterBackground: '#fff',
        gutterForeground: '#8a919966',
    },
    styles: [
        { tag: t.number, color: '#5c6166' },
        { tag: t.null, color: '#5c6166' },
        { tag: t.brace, color: '#FFEA94' },
        { tag: t.bracket, color: '#FFEA94' },
        { tag: t.paren, color: '#FFEA94' },
        { tag: t.punctuation, color: '#FFFFFF' },
        { tag: t.operator, color: '#FFFFFF' },
        { tag: t.blockComment, color: '#475243' },
        { tag: t.string, color: '#A65824' },
        { tag: t.variableName, color: '#CDF6FF' }
    ]
});

function lintTest(view: EditorView): Diagnostic[] {
    const diagnostics: Diagnostic[] = [];

    syntaxTree(view.state).iterate({
        enter: (node) => {
            if (node.type.isError) {
                diagnostics.push({
                    from: node.from,
                    to: node.to,
                    severity: "error",
                    message: "Syntax error"
                })
            }
        }
    });

    return diagnostics;
}


/* test */
import { kuiper } from "codemirror-lang-kuiper"
import { compile_expression, KuiperError, KuiperExpression } from '@cognite/kuiper_js';

function App() {
    const lang = kuiper([{
        label: "input",
        description: "Input value"
    }, {
        label: "context",
        description: "Context"
    }]);



    const [ expression, setExpression ] = React.useState<KuiperExpression | undefined>();
    const [ sampleData, setSampleData ] = React.useState<string | undefined>("{}");
    const lintReal = (view: EditorView): Diagnostic[] => {
        const data = view.state.doc.toString();
        let expr: KuiperExpression | undefined = undefined;
        try {
            expr = compile_expression(data, ["input"]);
        }
        catch (err) {
            if (err instanceof KuiperError) {
                const diagnostics: Diagnostic[] = [];
                if (err.start !== undefined && err.end !== undefined) {
                    diagnostics.push({
                        from: err.start,
                        to: err.end,
                        severity: "error",
                        message: err.message
                    });
                }
                err.free();
                return diagnostics;
            }
            return [];
        }

        if (!sampleData) return [];

        try {
            expr.run(JSON.parse(sampleData));
        } catch (err) {
            if (err instanceof KuiperError) {
                const diagnostics: Diagnostic[] = [];
                if (err.start !== undefined && err.end !== undefined) {
                    diagnostics.push({
                        from: err.start,
                        to: err.end,
                        severity: "error",
                        message: err.message
                    });
                }
                err.free();
                return diagnostics;
            }
        } finally {
            expr.free();
        }
        return [];
    }
    const onChange = React.useCallback((value: string, viewUpdate) => {
        let expr: KuiperExpression | undefined = undefined;
        try {
            expr = compile_expression(value, ["input"]);
        }
        catch (err) {
            if (err instanceof KuiperError) {
                console.log("Failed to compile: " + err.message + ", " + err.start + ":" + err.end);
                err.free()
            } else {
                console.log("Unexpected error during compile: " + err)
            }
            return;
        }
        if (expression) {
            expression.free();
        }
        setExpression(expr);
    }, []);

    const jsonLang = json();
    const onChangeSample = React.useCallback((value: string, viewUpdate) => {
        try {
            JSON.parse(value);
        } catch (err) {
            return;
        }
        setSampleData(value);
    }, []);

    let output: string | undefined = undefined;
    if (expression && sampleData) {
        try {
            let res = expression.run(JSON.parse(sampleData));
            output = JSON.stringify(res, undefined, 4);
        } catch (err) {
            if (err instanceof KuiperError) {
                console.log("Failed to transform: " + err.message + ", " + err.start + ":" + err.end);
                err.free()
            } else {
                console.log("Unexpected error during run: " + err)
            }
        }
    }

    return (
    <div>
        <CodeMirror
            value="{}"
            height='200px'
            theme={okaidia}
            extensions={[jsonLang]}
            onChange={onChangeSample}
        />
        <CodeMirror
            value=""
            height="200px"
            theme={okaidia}
            extensions={[lang, linter(lintReal)]}
            onChange={onChange}
        />
        <span>{output}</span>
    </div>

    );
}

const container = document.getElementById('root')!;
const root = createRoot(container); // createRoot(container!) if you use TypeScript
root.render(<App />);
