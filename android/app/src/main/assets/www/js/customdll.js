// Bridges to MainActivity.kt's pickCustomDllFile() — mirrors desktop's
// PickDllFile (Pages/Home.Settings.cs), a plain OpenFileDialog there, a SAF
// single-document picker + app-private-storage staging here (see
// MainActivity.kt's requestCustomDllFile/stageCustomDll for why the staging
// copy is needed: a SAF pick only ever hands back a content:// Uri, and
// ClientInjectionService's root shell command needs a real path).
const CustomDllPicker = {
    _pending: null,

    pick() {
        if (!Bridge.pickCustomDllFile) {
            return Promise.reject(new Error("File picker isn't available in this preview (no native bridge)."));
        }
        return new Promise((resolve) => {
            this._pending = resolve;
            Bridge.pickCustomDllFile();
        });
    },

    // Called from native (MainActivity.kt's pickCustomDllFile) once the SAF
    // document picker returns — path is null if the user cancelled.
    _onPicked(path) {
        if (!this._pending) return;
        const resolve = this._pending;
        this._pending = null;
        resolve(path);
    },
};
// See js/xboxauth.js's window.MicrosoftAuth comment — MainActivity.kt calls
// back via window.CustomDllPicker, which needs an explicit assignment.
window.CustomDllPicker = CustomDllPicker;
