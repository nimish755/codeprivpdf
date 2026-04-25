# CodePRivpdf - Privacy-First PDF Tools

A **privacy-first, browser-based PDF manipulation tool** built with Rust and WebAssembly. All processing happens entirely in your browser - your files never leave your device.

## 🔒 Privacy Features

- **Zero Knowledge Architecture**: No uploads, no servers, no tracking
- **100% Client-Side**: All PDF operations run locally in your browser
- **No Data Collection**: We never see your files or their contents
- **Offline Capable**: Works without internet after initial load (Service Worker cached)

## ✨ Features

| Feature | Description |
|---------|-------------|
| **Merge PDF** | Combine multiple PDF files into one. Drag to reorder. |
| **Split PDF** | Split by page ranges, fixed chunks, or individual pages |
| **Compress PDF** | Reduce file size with quality control, target size, or lossless |
| **Remove Pages** | Delete specific pages from a PDF |
| **Extract Pages** | Create a new PDF with only selected pages |

## 🛠️ Technology Stack

### Backend (Rust/WASM)
- **Rust 2021 Edition** - Memory-safe, high-performance language
- **lopdf v0.34** - PDF parsing and manipulation (pinned for WASM compatibility)
- **image** - Image processing (PNG/JPEG encoding)
- **wasm-bindgen** - Rust/JavaScript interop

### Frontend
- **Vanilla HTML/CSS/JS** - No heavy frameworks
- **PDF.js** - Mozilla's PDF renderer for page thumbnails
- **Web Workers + Comlink** - Non-blocking PDF operations
- **Service Worker** - WASM caching and offline support
- **ES6 Modules** - Modern JavaScript

## 📦 Project Structure

```
CodePRivpdf/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── pdf-core/           # Shared types and error handling
│   ├── pdf-merge/          # Merge multiple PDFs
│   ├── pdf-split/          # Split PDF by ranges
│   ├── pdf-pages/          # Remove/extract pages
│   ├── pdf-codecs/         # Image encoding/decoding
│   └── pdf-compress/       # Compression with quality control
├── web/
│   ├── index.html          # Landing page
│   ├── merge.html          # Merge tool
│   ├── split.html          # Split tool
│   ├── compress.html       # Compress tool
│   ├── pages.html          # Remove/Extract pages tool
│   ├── css/
│   │   └── styles.css      # Responsive CSS with dark mode
│   ├── js/
│   │   ├── pdf.worker.js   # Web Worker for PDF operations
│   │   ├── pdf-worker-client.js  # Main thread API
│   │   ├── pdf-renderer.js # PDF.js wrapper for thumbnails
│   │   ├── file-handler.js # Drag & drop utilities
│   │   ├── merge.js        # Merge page logic
│   │   ├── split.js        # Split page logic
│   │   ├── compress.js     # Compress page logic
│   │   ├── pages.js        # Pages page logic
│   │   └── sw.js           # Service Worker (offline support)
│   └── wasm/               # Compiled WASM modules (build output)
└── scripts/
    ├── build-all.sh        # Build all WASM modules (Unix)
    └── build-all.ps1       # Build all WASM modules (Windows)
```

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- A web server for local development

### Build

**Windows (PowerShell):**
```powershell
.\scripts\build-all.ps1
```

**Unix (bash):**
```bash
./scripts/build-all.sh
```

### Serve Locally

Use any static file server:

```bash
# Python
python -m http.server 8080 -d web


# Node.js (npx)
npx serve web

# Or use VS Code Live Server extension
```

Then open `http://localhost:8080` in your browser.

## 🏗️ Architecture

### Modular WASM Design

Each feature is a separate WASM module that loads on-demand:

```
User clicks "Merge PDF"
    → wasm-loader.js fetches pdf_merge.wasm
    → Service Worker caches for future use
    → pdf.worker.js runs merge in Web Worker
    → UI remains responsive during processing
```

### Compression Modes

The compression feature supports three modes:

1. **Quality (1-100)**: Direct JPEG quality control
2. **Target Size**: Binary search to achieve specific file size
3. **Lossless**: PNG optimization without quality loss

### lopdf Version Pinning

This project pins `lopdf = "=0.34"` because:
- v0.35+ has a 96% performance regression in WASM due to memchr issues
- v0.36+ requires special getrandom configuration for WASM

## 🔧 Development

### Adding a New Feature

1. Create a new crate in `crates/`
2. Add to workspace members in root `Cargo.toml`
3. Implement WASM exports with `#[wasm_bindgen]`
4. Add to `pdf.worker.js` API
5. Create HTML page and feature JS

### Testing

```bash
# Test a specific crate
cd crates/pdf-merge
cargo test

# Test all crates
cargo test --workspace
```

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [lopdf](https://github.com/J-F-Liu/lopdf) - PDF manipulation
- [PDF.js](https://mozilla.github.io/pdf.js/) - Mozilla's PDF renderer
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) - Rust WASM tooling
- [Comlink](https://github.com/GoogleChromeLabs/comlink) - Web Worker communication

---

**Built with ❤️ and Rust 🦀**
