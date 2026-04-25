
import { 
    setupDropZone, 
    readFileAsBytes,
    formatFileSize,
    downloadFile,
    downloadAsZip,
    parsePageRanges
} from './file-handler.js';
import { 
    splitPdfAll, 
    splitPdfByRanges, 
    initPdfWorker, 
    getPageCount 
} from './pdf-worker-client.js';

let currentFile = null;
let fileBytes = null;
let pageCount = 0;
let splitResults = [];

// DOM Elements
const dropZone = document.getElementById('dropZone');
const fileInput = document.getElementById('fileInput');
const fileInfo = document.getElementById('fileInfo');
const fileName = document.getElementById('fileName');
const fileSize = document.getElementById('fileSize');
const removeFileBtn = document.getElementById('removeFile');
const optionsPanel = document.getElementById('optionsPanel');
const splitMode = document.getElementById('splitMode');
const rangesOption = document.getElementById('rangesOption');
const rangesInput = document.getElementById('rangesInput');
const fixedOption = document.getElementById('fixedOption');
const pagesPerFile = document.getElementById('pagesPerFile');
const progressContainer = document.getElementById('progressContainer');
const progressFill = document.getElementById('progressFill');
const progressText = document.getElementById('progressText');
const actionButtons = document.getElementById('actionButtons');
const resultsSection = document.getElementById('resultsSection');
const loadingOverlay = document.getElementById('loadingOverlay');

const clearBtn = document.getElementById('clearBtn');
const splitBtn = document.getElementById('splitBtn');
const startOverBtn = document.getElementById('startOverBtn');
const downloadAllBtn = document.getElementById('downloadAllBtn');


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

    splitMode.addEventListener('change', handleModeChange);

    removeFileBtn.addEventListener('click', clearFile);
    clearBtn.addEventListener('click', clearFile);
    splitBtn.addEventListener('click', performSplit);
    startOverBtn.addEventListener('click', startOver);
    downloadAllBtn.addEventListener('click', downloadAll);
});


function handleModeChange() {
    const mode = splitMode.value;
    
    rangesOption.style.display = mode === 'ranges' ? 'block' : 'none';
    fixedOption.style.display = mode === 'fixed' ? 'block' : 'none';
    
    if (mode === 'fixed') {
        pagesPerFile.max = pageCount;
        pagesPerFile.value = Math.min(5, pageCount);
    }
}

async function handleFile(files) {
    const file = files[0];
    if (!file) return;

    currentFile = file;
    
    try {
        fileBytes = await readFileAsBytes(file);
        pageCount = await getPageCount(fileBytes);
        
        fileName.textContent = file.name;
        fileSize.textContent = `${formatFileSize(file.size)} • ${pageCount} pages`;
        
        dropZone.style.display = 'none';
        fileInfo.style.display = 'block';
        optionsPanel.style.display = 'block';
        actionButtons.style.display = 'flex';
        
        handleModeChange();
    } catch (err) {
        console.error('Failed to load file:', err);
        alert('Failed to load PDF file: ' + err.message);
        clearFile();
    }
}

function clearFile() {
    currentFile = null;
    fileBytes = null;
    pageCount = 0;
    splitResults = [];
    
    dropZone.style.display = 'block';
    fileInfo.style.display = 'none';
    optionsPanel.style.display = 'none';
    actionButtons.style.display = 'none';
    resultsSection.style.display = 'none';
    resultsSection.classList.remove('visible');
}

