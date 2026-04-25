// COMLINK MY BELOVED
import * as Comlink from 'https://cdn.jsdelivr.net/npm/comlink@4.4.1/+esm';

let worker = null;
let workerApi = null;
let initPromise = null;


export async function initPdfWorker() {
    if (workerApi) return workerApi;

    if (initPromise) return initPromise;

    initPromise = (async () => {
        worker = new Worker('/js/pdf.worker.js', { type: 'module' });
        workerApi = Comlink.wrap(worker);
        return workerApi;
    })();

    return initPromise;
}

export async function getPdfWorker() {
    if (!workerApi) {
        await initPdfWorker();
    }
    return workerApi;
}

// KILL THE WORKER
export function terminatePdfWorker() {
    if (worker) {
        worker.terminate();
        worker = null;
        workerApi = null;
        initPromise = null;
    }
}

// Helper to copy bytes to avoid ArrayBuffer detachment issues with Comlink
function copyBytes(pdfBytes) {
    return new Uint8Array(pdfBytes);
}

export async function mergePdfs(pdfBytesArray) {
    const api = await getPdfWorker();
    // Copy each PDF's bytes to avoid detachment
    const copiedArray = pdfBytesArray.map(bytes => copyBytes(bytes));
    return api.mergePdfs(copiedArray);
}

export async function splitPdfAll(pdfBytes) {
    const api = await getPdfWorker();
    return api.splitPdfAll(copyBytes(pdfBytes));
}

export async function splitPdfByRanges(pdfBytes, ranges) {
    const api = await getPdfWorker();
    return api.splitPdfByRanges(copyBytes(pdfBytes), ranges);
}

export async function getPageCount(pdfBytes) {
    const api = await getPdfWorker();
    return api.getPageCount(copyBytes(pdfBytes));
}

export async function removePages(pdfBytes, pageIndices) {
    const api = await getPdfWorker();
    return api.removePages(copyBytes(pdfBytes), pageIndices);
}

export async function extractPages(pdfBytes, pageIndices) {
    const api = await getPdfWorker();
    return api.extractPages(copyBytes(pdfBytes), pageIndices);
}

export async function compressPdf(pdfBytes, quality) {
    const api = await getPdfWorker();
    return api.compressPdf(copyBytes(pdfBytes), quality);
}

export async function compressPdfToTarget(pdfBytes, targetSizeKb) {
    const api = await getPdfWorker();
    return api.compressPdfToTarget(copyBytes(pdfBytes), targetSizeKb);
}

export async function compressPdfLossless(pdfBytes) {
    const api = await getPdfWorker();
    return api.compressPdfLossless(copyBytes(pdfBytes));
}

export async function analyzePdf(pdfBytes) {
    const api = await getPdfWorker();
    return api.analyzePdf(copyBytes(pdfBytes));
}


export async function preloadModules(moduleNames) {
    const api = await getPdfWorker();
    return api.preloadModules(moduleNames);
}
