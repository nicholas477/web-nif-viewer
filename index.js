import init from './target/esp-viewer.js';

async function loadWasmWithProgress(wasmUrl) {
    const progressBar = document.getElementById('progress-bar');
    const progressText = document.getElementById('progress-text');
    const overlay = document.getElementById('loading-overlay');

    const response = await fetch(wasmUrl);
    if (!response.ok) {
        throw new Error(`Failed to fetch ${wasmUrl}: ${response.statusText}`);
    }

    const contentLength = response.headers.get('content-length');
    const total = contentLength ? parseInt(contentLength, 10) : 0;

    const reader = response.body.getReader();
    const chunks = [];
    let loaded = 0;

    while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        chunks.push(value);
        loaded += value.length;

        if (total > 0) {
            const percent = Math.min(100, Math.round((loaded / total) * 100));
            progressBar.style.width = `${percent}%`;
            progressText.textContent = `Loading WebAssembly... ${percent}%`;
        } else {
            const loadedMb = (loaded / (1024 * 1024)).toFixed(1);
            progressText.textContent = `Loading... ${loadedMb} MB`;
        }
    }

    progressText.textContent = 'Compiling WebAssembly...';

    // Merge chunks into a single ArrayBuffer
    const wasmBytes = new Uint8Array(loaded);
    let offset = 0;
    for (const chunk of chunks) {
        wasmBytes.set(chunk, offset);
        offset += chunk.length;
    }

    // Asynchronously compile and instantiate to avoid the >8MB restriction
    await init({ module_or_path: wasmBytes.buffer });

    // Hide the loader once initialization completes
    overlay.style.display = 'none';

    // Disable right click context menu on the canvas to prevent default browser behavior
    const canvas = document.querySelector('canvas');
    if (canvas) {
        canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    }
}

loadWasmWithProgress('./target/esp-viewer_bg.wasm').catch((err) => {
    console.error(err);
    document.getElementById('progress-text').textContent = 'Failed to load module.';
    document.getElementById('progress-bar').style.background = '#d9534f';
});