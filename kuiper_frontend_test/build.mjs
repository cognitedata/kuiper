#!/usr/bin/env node
import * as esbuild from 'esbuild'
import { wasmLoader } from 'esbuild-plugin-wasm'

let ctx = await esbuild.context({
    entryPoints: ['src/app.tsx'],
    bundle: true,
    outdir: "www/js",
    plugins: [wasmLoader()],
    format: 'esm'
});

ctx.serve
let { host, port } = await ctx.serve({ servedir: "www/" })
console.log("Running server on http://localhost:" + port);
