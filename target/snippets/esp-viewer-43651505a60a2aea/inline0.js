
let resourceDirectories = [];
const resourceDatabase = () => new Promise((resolve, reject) => {
  const request = indexedDB.open('nif-viewer-settings', 1);
  request.onupgradeneeded = () => request.result.createObjectStore('settings');
  request.onsuccess = () => resolve(request.result);
  request.onerror = () => reject(request.error);
});
async function saveResourceDirectories() {
  const database = await resourceDatabase();
  const transaction = database.transaction('settings', 'readwrite');
  transaction.objectStore('settings').put(resourceDirectories, 'resourceDirectories');
}
export async function load_resource_directories() {
  const database = await resourceDatabase();
  const transaction = database.transaction('settings', 'readonly');
  const request = transaction.objectStore('settings').get('resourceDirectories');
  request.onsuccess = () => { resourceDirectories = request.result || []; };
}
export function resource_directory_handles() { return resourceDirectories; }
export function pick_resource_directory() {
  window.showDirectoryPicker().then(async handle => {
    resourceDirectories.push(handle);
    await saveResourceDirectories();
  }).catch(() => {});
}
export function remove_resource_directory(index) {
  resourceDirectories.splice(index, 1);
  saveResourceDirectories().catch(() => {});
}
export function move_resource_directory(index, destination) {
  [resourceDirectories[index], resourceDirectories[destination]] = [resourceDirectories[destination], resourceDirectories[index]];
  saveResourceDirectories().catch(() => {});
}
