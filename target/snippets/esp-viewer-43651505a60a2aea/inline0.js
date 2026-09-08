
let resourceDirectories = [];
let resourceFilesystemUpdated = false;
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
    await new Promise((resolve, reject) => {
        request.onsuccess = () => {
            resourceDirectories = request.result || [];
            resolve();
        };
        request.onerror = () => reject(request.error);
    });
}
export async function log_resource_directory_permissions() {
        let needsPermission = false;
    for (const directory of resourceDirectories) {
        try {
            const permission = await directory.queryPermission({ mode: 'read' });
                        needsPermission ||= permission !== 'granted';
            console.info(`Resource folder ${directory.name}: ${permission}`);
        } catch (error) {
                        needsPermission = true;
            console.warn(`Could not check access to resource folder ${directory.name}:`, error);
        }
    }
        return needsPermission;
}
export function request_resource_directory_permissions() {
    Promise.all(resourceDirectories.map(directory => directory.requestPermission({ mode: 'read' })
            .then(permission => console.info(`Resource folder ${directory.name}: ${permission}`))
            .catch(error => console.warn(`Could not request access to resource folder ${directory.name}:`, error))))
        .then(() => { resourceFilesystemUpdated = true; });
}
export function take_resource_filesystem_update() {
    const updated = resourceFilesystemUpdated;
    resourceFilesystemUpdated = false;
    return updated;
}
export function resource_directory_handles() { return resourceDirectories; }
export function pick_resource_directory() {
  window.showDirectoryPicker().then(async handle => {
    resourceDirectories.push(handle);
    await saveResourceDirectories();
        resourceFilesystemUpdated = true;
  }).catch(() => {});
}
export function remove_resource_directory(index) {
  resourceDirectories.splice(index, 1);
  saveResourceDirectories().catch(() => {});
    resourceFilesystemUpdated = true;
}
export function move_resource_directory(index, destination) {
  [resourceDirectories[index], resourceDirectories[destination]] = [resourceDirectories[destination], resourceDirectories[index]];
  saveResourceDirectories().catch(() => {});
    resourceFilesystemUpdated = true;
}
