
import { 
    setupDropZone, 
    readFileAsBytes,
    formatFileSize,
    downloadFile
} from './file-handler.js';
import { 
    removePages, 
    extractPages,
    initPdfWorker, 
    getPageCount 
} from './pdf-worker-client.js';
import { PdfRenderer } from './pdf-renderer.js';

let currentFile = null;
let fileBytes = null;
let pageCount = 0;
let selectedPages = new Set();
let processedResult = null;

// DOM Elements
const dropZone = document.getElementById('dropZone');
const fileInput = document.getElementById('fileInput');
const fileInfo = document.getElementById('fileInfo');
const fileName = document.getElementById('fileName');
const fileSize = document.getElementById('fileSize');
const removeFileBtn = document.getElementById('removeFile');
const optionsPanel = document.getElementById('optionsPanel');
const pageMode = document.getElementById('pageMode');
const modeDescription = document.getElementById('modeDescription');
const selectAllBtn = document.getElementById('selectAllBtn');
const selectNoneBtn = document.getElementById('selectNoneBtn');
const thumbnailsGrid = document.getElementById('thumbnailsGrid');
const progressContainer = document.getElementById('progressContainer');
const progressFill = document.getElementById('progressFill');
const progressText = document.getElementById('progressText');
const actionButtons = document.getElementById('actionButtons');
const resultsSection = document.getElementById('resultsSection');
const processBtn = document.getElementById('processBtn');
const loadingOverlay = document.getElementById('loadingOverlay');

// Buttons
const clearBtn = document.getElementById('clearBtn');
const startOverBtn = document.getElementById('startOverBtn');
const downloadBtn = document.getElementById('downloadBtn');

document.addEventListener('DOMContentLoaded', async () => {
    loadingOverlay.classList.add('visible');
    
    try {
        await initPdfWorker();
    } catch (err) {
        console.error('Failed to initialize PDF worker:', err);
    } finally {
        loadingOverlay.classList.remove('visible');
    }

    setupDropZone(dropZone, fileInput, handleFile, { multiple: false });

    pageMode.addEventListener('change', handleModeChange);

    selectAllBtn.addEventListener('click', selectAll);
    selectNoneBtn.addEventListener('click', selectNone);

    removeFileBtn.addEventListener('click', clearFile);
    clearBtn.addEventListener('click', clearFile);
    processBtn.addEventListener('click', performProcess);
    startOverBtn.addEventListener('click', startOver);
    downloadBtn.addEventListener('click', downloadResult);
});


function handleModeChange() {
    const mode = pageMode.value;
    if (mode === 'remove') {
        modeDescription.textContent = 'Selected pages will be removed from the PDF.';
        processBtn.textContent = 'Remove Pages';
    } else {
        modeDescription.textContent = 'Only selected pages will be kept in the new PDF.';
        processBtn.textContent = 'Extract Pages';
    }
}

async function handleFile(files) {
    const file = files[0];
    if (!file) return;

    currentFile = file;
    selectedPages.clear();
    
    try {
        progressContainer.classList.add('visible');
        progressFill.style.width = '0%';
        progressText.textContent = 'Loading PDF...';
        
        // Load 
        fileBytes = await readFileAsBytes(file);
        progressFill.style.width = '30%';
        
        pageCount = await getPageCount(fileBytes);
        progressFill.style.width = '50%';
        
        // Update 
        fileName.textContent = file.name;
        fileSize.textContent = `${formatFileSize(file.size)} • ${pageCount} pages`;
        
        dropZone.style.display = 'none';
        fileInfo.style.display = 'block';
        optionsPanel.style.display = 'block';
        
        // Render. Coping this always works PDF.js pls work(i tried rust for 3 hours with WASM)
        progressText.textContent = 'Generating thumbnails...';
        await renderThumbnailsGrid();
        
        progressContainer.classList.remove('visible');
        actionButtons.style.display = 'flex';
        
        handleModeChange();
        updateProcessButton();
    } catch (err) {
        console.error('Failed to load file:', err);
        progressText.textContent = `Error: ${err.message}`;
        progressFill.style.backgroundColor = 'var(--color-error)';
        
        setTimeout(() => {
            progressContainer.classList.remove('visible');
            clearFile();
        }, 2000);
    }
}

async function renderThumbnailsGrid() {
    thumbnailsGrid.innerHTML = '';
    
    try {
        // Use PDF.js renderer for thumbnails
        const renderer = new PdfRenderer();
        // Pass a copy to avoid ArrayBuffer detachment (PDF.js may transfer the buffer)
        await renderer.loadDocument(new Uint8Array(fileBytes));
        
        for (let i = 0; i < pageCount; i++) {
            const dataUrl = await renderer.renderThumbnail(i + 1, 120);
            const thumb = createThumbnailFromDataUrl(i, dataUrl);
            thumbnailsGrid.appendChild(thumb);
            progressFill.style.width = `${50 + ((i + 1) / pageCount) * 50}%`;
        }
        
        renderer.destroy();
    } catch (err) {
        console.warn('Thumbnail rendering failed, using placeholders:', err);
        
        // Fallback to placeholder 
        for (let i = 0; i < pageCount; i++) {
            const thumb = createPlaceholderThumbnail(i);
            thumbnailsGrid.appendChild(thumb);
        }
    }
}


