

import { 
    setupDropZone, 
    createFileListItem, 
    setupSortableList,
    readFileAsBytes,
    formatFileSize,
    downloadFile 
} from './file-handler.js';
import { mergePdfs, initPdfWorker, getPageCount } from './pdf-worker-client.js';

let files = [];
let mergedResult = null;

// DOM Elements
const dropZone = document.getElementById('dropZone');
const fileInput = document.getElementById('fileInput');
const fileList = document.getElementById('fileList');
const addMoreContainer = document.getElementById('addMoreContainer');
const addMoreBtn = document.getElementById('addMoreBtn');
const addMoreInput = document.getElementById('addMoreInput');
const actionButtons = document.getElementById('actionButtons');
const progressContainer = document.getElementById('progressContainer');
const progressFill = document.getElementById('progressFill');
const progressText = document.getElementById('progressText');
const resultsSection = document.getElementById('resultsSection');
const loadingOverlay = document.getElementById('loadingOverlay');

// Buttons
const clearBtn = document.getElementById('clearBtn');
const mergeBtn = document.getElementById('mergeBtn');
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

    setupDropZone(dropZone, fileInput, handleFiles, { multiple: true });

    setupSortableList(fileList, handleReorder);

    fileList.addEventListener('click', (e) => {
        if (e.target.classList.contains('file-remove')) {
            const index = parseInt(e.target.dataset.index, 10);
            removeFile(index);
        }
    });

    clearBtn.addEventListener('click', clearAll);
    mergeBtn.addEventListener('click', performMerge);
    startOverBtn.addEventListener('click', startOver);
    downloadBtn.addEventListener('click', downloadResult);
    
    addMoreBtn.addEventListener('click', () => addMoreInput.click());
    addMoreInput.addEventListener('change', (e) => {
        if (e.target.files.length > 0) {
            handleFiles(Array.from(e.target.files));
            e.target.value = ''; // Reset
        }
    });
});

async function handleFiles(newFiles) {
    // Add files to list
    for (const file of newFiles) {
        files.push({
            file,
            bytes: null, // Load lazily (cope rn)
            pageCount: null
        });
    }

    await updateFileList();
    updateUI();
}

async function updateFileList() {
    fileList.innerHTML = '';

    for (let i = 0; i < files.length; i++) {
        const fileInfo = files[i];
        const item = createFileListItem(fileInfo.file, i, true);
        
        if (fileInfo.pageCount !== null) {
            const sizeElem = item.querySelector('.file-size');
            sizeElem.textContent = `${formatFileSize(fileInfo.file.size)} • ${fileInfo.pageCount} pages`;
        }
        
        fileList.appendChild(item);

        // Load page count in background if not loaded
        if (fileInfo.pageCount === null && fileInfo.bytes === null) {
            loadFileInfo(i);
        }
    }
}

async function loadFileInfo(index) {
    const fileInfo = files[index];
    if (!fileInfo || fileInfo.bytes !== null) return;

    try {
        fileInfo.bytes = await readFileAsBytes(fileInfo.file);
        fileInfo.pageCount = await getPageCount(fileInfo.bytes);
        
        //  display
        const item = fileList.querySelector(`[data-index="${index}"]`);
        if (item) {
            const sizeElem = item.querySelector('.file-size');
            sizeElem.textContent = `${formatFileSize(fileInfo.file.size)} • ${fileInfo.pageCount} pages`;
        }
    } catch (err) {
        console.error('Failed to load file info:', err);
    }
}

function handleReorder(newOrder) {
    const reorderedFiles = newOrder.map(i => files[i]);
    files = reorderedFiles;
}

function removeFile(index) {
    files.splice(index, 1);
    updateFileList();
    updateUI();
}

function clearAll() {
    files = [];
    fileList.innerHTML = '';
    updateUI();
}


function updateUI() {
    const hasFiles = files.length > 0;
    const hasMultipleFiles = files.length >= 2;
    
    actionButtons.style.display = hasFiles ? 'flex' : 'none';
    addMoreContainer.style.display = hasFiles ? 'block' : 'none';
    mergeBtn.disabled = !hasMultipleFiles;
    
    // Show hint if only 1 file
    if (files.length === 1) {
        mergeBtn.textContent = 'Add at least 2 PDFs to merge';
        mergeBtn.title = 'You need at least 2 PDF files to merge';
    } else {
        mergeBtn.textContent = 'Merge PDFs';
        mergeBtn.title = '';
    }
    
    dropZone.style.display = hasFiles ? 'none' : 'block';
    
    resultsSection.classList.remove('visible');
}

// Kermit in fire meme here because this code killed me.
async function performMerge() {
    if (files.length < 2) return;

    try {
        // Show progress
        progressContainer.classList.add('visible');
        actionButtons.style.display = 'none';
        progressFill.style.width = '0%';
        progressText.textContent = 'Loading files...';

        const bytesArray = [];
        for (let i = 0; i < files.length; i++) {
            if (files[i].bytes === null) {
                files[i].bytes = await readFileAsBytes(files[i].file);
            }
            bytesArray.push(files[i].bytes);
            progressFill.style.width = `${((i + 1) / files.length) * 50}%`;
        }

        progressText.textContent = 'Merging PDFs...';
        progressFill.style.width = '60%';

        //  merge
        mergedResult = await mergePdfs(bytesArray);
        
        progressFill.style.width = '100%';
        progressText.textContent = 'Complete!';

        // Calculate stats
        const totalPages = files.reduce((sum, f) => sum + (f.pageCount || 0), 0);
        document.getElementById('resultPages').textContent = totalPages;
        document.getElementById('resultSize').textContent = formatFileSize(mergedResult.length);

        setTimeout(() => {
            progressContainer.classList.remove('visible');
            fileList.innerHTML = '';
            resultsSection.classList.add('visible');
            resultsSection.style.display = 'block';
        }, 500);

    } catch (err) {
        console.error('Merge failed:', err);
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
    files = [];
    mergedResult = null;
    fileList.innerHTML = '';
    resultsSection.style.display = 'none';
    resultsSection.classList.remove('visible');
    dropZone.style.display = 'block';
    updateUI();
}

// COPY PASTED CODE YAYAYA
function downloadResult() {
    if (!mergedResult) return;
    
    const baseName = files.length > 0 
        ? files[0].file.name.replace('.pdf', '') 
        : 'document';
    downloadFile(mergedResult, `${baseName}_merged.pdf`);
}
