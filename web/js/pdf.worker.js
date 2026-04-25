/**
 * PDF Worker - Runs WASM operations in a Web Worker
 * Uses Comlink for clean async communication
 * 
 * Note: PDF rendering is handled by PDF.js on the main thread,
 * not in this worker. PDF manipulation (split, merge, compress) 
 * uses Rust/WASM here.
 * ^^ AI gened explanation ^^ cause i forgot what this did once and messed up for like 2 hrs...
 */

import * as Comlink from 'https://cdn.jsdelivr.net/npm/comlink@4.4.1/+esm';
// WASM = WASSUP MOFO
let wasmModules = {};

async function ensureModule(name) {
    if (!wasmModules[name]) {
        const crateName = name.replace(/_/g, '-');
        const moduleName = name;
        const module = await import(`/wasm/${crateName}/pkg/${moduleName}.js`);
        await module.default();
        wasmModules[name] = module;
    }
    return wasmModules[name];
}


const workerApi = {

    async mergePdfs(pdfBytes) {
        const module = await ensureModule('pdf_merge');
        return module.merge_pdfs(pdfBytes);
    },


    async splitPdfAll(pdfBytes) {
        const module = await ensureModule('pdf_split');
        return module.split_pdf(pdfBytes);
    },

    async splitPdfByRanges(pdfBytes, ranges) {
        const module = await ensureModule('pdf_split');
        return module.split_pdf_by_ranges(pdfBytes, ranges);
    },


    async getPageCount(pdfBytes) {
        const module = await ensureModule('pdf_split');
        return module.get_page_count(pdfBytes);
    },

    async removePages(pdfBytes, pageIndices) {
        const module = await ensureModule('pdf_pages');
        return module.remove_pages(pdfBytes, pageIndices);
    },


    async extractPages(pdfBytes, pageIndices) {
        const module = await ensureModule('pdf_pages');
        return module.extract_pages(pdfBytes, pageIndices);
    },


    async reorderPages(pdfBytes, newOrder) {
        const module = await ensureModule('pdf_pages');
        return module.reorder_pages(pdfBytes, newOrder);
    },


    async compressPdf(pdfBytes, quality) {
        const module = await ensureModule('pdf_compress');
        return module.compress_pdf_with_quality(pdfBytes, quality);
    },

    async compressPdfToTarget(pdfBytes, targetSizeKb) {
        const module = await ensureModule('pdf_compress');
        // Convert KB to bytes
        return module.compress_pdf_to_target(pdfBytes, targetSizeKb * 1024);
    },


    async compressPdfLossless(pdfBytes) {
        const module = await ensureModule('pdf_compress');
        return module.compress_pdf_lossless(pdfBytes);
    },


    async analyzePdf(pdfBytes) {
        const module = await ensureModule('pdf_compress');
        return module.analyze_pdf(pdfBytes);
    },


    isModuleLoaded(moduleName) {
        return !!wasmModules[moduleName];
    },


    async preloadModules(moduleNames) {
        await Promise.all(moduleNames.map(name => ensureModule(name)));
    }
};

Comlink.expose(workerApi);
