// Glacier Launcher – native window interop
// These functions communicate with the WPF host via WebView2's postMessage API.

window.startDrag = function () {
    if (window.chrome && window.chrome.webview) {
        window.chrome.webview.postMessage('startDrag');
    }
};

window.closeWindow = function () {
    if (window.chrome && window.chrome.webview) {
        window.chrome.webview.postMessage('close');
    }
};

window.minimizeWindow = function () {
    if (window.chrome && window.chrome.webview) {
        window.chrome.webview.postMessage('minimize');
    }
};
