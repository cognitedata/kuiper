import * as React from 'react'
import CodeMirror from '@uiw/react-codemirror';
import { createRoot } from 'react-dom/client';
import { okaidia } from '@uiw/codemirror-theme-okaidia';
import { createTheme } from '@uiw/codemirror-themes'
import { tags as t } from '@lezer/highlight';
import {linter, Diagnostic} from "@codemirror/lint"
import { EditorView } from "@codemirror/view"
import { syntaxTree } from "@codemirror/language"

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

function App() {
    const lang = kuiper([{
        label: "input",
        description: "Input value"
    }, {
        label: "context",
        description: "Context"
    }]);
    const onChange = React.useCallback((value: string, viewUpdate) => {
        console.log('value:', lang.language.parser.parse(value).topNode);
      }, []);

      console.log(lang.language.name, lang.extension);
      return (
        <CodeMirror
          value=""
          height="200px"
          theme={kuiperTheme}
          extensions={[lang, linter(lintTest)]}
          onChange={onChange}

        />
      );
}

const container = document.getElementById('root')!;
const root = createRoot(container); // createRoot(container!) if you use TypeScript
root.render(<App />);
