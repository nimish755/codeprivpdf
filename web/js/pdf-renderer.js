
// YOINK
const PDFJS_CDN = 'https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/';
let pdfjsLib = null;

async function initPdfJs() {
    if (pdfjsLib) return pdfjsLib;
    
    return new Promise((resolve, reject) => {
        // Check if  loaded globally
        if (window.pdfjsLib) {
            pdfjsLib = window.pdfjsLib;
            pdfjsLib.GlobalWorkerOptions.workerSrc = PDFJS_CDN + 'pdf.worker.min.js';
            resolve(pdfjsLib);
            return;
        }
        
        const script = document.createElement('script');
        script.src = PDFJS_CDN + 'pdf.min.js';
        script.onload = () => {
            if (window.pdfjsLib) {
                pdfjsLib = window.pdfjsLib;
                pdfjsLib.GlobalWorkerOptions.workerSrc = PDFJS_CDN + 'pdf.worker.min.js';
                resolve(pdfjsLib);
            } else {
                reject(new Error('PDF.js library not found after loading'));
            }
        };
        script.onerror = () => reject(new Error('Failed to load PDF.js script'));
        document.head.appendChild(script);
    });
}

export class PdfRenderer {
    constructor() {
        this.pdfDoc = null;
        this.pdfBytes = null;
    }

    async loadDocument(pdfBytes) {
        await initPdfJs();
        
        this.pdfBytes = pdfBytes;
        this.pdfDoc = await pdfjsLib.getDocument({ data: pdfBytes }).promise;
        
        return {
            pageCount: this.pdfDoc.numPages
        };
    }

    // Numbero pages
    getPageCount() {
        return this.pdfDoc ? this.pdfDoc.numPages : 0;
    }


    async renderPageToCanvas(pageNumber, canvas, scale = 1.0) {
        if (!this.pdfDoc) {
            throw new Error('No PDF document loaded');
        }

        const page = await this.pdfDoc.getPage(pageNumber);
        const viewport = page.getViewport({ scale });

        canvas.width = viewport.width;
        canvas.height = viewport.height;

        const ctx = canvas.getContext('2d');
        const renderContext = {
            canvasContext: ctx,
            viewport: viewport
        };

        await page.render(renderContext).promise;
        
        return {
            width: viewport.width,
            height: viewport.height
        };
    }

    async renderPageToImageData(pageNumber, scale = 1.0) {
        const canvas = document.createElement('canvas');
        await this.renderPageToCanvas(pageNumber, canvas, scale);
        
        const ctx = canvas.getContext('2d');
        return ctx.getImageData(0, 0, canvas.width, canvas.height);
    }

    async renderPageToDataUrl(pageNumber, scale = 1.0, format = 'image/png') {
        const canvas = document.createElement('canvas');
        await this.renderPageToCanvas(pageNumber, canvas, scale);
        return canvas.toDataURL(format);
    }

    async renderThumbnail(pageNumber, maxWidth = 150) {
        if (!this.pdfDoc) {
            throw new Error('No PDF document loaded');
        }

        const page = await this.pdfDoc.getPage(pageNumber);
        const originalViewport = page.getViewport({ scale: 1.0 });
        
        // Calculate scale to fit maxWidth
        const scale = maxWidth / originalViewport.width;
        
        return this.renderPageToDataUrl(pageNumber, scale);
    }

    async renderAllThumbnails(maxWidth = 150, onProgress = null) {
        if (!this.pdfDoc) {
            throw new Error('No PDF document loaded');
        }

        const thumbnails = [];
        const total = this.pdfDoc.numPages;

        for (let i = 1; i <= total; i++) {
            const thumbnail = await this.renderThumbnail(i, maxWidth);
            thumbnails.push(thumbnail);
            
            if (onProgress) {
                onProgress(i, total);
            }
        }

        return thumbnails;
    }

    async getPageSize(pageNumber) {
        if (!this.pdfDoc) {
            throw new Error('No PDF document loaded');
        }

        const page = await this.pdfDoc.getPage(pageNumber);
        const viewport = page.getViewport({ scale: 1.0 });
        
        return {
            width: viewport.width,
            height: viewport.height
        };
    }

    async getAllPageSizes() {
        if (!this.pdfDoc) {
            throw new Error('No PDF document loaded');
        }

        const sizes = [];
        for (let i = 1; i <= this.pdfDoc.numPages; i++) {
            sizes.push(await this.getPageSize(i));
        }
        return sizes;
    }

    destroy() {
        if (this.pdfDoc) {
            this.pdfDoc.destroy();
            this.pdfDoc = null;
        }
        this.pdfBytes = null;
    }
}

// TS is ai gened 
export async function renderPdfPage(pdfBytes, pageNumber, canvas, scale = 1.0) {
    const renderer = new PdfRenderer();
    await renderer.loadDocument(pdfBytes);
    await renderer.renderPageToCanvas(pageNumber, canvas, scale);
    renderer.destroy();
}

export async function getPdfPageCount(pdfBytes) {
    await initPdfJs();
    const doc = await pdfjsLib.getDocument({ data: pdfBytes }).promise;
    const count = doc.numPages;
    doc.destroy();
    return count;
}

export async function renderPdfThumbnails(pdfBytes, maxWidth = 150, onProgress = null) {
    const renderer = new PdfRenderer();
    await renderer.loadDocument(pdfBytes);
    const thumbnails = await renderer.renderAllThumbnails(maxWidth, onProgress);
    renderer.destroy();
    return thumbnails;
}

if (typeof window !== 'undefined') {
    window.PdfRenderer = PdfRenderer;
    window.renderPdfPage = renderPdfPage;
    window.getPdfPageCount = getPdfPageCount;
    window.renderPdfThumbnails = renderPdfThumbnails;
}
