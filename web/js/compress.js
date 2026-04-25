import {
    setupDropZone,
    readFileAsBytes,
    formatFileSize,
    downloadFile
} from './file-handler.js';
import {
    compressPdf,
    compressPdfToTarget,
    compressPdfLossless,
    initPdfWorker
} from './pdf-worker-client.js';

// State
let currentFile = null;
let fileBytes = null;
let compressedResult = null;
let originalSize = 0;

// DOM Elements
const dropZone = document.getElementById('dropZone');
const fileInput = document.getElementById('fileInput');
const fileInfo = document.getElementById('fileInfo');
const fileName = document.getElementById('fileName');
const fileSize = document.getElementById('fileSize');
const removeFileBtn = document.getElementById('removeFile');
const optionsPanel = document.getElementById('optionsPanel');
const compressionMode = document.getElementById('compressionMode');
const qualityOption = document.getElementById('qualityOption');
const qualitySlider = document.getElementById('qualitySlider');
const qualityValue = document.getElementById('qualityValue');
const targetOption = document.getElementById('targetOption');
const targetSize = document.getElementById('targetSize');
const losslessOption = document.getElementById('losslessOption');
const progressContainer = document.getElementById('progressContainer');
const progressFill = document.getElementById('progressFill');
const progressText = document.getElementById('progressText');
const actionButtons = document.getElementById('actionButtons');
const resultsSection = document.getElementById('resultsSection');
const loadingOverlay = document.getElementById('loadingOverlay');

const clearBtn = document.getElementById('clearBtn');
const compressBtn = document.getElementById('compressBtn');
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

    compressionMode.addEventListener('change', handleModeChange);

    qualitySlider.addEventListener('input', () => {
        qualityValue.textContent = qualitySlider.value;
    });

    removeFileBtn.addEventListener('click', clearFile);
    clearBtn.addEventListener('click', clearFile);
    compressBtn.addEventListener('click', performCompress);
    startOverBtn.addEventListener('click', startOver);
    downloadBtn.addEventListener('click', downloadResult);
});


function handleModeChange() {
    const mode = compressionMode.value;

    qualityOption.style.display = mode === 'quality' ? 'block' : 'none';
    targetOption.style.display = mode === 'target' ? 'block' : 'none';
    losslessOption.style.display = mode === 'lossless' ? 'block' : 'none';

    // Set reasonable default target size
    if (mode === 'target' && originalSize > 0) {
        const defaultTarget = Math.round(originalSize / 1024 / 2); // Half of original
        targetSize.value = Math.max(10, defaultTarget);
    }
}


async function handleFile(files) {
    const file = files[0];
    if (!file) return;

    currentFile = file;
    originalSize = file.size;

    try {
        // Load file
        fileBytes = await readFileAsBytes(file);

        // Update stuff
        fileName.textContent = file.name;
        fileSize.textContent = formatFileSize(file.size);

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

/**
 * delt current file
 */
function clearFile() {
    currentFile = null;
    fileBytes = null;
    compressedResult = null;
    originalSize = 0;

    dropZone.style.display = 'block';
    fileInfo.style.display = 'none';
    optionsPanel.style.display = 'none';
    actionButtons.style.display = 'none';
    resultsSection.style.display = 'none';
    resultsSection.classList.remove('visible');
}

/**
 * Perform the compression
 */
async function performCompress() {
    if (!fileBytes) return;

    const mode = compressionMode.value;

    try {
        progressContainer.classList.add('visible');
        actionButtons.style.display = 'none';
        progressFill.style.width = '0%';
        progressText.textContent = 'Compressing...';

        // Build compression mode object
        let compressionModeObj;
        if (mode === 'quality') {
            const quality = parseInt(qualitySlider.value, 10);
            compressionModeObj = { type: 'Quality', value: quality };
            progressText.textContent = `Compressing at ${quality}% quality...`;
        } else if (mode === 'target') {
            const target = parseInt(targetSize.value, 10);
            if (isNaN(target) || target < 10) {
                throw new Error('Invalid target size');
            }
            compressionModeObj = { type: 'TargetSize', value: target * 1024 };
            progressText.textContent = `Compressing to ~${target} KB...`;
        } else {
            compressionModeObj = { type: 'Lossless' };
            progressText.textContent = 'Applying lossless compression...';
        }

        progressFill.style.width = '30%';

        let result;
        if (mode === 'quality') {
            const quality = parseInt(qualitySlider.value, 10);
            result = await compressPdf(fileBytes, quality);
        } else if (mode === 'target') {
            const target = parseInt(targetSize.value, 10);
            result = await compressPdfToTarget(fileBytes, target);
        } else {
            result = await compressPdfLossless(fileBytes);
        }

        progressFill.style.width = '100%';
        progressText.textContent = 'Complete!';

        compressedResult = result;

        const newSize = result.length;
        const reduction = ((originalSize - newSize) / originalSize * 100).toFixed(1);

        document.getElementById('originalSize').textContent = formatFileSize(originalSize);
        document.getElementById('compressedSize').textContent = formatFileSize(newSize);
        document.getElementById('reduction').textContent =
            newSize < originalSize ? `${reduction}%` : 'No reduction';

        // Show results
        setTimeout(() => {
            progressContainer.classList.remove('visible');
            optionsPanel.style.display = 'none';
            fileInfo.style.display = 'none';
            resultsSection.classList.add('visible');
            resultsSection.style.display = 'block';
        }, 500);

    } catch (err) {
        console.error('Compression failed:', err);
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

// DOWNLOADS IF IT WORKS IG
function downloadResult() {
    if (!compressedResult || !currentFile) return;

    const baseName = currentFile.name.replace('.pdf', '');
    downloadFile(compressedResult, `${baseName}_compressed.pdf`);
}
