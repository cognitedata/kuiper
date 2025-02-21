import { spawn } from 'node:child_process';
import { text } from 'node:stream/consumers';
import * as vscode from "vscode";
import { ExtensionContext } from 'vscode';
import * as lc from "vscode-languageclient/node";

export interface KuiperExtensionApi {
    readonly client?: lc.LanguageClient;
}

export class KuiperLanguageClient extends lc.LanguageClient {
}


export class Context implements KuiperExtensionApi {
    private _client: lc.LanguageClient | undefined;
    private _serverPath: string | undefined;

    get client() {
        return this._client;
    }

    constructor(
        readonly extCtx: vscode.ExtensionContext,
    ) {
    }

    private async getOrCreateClient(): Promise<KuiperLanguageClient> {
        if (!this._client) {
            this._serverPath = "/home/einar/projects/kuiper/target/release/kuiper_lsp";
            text(spawn(this._serverPath).stdout.setEncoding("utf-8")).then((data) => {
                console.log(data);
            });
            const run: lc.Executable = {
                command: this._serverPath,
            };
            const serverOptions = {
                run,
                debug: run,
            };

            const clientOptions: lc.LanguageClientOptions = {
                documentSelector: [{ scheme: "file", "language": "kuiper" }],
            };

            this._client = new KuiperLanguageClient(
                "kuiper-lang",
                "Kuiper Language Server",
                serverOptions,
                clientOptions,
            )
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

export async function activate(context: ExtensionContext): Promise<KuiperExtensionApi> {
    const ctx = new Context(context);

    await ctx.start().catch((err) => {
        void vscode.window.showErrorMessage(
            `Cannot activate cognite-kuiper extension: ${err.message}`,
        );
        throw err;
    });
    return ctx;
}

export async function deactivate() {
}