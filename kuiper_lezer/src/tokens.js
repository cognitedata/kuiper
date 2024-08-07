import { ExternalTokenizer } from "@lezer/lr";
import * as terms from "./kuiper.grammar.terms";

const CHAR_SINGLE_QOUTE = "'".codePointAt(0);
const CHAR_DOUBLE_QUOTE = "\"".codePointAt(0);
const CHAR_FNUT = "`".codePointAt(0);
const CHAR_SLASH = "/".codePointAt(0);
const CHAR_STAR = "*".codePointAt(0);
const CHAR_BACKSLASH = "\\".codePointAt(0);
const CHAR_NEWLINE = "\n".codePointAt(0)


const makeStringContent = (term, token) => {
    return new ExternalTokenizer((input, stack) => {
        let offset = 0;
        let escaping = false;
        while (true) {
            let c = input.peek(offset);
            if (c === -1) break;
            if (escaping) {
                escaping = false;
            } else if (c === CHAR_BACKSLASH) {
                escaping = true;
            } else if (c === term) {
                if (offset > 0) {
                    input.acceptToken(token, offset);
                }
                return;
            }
            offset = offset + 1;
        }
    });
};

const isBlockCommentStart = (input, offset) => {
    return (
        input.peek(offset) === CHAR_SLASH && input.peek(offset + 1) === CHAR_STAR
    );
};

const isBlockCommentEnd = (input, offset) => {
    return (
        input.peek(offset) === CHAR_STAR && input.peek(offset + 1) === CHAR_SLASH
    );
}

export const blockComment = new ExternalTokenizer((input, stack) => {
    if (isBlockCommentStart(input, 0)) {
        let offset = 2;
        while (true) {
            let c = input.peek(offset);
            if (c === -1) break;
            if (isBlockCommentEnd(input, offset)) {
                input.acceptToken(terms.blockComment, offset + 2);
                return;
            }
            offset = offset + 1;
        }
    }
});

const isLineCommentStart = (input, offset) => {
    return (
        input.peek(offset) === CHAR_SLASH && input.peek(offset + 1) === CHAR_SLASH
    );
};

const isLineCommentEnd = (input, offset) => {
    return (
        input.peek(offset) === CHAR_NEWLINE
    );
}

export const lineComment = new ExternalTokenizer((input, stack) => {
    if (isLineCommentStart(input, 0)) {
        let offset = 2;
        while (true) {
            let c = input.peek(offset);
            if (c === -1) {
                input.acceptToken(terms.lineComment, offset);
                return;
            };
            if (isLineCommentEnd(input, offset)) {
                input.acceptToken(terms.lineComment, offset + 1);
                return;
            }
            offset = offset + 1;
        }
    }
})

export const doubleQuoteStringContent = makeStringContent(CHAR_DOUBLE_QUOTE, terms.doubleQuoteStringContent);
export const singleQuoteStringContent = makeStringContent(CHAR_SINGLE_QOUTE, terms.singleQuoteStringContent);
export const identifierContent = makeStringContent(CHAR_FNUT, terms.identifierContent);