async function performSplit() {
    if (!fileBytes) return;

    const mode = splitMode.value;
    
    try {
        progressContainer.classList.add('visible');
        actionButtons.style.display = 'none';
        progressFill.style.width = '0%';
        progressText.textContent = 'Processing...';

        let results = [];

        if (mode === 'all') {
            progressText.textContent = 'Splitting all pages...';
            results = await splitPdfAll(fileBytes);
            
        } else if (mode === 'ranges') {
            const rangeStr = rangesInput.value.trim();
            if (!rangeStr) {
                throw new Error('Please enter page ranges');
            }
            
            const ranges = parseRangesString(rangeStr);
            if (ranges.length === 0) {
                throw new Error('Invalid page ranges');
            }
            
            progressText.textContent = `Splitting into ${ranges.length} parts...`;
            results = await splitPdfByRanges(fileBytes, ranges);
            
        } else if (mode === 'fixed') {
            // Split every N pages
            const n = parseInt(pagesPerFile.value, 10);
            if (isNaN(n) || n < 1) {
                throw new Error('Invalid pages per file');
            }
            
            const ranges = [];
            for (let i = 1; i <= pageCount; i += n) {
                ranges.push({
                    start: i,  // 1-indexed (Rust expects 1-indexed)
                    end: Math.min(i + n - 1, pageCount)
                });
            }
            
            progressText.textContent = `Splitting into ${ranges.length} parts...`;
            results = await splitPdfByRanges(fileBytes, ranges);
        }

        progressFill.style.width = '100%';
        progressText.textContent = 'Complete!';

        splitResults = results.map((data, i) => ({
            data,
            name: generateResultName(currentFile.name, i, results.length)
        }));

        // Calculate total pages extracted
        let totalPages = 0;
        if (mode === 'all') {
            totalPages = pageCount;
        } else if (mode === 'ranges') {
            // Sum up pages from each range
            const rangeStr = rangesInput.value.trim();
            const ranges = parseRangesString(rangeStr);
            totalPages = ranges.reduce((sum, r) => sum + (r.end - r.start + 1), 0);
        } else if (mode === 'fixed') {
            // All pages are included, just split into chunks
            totalPages = pageCount;
        }

        document.getElementById('resultFiles').textContent = splitResults.length;
        document.getElementById('resultPages').textContent = totalPages;

        setTimeout(() => {
            progressContainer.classList.remove('visible');
            optionsPanel.style.display = 'none';
            fileInfo.style.display = 'none';
            resultsSection.classList.add('visible');
            resultsSection.style.display = 'block';
        }, 500);

    } catch (err) {
        console.error('Split failed:', err);
        progressText.textContent = `Error: ${err.message}`;
        progressFill.style.backgroundColor = 'var(--color-error)';
        
        setTimeout(() => {
            progressContainer.classList.remove('visible');
            actionButtons.style.display = 'flex';
            progressFill.style.backgroundColor = '';
        }, 2000);
    }
}

function parseRangesString(str) {
    const ranges = [];
    const parts = str.split(',').map(s => s.trim()).filter(Boolean);

    for (const part of parts) {
        if (part.includes('-')) {
            const [startStr, endStr] = part.split('-').map(s => s.trim());
            const start = parseInt(startStr, 10); // Keep 1-indexed (Rust expects 1-indexed)
            const end = parseInt(endStr, 10);
            
            if (!isNaN(start) && !isNaN(end) && start >= 1 && end >= start && end <= pageCount) {
                ranges.push({ start, end });
            }
        } else {
            const page = parseInt(part, 10);
            if (!isNaN(page) && page >= 1 && page <= pageCount) {
                ranges.push({ start: page, end: page });
            }
        }
    }

    return ranges;
}


function generateResultName(originalName, index, total) {
    const baseName = originalName.replace('.pdf', '');
    const padLength = String(total).length;
    const paddedIndex = String(index + 1).padStart(padLength, '0');
    return `${baseName}_part${paddedIndex}.pdf`;
}

function startOver() {
    clearFile();
}

// ZIP DOWNLOAD YIPEEEE
async function downloadAll() {
    if (splitResults.length === 0) return;

    if (splitResults.length === 1) {
        downloadFile(splitResults[0].data, splitResults[0].name);
    } else {
        const baseName = currentFile ? currentFile.name.replace('.pdf', '') : 'split';
        await downloadAsZip(splitResults, `${baseName}_split.zip`);
    }
}
