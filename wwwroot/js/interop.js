window.startDrag      = () => postToHost('startDrag');
window.closeWindow    = () => postToHost('close');
window.minimizeWindow = () => postToHost('minimize');
window.maximizeWindow = () => postToHost('maximize');

function postToHost(msg) {
    if (window.chrome?.webview) window.chrome.webview.postMessage(msg);
}

// Register .NET drop callback
window.glacierRegisterDropHandler = (dotNetRef) => {
    window._glacierDotNet = dotNetRef;
};

// Open native file picker for DLL (returns path via callback)
window.pickDllFile = async (dotNetRef) => {
    // WebView2 doesn't expose a native file picker directly,
    // so we create a temporary <input type="file"> and read the path
    return new Promise(resolve => {
        const input = document.createElement('input');
        input.type   = 'file';
        input.accept = '.dll';
        input.style.display = 'none';
        document.body.appendChild(input);

        input.onchange = () => {
            const file = input.files[0];
            if (file) {
                const path = file.path || file.name;
                dotNetRef.invokeMethodAsync('OnDllDropped', path);
            }
            document.body.removeChild(input);
            resolve();
        };
        input.oncancel = () => { document.body.removeChild(input); resolve(); };
        input.click();
    });
};

// Drag-and-drop DLL anywhere on the window
document.addEventListener('dragover', e => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
    document.body.classList.add('drop-active');
});

document.addEventListener('dragleave', e => {
    if (!e.relatedTarget || e.relatedTarget === document.documentElement)
        document.body.classList.remove('drop-active');
});

document.addEventListener('drop', e => {
    e.preventDefault();
    document.body.classList.remove('drop-active');

    const files = Array.from(e.dataTransfer.files || [])
        .filter(f => f.name.toLowerCase().endsWith('.dll'));

    if (!files.length) return;

    const path = files[0].path;
    if (path && window._glacierDotNet)
        window._glacierDotNet.invokeMethodAsync('OnDllDropped', path);
});
