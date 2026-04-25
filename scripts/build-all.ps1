# Build all WASM modules for CodePRivpdf (Windows PowerShell version)
# Requires: rustup, wasm-pack
# AI gen cause lazy

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$CratesDir = Join-Path $ProjectRoot "crates"
$OutputDir = Join-Path $ProjectRoot "web\wasm"

Write-Host "=== CodePRivpdf WASM Build ===" -ForegroundColor Cyan
Write-Host "Project root: $ProjectRoot"
Write-Host ""

# Ensure wasm-pack is installed
if (-not (Get-Command wasm-pack -ErrorAction SilentlyContinue)) {
    Write-Host "Installing wasm-pack..." -ForegroundColor Yellow
    cargo install wasm-pack
}

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

# List of crates to build (only crates that export WASM bindings)
# pdf-codecs and pdf-core are internal libraries, not WASM modules
$Crates = @(
    "pdf-merge",
    "pdf-split",
    "pdf-pages",
    "pdf-compress"
)

# Build each crate
foreach ($Crate in $Crates) {
    $CrateDir = Join-Path $CratesDir $Crate
    
    if (-not (Test-Path $CrateDir)) {
        Write-Host "WARNING: Crate directory not found: $CrateDir" -ForegroundColor Yellow
        continue
    }
    
    Write-Host "Building $Crate..." -ForegroundColor Green
    
    Push-Location $CrateDir
    
    try {
        # Build with wasm-pack
        $OutPath = Join-Path $OutputDir "$Crate\pkg"
        wasm-pack build --target web --out-dir $OutPath --release
        
        # Copy the main wasm file to root wasm dir for easy access
        $WasmName = $Crate -replace '-', '_'
        $WasmSource = Join-Path $OutPath "${WasmName}_bg.wasm"
        $JsBindingsSource = Join-Path $OutPath "${WasmName}.js"
        
        if (Test-Path $WasmSource) {
            Copy-Item $WasmSource (Join-Path $OutputDir "${WasmName}.wasm")
        }
        if (Test-Path $JsBindingsSource) {
            Copy-Item $JsBindingsSource (Join-Path $OutputDir "${WasmName}.js")
        }
        # _bg.js may not exist in all wasm-pack versions
        $JsBgSource = Join-Path $OutPath "${WasmName}_bg.js"
        if (Test-Path $JsBgSource) {
            Copy-Item $JsBgSource (Join-Path $OutputDir "${WasmName}_bg.js")
        }
        
        Write-Host "✓ $Crate built successfully" -ForegroundColor Green
    }
    finally {
        Pop-Location
    }
    
    Write-Host ""
}

Write-Host "=== Build Complete ===" -ForegroundColor Cyan
Write-Host "WASM files output to: $OutputDir"
Write-Host ""

# Show file sizes
Write-Host "File sizes:" -ForegroundColor Yellow
foreach ($Crate in $Crates) {
    $WasmName = $Crate -replace '-', '_'
    $WasmFile = Join-Path $OutputDir "${WasmName}.wasm"
    if (Test-Path $WasmFile) {
        $Size = (Get-Item $WasmFile).Length
        $SizeKB = [math]::Round($Size / 1024, 1)
        Write-Host "  ${WasmName}.wasm: ${SizeKB} KB"
    }
}

Write-Host ""
Write-Host "Run 'scripts\compress-wasm.ps1' to apply Brotli compression"
