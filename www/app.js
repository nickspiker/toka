// Toka VM Browser Integration
// Loads WASM module and provides interactive example programs

// Immediate log to verify module loads
console.log('[APP.JS] Module loaded!');
const consoleEl = document.getElementById('console');
if (consoleEl) {
    const line = document.createElement('div');
    line.className = 'console-line console-info';
    line.textContent = '[APP.JS] Module script executed!';
    consoleEl.appendChild(line);
}

let TokaVM = null;
let wasmModule = null;
let currentVM = null;
let canvas = null;
let ctx = null;

// Console logging to HTML
function log(message, type = 'info') {
    const consoleEl = document.getElementById('console');
    const line = document.createElement('div');
    line.className = `console-line console-${type}`;
    line.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
    consoleEl.appendChild(line);
    consoleEl.scrollTop = consoleEl.scrollHeight;

    // Also log to browser console
    if (type === 'error') {
        console.error(message);
    } else {
        console.log(message);
    }
}

// Status display (just logs to console now)
function setStatus(message, isError = false) {
    log(message, isError ? 'error' : 'info');
}

// Initialize WASM module
async function init() {
    try {
        log('Starting WASM initialization...', 'info');
        setStatus('Loading WASM module...');

        // Import the WASM module
        log('Importing module from ./pkg/toka.js', 'info');
        const module = await import('./pkg/toka.js');
        log('Module imported successfully', 'info');

        log('Calling module.default() to initialize WASM...', 'info');
        await module.default();
        log('WASM initialized successfully', 'info');

        TokaVM = module.TokaVM;
        wasmModule = module;

        setStatus('WASM module loaded successfully');
        log('Ready to run programs', 'info');

    } catch (err) {
        setStatus(`Failed to load WASM: ${err.message}`, true);
        log(`WASM load error: ${err.message}`, 'error');
        log(`Stack: ${err.stack}`, 'error');
    }
}

// Create new VM instance with bytecode using window dimensions
function createVM(bytecode) {
    if (!TokaVM) {
        setStatus('WASM module not loaded!', true);
        return null;
    }

    try {
        const width = window.innerWidth;
        const height = window.innerHeight;
        log(`Creating VM with ${bytecode.length} bytes of bytecode at ${width}x${height}`, 'info');
        const vm = new TokaVM(bytecode, width, height);
        log('VM created successfully', 'info');
        setStatus('VM created successfully');
        return vm;
    } catch (err) {
        setStatus(`VM creation failed: ${err.message}`, true);
        log(`VM creation error: ${err.stack}`, 'error');
        return null;
    }
}

// Render VM canvas to HTML canvas
function render() {
    if (!currentVM) return;

    try {
        // Get RGBA bytes from VM
        const rgba = currentVM.get_canvas_rgba();

        // Create ImageData and render
        const imageData = new ImageData(
            new Uint8ClampedArray(rgba),
            currentVM.width(),
            currentVM.height()
        );

        ctx.putImageData(imageData, 0, 0);
    } catch (err) {
        setStatus(`Render error: ${err.message}`, true);
        console.error('Render error:', err);
    }
}

// Setup canvas
function setupCanvas() {
    canvas = document.getElementById('canvas');
    ctx = canvas.getContext('2d');

    // Set canvas size to window size
    function resizeCanvas() {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;

        // Clear to black
        ctx.fillStyle = '#000';
        ctx.fillRect(0, 0, canvas.width, canvas.height);

        // Re-render if VM exists
        if (currentVM) {
            render();
        }
    }

    resizeCanvas();
    window.addEventListener('resize', resizeCanvas);
}

// Main entry point
async function main() {
    log('Application starting...', 'info');
    setupCanvas();
    await init();
    log('Toka VM ready', 'info');
}

main().catch(err => {
    log(`Fatal error: ${err.message}`, 'error');
    log(`Stack: ${err.stack}`, 'error');
    setStatus(`Fatal error: ${err.message}`, true);
});