function createThumbnailFromDataUrl(pageIndex, dataUrl) {
    const div = document.createElement('div');
    div.className = 'thumbnail';
    div.dataset.page = pageIndex;
    
    const img = document.createElement('img');
    img.src = dataUrl;
    img.className = 'thumbnail-image';
    img.alt = `Page ${pageIndex + 1}`;
    
    div.innerHTML = `
        <div class="thumbnail-checkbox">✓</div>
        <div class="thumbnail-number">Page ${pageIndex + 1}</div>
    `;
    div.insertBefore(img, div.firstChild);
    
    div.addEventListener('click', () => togglePage(pageIndex));
    
    return div;
}


function createPlaceholderThumbnail(pageIndex) {
    const div = document.createElement('div');
    div.className = 'thumbnail';
    div.dataset.page = pageIndex;
    
    div.innerHTML = `
        <div class="thumbnail-image" style="background: linear-gradient(135deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%); display: flex; align-items: center; justify-content: center; font-size: 2rem; color: #999;">
            📄
        </div>
        <div class="thumbnail-checkbox">✓</div>
        <div class="thumbnail-number">Page ${pageIndex + 1}</div>
    `;
    
    div.addEventListener('click', () => togglePage(pageIndex));
    
    return div;
}


function togglePage(pageIndex) {
    if (selectedPages.has(pageIndex)) {
        selectedPages.delete(pageIndex);
    } else {
        selectedPages.add(pageIndex);
    }
    
    updateThumbnailSelection(pageIndex);
    updateProcessButton();
}


function updateThumbnailSelection(pageIndex) {
    const thumb = thumbnailsGrid.querySelector(`[data-page="${pageIndex}"]`);
    if (thumb) {
        thumb.classList.toggle('selected', selectedPages.has(pageIndex));
    }
}


function updateAllThumbnailSelections() {
    const thumbs = thumbnailsGrid.querySelectorAll('.thumbnail');
    thumbs.forEach(thumb => {
        const pageIndex = parseInt(thumb.dataset.page, 10);
        thumb.classList.toggle('selected', selectedPages.has(pageIndex));
    });
}


function selectAll() {
    for (let i = 0; i < pageCount; i++) {
        selectedPages.add(i);
    }
    updateAllThumbnailSelections();
    updateProcessButton();
}

function selectNone() {
    selectedPages.clear();
    updateAllThumbnailSelections();
    updateProcessButton();
}

function updateProcessButton() {
    const mode = pageMode.value;
    const hasSelection = selectedPages.size > 0;
    const allSelected = selectedPages.size === pageCount;
    
    // For remove mode: can't remove all pages
    // For extract mode: need at least one page
    if (mode === 'remove') {
        processBtn.disabled = !hasSelection || allSelected;
    } else {
        processBtn.disabled = !hasSelection;
    }
}

function clearFile() {
    currentFile = null;
    fileBytes = null;
    pageCount = 0;
    selectedPages.clear();
    processedResult = null;
    
    thumbnailsGrid.innerHTML = '';
    dropZone.style.display = 'block';
    fileInfo.style.display = 'none';
    optionsPanel.style.display = 'none';
    actionButtons.style.display = 'none';
    resultsSection.style.display = 'none';
    resultsSection.classList.remove('visible');
}


async function performProcess() {
    if (!fileBytes || selectedPages.size === 0) return;

    const mode = pageMode.value;
    // Convert 0-indexed page indices to 1-indexed page numbers (Rust expects 1-indexed)
    const pageNumbers = Array.from(selectedPages).map(i => i + 1).sort((a, b) => a - b);
    
    try {
        progressContainer.classList.add('visible');
        actionButtons.style.display = 'none';
        progressFill.style.width = '0%';
        
        let result;
        let newPageCount;

        if (mode === 'remove') {
            progressText.textContent = `Removing ${pageNumbers.length} pages...`;
            progressFill.style.width = '30%';
            result = await removePages(fileBytes, pageNumbers);
            newPageCount = pageCount - pageNumbers.length;
        } else {
            progressText.textContent = `Extracting ${pageNumbers.length} pages...`;
            progressFill.style.width = '30%';
            result = await extractPages(fileBytes, pageNumbers);
            newPageCount = pageNumbers.length;
        }

        progressFill.style.width = '100%';
        progressText.textContent = 'Complete!';

        processedResult = result;

        document.getElementById('resultTitle').textContent = 
            mode === 'remove' ? 'Pages Removed!' : 'Pages Extracted!';
        document.getElementById('resultOriginalPages').textContent = pageCount;
        document.getElementById('resultNewPages').textContent = newPageCount;

        setTimeout(() => {
            progressContainer.classList.remove('visible');
            optionsPanel.style.display = 'none';
            fileInfo.style.display = 'none';
            thumbnailsGrid.innerHTML = '';
            resultsSection.classList.add('visible');
            resultsSection.style.display = 'block';
        }, 500);

    } catch (err) {
        console.error('Processing failed:', err);
        progressText.textContent = `Error: ${err.message}`;
        progressFill.style.backgroundColor = 'var(--color-error)';
        
        setTimeout(() => {
            progressContainer.classList.remove('visible');
            actionButtons.style.display = 'flex';
            progressFill.style.backgroundColor = '';
        }, 2000);
    }
}

function startOver() {
    clearFile();
}

// MORE COPA PASTA BUT CHANGED?
function downloadResult() {
    if (!processedResult || !currentFile) return;
    
    const mode = pageMode.value;
    const baseName = currentFile.name.replace('.pdf', '');
    const suffix = mode === 'remove' ? '_modified' : '_extracted';
    downloadFile(processedResult, `${baseName}${suffix}.pdf`);
}
