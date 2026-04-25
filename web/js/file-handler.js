
export function formatFileSize(bytes) {
    if (bytes < 1024) {
        return `${bytes} B`;
    } else if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(1)} KB`;
    } else {
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }
}

//  Validate that a file is a PDF
//  Also just check the extension as a fallback but istg some hacker knows how to make pip file "act" like pdf or some bs
export function isPdfFile(file) {
    return file.type === 'application/pdf' || 
           file.name.toLowerCase().endsWith('.pdf');
}


export async function readFileAsBytes(file) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => {
            resolve(new Uint8Array(reader.result));
        };
        reader.onerror = () => reject(reader.error);
        reader.readAsArrayBuffer(file);
    });
}


export function downloadFile(data, filename) {
    const blob = new Blob([data], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    
    setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/**
 * Download multiple files as a ZIP
 * @param {Array<{data: Uint8Array, name: string}>} files - Files to zip
 * @param {string} zipName - Name for the ZIP file
 * AI gened this code cause idk and using jszip because i am too lazy to write zip code myself 
 */
export async function downloadAsZip(files, zipName) {
    const { default: JSZip } = await import('https://cdn.jsdelivr.net/npm/jszip@3.10.1/+esm');
    
    const zip = new JSZip();
    for (const file of files) {
        zip.file(file.name, file.data);
    }
    
    const blob = await zip.generateAsync({ type: 'blob' });
    const url = URL.createObjectURL(blob);
    
    const a = document.createElement('a');
    a.href = url;
    a.download = zipName;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    
    setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function setupDropZone(dropZone, fileInput, onFiles, options = {}) {
    const { multiple = true, validatePdf = true } = options;

    dropZone.addEventListener('click', () => {
        fileInput.click();
    });

    fileInput.addEventListener('change', (e) => {
        const files = Array.from(e.target.files || []);
        handleFiles(files);
        fileInput.value = ''; // Reset for re-selection
    });

    // Drag events
    dropZone.addEventListener('dragover', (e) => {
        e.preventDefault();
        e.stopPropagation();
        dropZone.classList.add('dragover');
    });

    dropZone.addEventListener('dragleave', (e) => {
        e.preventDefault();
        e.stopPropagation();
        dropZone.classList.remove('dragover');
    });

    dropZone.addEventListener('drop', (e) => {
        e.preventDefault();
        e.stopPropagation();
        dropZone.classList.remove('dragover');
        
        const files = Array.from(e.dataTransfer.files || []);
        handleFiles(files);
    });

    function handleFiles(files) {
        let validFiles = files;
        
        if (validatePdf) {
            validFiles = files.filter(isPdfFile);
            
            if (validFiles.length !== files.length) {
                const rejected = files.length - validFiles.length;
                console.warn(`${rejected} non-PDF files rejected`);
            }
        }

        if (!multiple && validFiles.length > 1) {
            validFiles = [validFiles[0]];
        }

        if (validFiles.length > 0) {
            onFiles(validFiles);
        }
    }
}


export function createFileListItem(file, index, draggable = false) {
    const item = document.createElement('div');
    item.className = 'file-item';
    item.dataset.index = index;
    
    if (draggable) {
        item.draggable = true;
    }

    item.innerHTML = `
        ${draggable ? '<div class="file-drag-handle">⋮⋮</div>' : ''}
        <div class="file-icon">📄</div>
        <div class="file-info">
            <div class="file-name">${escapeHtml(file.name)}</div>
            <div class="file-size">${formatFileSize(file.size)}</div>
        </div>
        <button class="file-remove" data-index="${index}" title="Remove">✕</button>
    `;

    return item;
}

// ANTI hack for xss(according to AI vunr check, code by me so can still be broken but chatgpt says ok)
export function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}


export function setupSortableList(container, onReorder) {
    let draggedItem = null;
    let draggedIndex = null;

    container.addEventListener('dragstart', (e) => {
        const item = e.target.closest('.file-item');
        if (!item) return;
        
        draggedItem = item;
        draggedIndex = parseInt(item.dataset.index, 10);
        item.style.opacity = '0.5';
    });

    container.addEventListener('dragend', (e) => {
        const item = e.target.closest('.file-item');
        if (item) {
            item.style.opacity = '';
        }
        draggedItem = null;
        draggedIndex = null;
    });

    container.addEventListener('dragover', (e) => {
        e.preventDefault();
        const item = e.target.closest('.file-item');
        if (!item || item === draggedItem) return;

        const rect = item.getBoundingClientRect();
        const midY = rect.top + rect.height / 2;
        
        if (e.clientY < midY) {
            container.insertBefore(draggedItem, item);
        } else {
            container.insertBefore(draggedItem, item.nextSibling);
        }
    });

    container.addEventListener('drop', (e) => {
        e.preventDefault();
        
        // Get new order from DOM
        const items = container.querySelectorAll('.file-item');
        const newOrder = Array.from(items).map(item => parseInt(item.dataset.index, 10));
        
        // Update indices
        items.forEach((item, i) => {
            item.dataset.index = i;
            const removeBtn = item.querySelector('.file-remove');
            if (removeBtn) removeBtn.dataset.index = i;
        });
        
        onReorder(newOrder);
    });
}


export function parsePageRanges(rangeStr, totalPages) {
    const pages = new Set();
    const parts = rangeStr.split(',').map(s => s.trim()).filter(Boolean);

    for (const part of parts) {
        if (part.includes('-')) {
            const [startStr, endStr] = part.split('-').map(s => s.trim());
            const start = parseInt(startStr, 10);
            const end = parseInt(endStr, 10);
            
            if (isNaN(start) || isNaN(end)) continue;
            
            for (let i = Math.max(1, start); i <= Math.min(totalPages, end); i++) {
                pages.add(i - 1); // Convert to 0-based
            }
        } else {
            const page = parseInt(part, 10);
            if (!isNaN(page) && page >= 1 && page <= totalPages) {
                pages.add(page - 1); // Convert to 0-based
            }
        }
    }

    return Array.from(pages).sort((a, b) => a - b);
}
