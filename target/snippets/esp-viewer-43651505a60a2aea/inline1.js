
const recentFileDatabase = () => new Promise((resolve, reject) => {
    const request = indexedDB.open('esp-viewer-recent-files', 1);
    request.onupgradeneeded = () => request.result.createObjectStore('files');
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
});
async function saveRecentFile(handle) {
    const database = await recentFileDatabase();
    const transaction = database.transaction('files', 'readwrite');
    transaction.objectStore('files').put(handle, handle.name);
}
export async function pick_persisted_file(accepts) {
    if (!window.showOpenFilePicker) return null;
    const extensions = accepts.split(',').map(value => value.trim()).filter(value => value.startsWith('.'));
    const [handle] = await window.showOpenFilePicker({
        multiple: false,
        types: extensions.length ? [{ description: 'Supported files', accept: { 'application/octet-stream': extensions } }] : [],
    });
    await saveRecentFile(handle);
    return handle;
}
export async function open_persisted_file(name) {
    const database = await recentFileDatabase();
    const transaction = database.transaction('files', 'readonly');
    const request = transaction.objectStore('files').get(name);
    const handle = await new Promise((resolve, reject) => {
        request.onsuccess = () => resolve(request.result || null);
        request.onerror = () => reject(request.error);
    });
    if (!handle) return null;
    const permission = await handle.queryPermission({ mode: 'read' });
    if (permission !== 'granted' && await handle.requestPermission({ mode: 'read' }) !== 'granted') return null;
    return handle;
}
