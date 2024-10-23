"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.deactivate = exports.activate = exports.Context = exports.KuiperLanguageClient = void 0;
const node_child_process_1 = require("node:child_process");
const consumers_1 = require("node:stream/consumers");
const vscode = require("vscode");
const lc = require("vscode-languageclient/node");
class KuiperLanguageClient extends lc.LanguageClient {
}
exports.KuiperLanguageClient = KuiperLanguageClient;
class Context {
    get client() {
        return this._client;
    }
    constructor(extCtx) {
        this.extCtx = extCtx;
    }
    async getOrCreateClient() {
        if (!this._client) {
            this._serverPath = "/home/einar/projects/kuiper/target/release/kuiper_lsp";
            (0, consumers_1.text)((0, node_child_process_1.spawn)(this._serverPath).stdout.setEncoding("utf-8")).then((data) => {
                console.log(data);
            });
            const run = {
                command: this._serverPath,
            };
            const serverOptions = {
                run,
                debug: run,
            };
            const clientOptions = {
                documentSelector: [{ scheme: "file", "language": "kuiper" }],
            };
            this._client = new KuiperLanguageClient("kuiper-lang", "Kuiper Language Server", serverOptions, clientOptions);
        }
        return this._client;
    }
    async start() {
        const client = await this.getOrCreateClient();
        if (!client) {
            return;
        }
        await client.start();
    }
    async stop() {
        if (!this._client) {
            return;
        }
        await this._client.stop();
    }
}
exports.Context = Context;
async function activate(context) {
    console.log("Debug please");
    vscode.window.showErrorMessage("Test");
    const ctx = new Context(context);
    await ctx.start().catch((err) => {
        void vscode.window.showErrorMessage(`Cannot activate cognite-kuiper extension: ${err.message}`);
        throw err;
    });
    return ctx;
}
exports.activate = activate;
async function deactivate() {
}
exports.deactivate = deactivate;
//# sourceMappingURL=extension.js.map