use wasm_bindgen::prelude::*;

// Inline JS to recursively retrieve files (preserving webkitRelativePath) from drag-and-drop DataTransfer.
#[wasm_bindgen(inline_js = r#"
export async function getFilesFromDataTransfer(dataTransfer) {
    const items = dataTransfer.items;
    if (!items) return [];
    
    let fileEntries = [];
    let rootFolderName = null;

    async function traverseEntry(entry, path = '') {
        if (entry.isFile) {
            const file = await new Promise((resolve, reject) => {
                entry.file((file) => {
                    if (!rootFolderName && path) {
                        rootFolderName = path.split('/')[0];
                    }
                    const fullPath = path ? (path + '/' + entry.name) : entry.name;
                    const fileWithPath = new File([file], entry.name, {
                        type: file.type,
                        lastModified: file.lastModified,
                    });

                    if (rootFolderName) {
                        const relativePath = fullPath.startsWith(rootFolderName)
                            ? fullPath
                            : (rootFolderName + '/' + fullPath);
                        Object.defineProperty(fileWithPath, 'webkitRelativePath', {
                            value: relativePath,
                            writable: false,
                            configurable: true,
                        });
                    } else {
                        Object.defineProperty(fileWithPath, 'webkitRelativePath', {
                            value: fullPath,
                            writable: false,
                            configurable: true,
                        });
                    }
                    resolve(fileWithPath);
                }, reject);
            });
            fileEntries.push(file);
        } else if (entry.isDirectory) {
            if (!path && !rootFolderName) {
                rootFolderName = entry.name;
            }
            const dirReader = entry.createReader();
            let entries = [];

            let readEntries = await new Promise((resolve, reject) => {
                const readNextBatch = () => {
                    dirReader.readEntries((batch) => {
                        if (batch.length > 0) {
                            entries = entries.concat(batch);
                            readNextBatch();
                        } else {
                            resolve(entries);
                        }
                    }, reject);
                };
                readNextBatch();
            });

            const dirPath = path ? (path + '/' + entry.name) : entry.name;
            for (const childEntry of entries) {
                await traverseEntry(childEntry, dirPath);
            }
        }
    }

    for (const item of items) {
        if (item.webkitGetAsEntry) {
            const entry = item.webkitGetAsEntry();
            if (entry) {
                await traverseEntry(entry);
            }
        }
    }

    fileEntries.sort((a, b) => a.webkitRelativePath.localeCompare(b.webkitRelativePath));
    return fileEntries;
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = getFilesFromDataTransfer, catch)]
    pub async fn get_files_from_data_transfer(data_transfer: &web_sys::DataTransfer) -> Result<js_sys::Array, JsValue>;
}

#[wasm_bindgen(inline_js = r#"
export function copyTextToClipboard(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text);
        return true;
    }
    const textArea = document.createElement("textarea");
    textArea.value = text;
    textArea.style.position = "fixed";
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();
    try {
        const successful = document.execCommand('copy');
        document.body.removeChild(textArea);
        return successful;
    } catch (err) {
        document.body.removeChild(textArea);
        return false;
    }
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = copyTextToClipboard)]
    pub fn copy_text_to_clipboard(text: &str) -> bool;
}
