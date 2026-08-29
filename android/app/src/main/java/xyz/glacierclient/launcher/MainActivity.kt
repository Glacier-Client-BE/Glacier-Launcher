package xyz.glacierclient.launcher

import android.annotation.SuppressLint
import android.app.Dialog
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.view.ViewGroup
import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.edit
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import xyz.glacierclient.launcher.service.BedrockBackupService
import xyz.glacierclient.launcher.service.BedrockStorageService
import xyz.glacierclient.launcher.service.ClientInjectionService
import xyz.glacierclient.launcher.service.DiscordRpcService
import xyz.glacierclient.launcher.service.JavaEditionBridge
import xyz.glacierclient.launcher.service.JavaInstanceService
import xyz.glacierclient.launcher.service.LevelDatService
import xyz.glacierclient.launcher.service.LauncherUpdateService
import xyz.glacierclient.launcher.service.ModpackInstallService
import java.io.File

/**
 * Hosts a single full-screen WebView loading assets/www/index.html — the same
 * app.css and image assets as the desktop app's wwwroot, so the UI is
 * pixel-identical rather than a Compose re-approximation. Navigation and all
 * panel rendering happen in JS (js/app.js, js/panels.js); this class is only
 * the native bridge for things a WebView genuinely cannot do itself (root
 * checks, launching other installed apps, and settings persistence, since
 * WebView's own localStorage isn't guaranteed durable across app updates).
 */
class MainActivity : ComponentActivity() {

    private lateinit var insetsController: WindowInsetsControllerCompat
    private lateinit var webView: WebView
    private var onBedrockStorageResult: ((Boolean) -> Unit)? = null

    // Must be registered before the activity reaches STARTED — a class-body
    // property initializer runs during construction, ahead of onCreate.
    private val openBedrockStorageTree = registerForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        var granted = false
        if (uri != null) {
            // WRITE as well as READ: the level.dat editor and the Bedrock
            // backup/pack panels open output streams through this same tree,
            // and a read-only persisted grant makes every one of those writes
            // fail at the ContentResolver.
            contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
            // The picker will happily hand back any folder the user taps, and
            // storing an unrelated one would leave the panels permanently
            // empty with no explanation. Only accept a tree the Bedrock
            // folders are actually reachable from.
            if (BedrockStorageService.isUsableBedrockTree(this, uri)) {
                BedrockStorageService.onAccessGranted(this, uri)
                granted = true
            }
        }
        onBedrockStorageResult?.invoke(granted)
        onBedrockStorageResult = null
    }

    // Called by AndroidBridge.requestBedrockStorageAccess(). The picker is
    // pre-pointed at Android/data/com.mojang.minecraftpe/.../com.mojang,
    // which is the only way to reach it: /Android cannot be navigated into
    // by hand in the picker on Android 11+, so launching with null (as this
    // did) left the folder genuinely unreachable. Passing a document Uri
    // opens the picker inside the restricted path instead — the same
    // approach MT Manager and other file managers use. See
    // BedrockStorageService.bedrockPickerInitialUri.
    fun requestBedrockStorageAccess(onResult: (Boolean) -> Unit) {
        onBedrockStorageResult = onResult
        openBedrockStorageTree.launch(BedrockStorageService.bedrockPickerInitialUri())
    }

    private var onSkinPngPicked: ((String?) -> Unit)? = null

    // Skin Library "Add PNG" — the button was disabled with the hint "Needs a
    // native file picker this app doesn't have yet". A skin is a small PNG, so
    // it is read straight to a base64 data URL rather than staged on disk:
    // js/skinlibrary.js already stores library entries as data URLs, and that
    // keeps the picked file out of app storage entirely.
    private val openSkinPngDocument = registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        onSkinPngPicked?.invoke(uri?.let { readAsDataUrl(it) })
        onSkinPngPicked = null
    }

    fun requestSkinPng(onResult: (String?) -> Unit) {
        onSkinPngPicked = onResult
        openSkinPngDocument.launch(arrayOf("image/png"))
    }

    /** Minecraft skins are 64x64/64x32 PNGs — a few KB. Capped anyway so a
     *  mis-picked large image can't be inlined into the WebView as base64. */
    private fun readAsDataUrl(uri: Uri): String? = runCatching {
        val bytes = contentResolver.openInputStream(uri)?.use { it.readBytes() } ?: return@runCatching null
        if (bytes.size > 2 * 1024 * 1024) return@runCatching null
        "data:image/png;base64," + android.util.Base64.encodeToString(bytes, android.util.Base64.NO_WRAP)
    }.getOrNull()

    private var onCustomDllPicked: ((String?) -> Unit)? = null

    // Mirrors desktop's PickDllFile (Pages/Home.Settings.cs) — a plain
    // OpenFileDialog there, SAF's single-document picker here. The picked
    // file only ever exists as a content:// Uri, not a real filesystem path,
    // so this copies it into app-private storage (filesDir) and hands back
    // that real path — ClientInjectionService.attemptInject's root shell
    // command needs an actual path, same as it already does for the
    // (still-manual, no UI yet before this) staging fallback.
    private val openCustomDllDocument = registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val staged = uri?.let { stageCustomDll(it) }
        onCustomDllPicked?.invoke(staged)
        onCustomDllPicked = null
    }

    fun requestCustomDllFile(onResult: (String?) -> Unit) {
        onCustomDllPicked = onResult
        // "*/*": .so isn't a recognized MIME type on Android's own database,
        // so a narrower filter would silently hide valid files on many OEMs.
        openCustomDllDocument.launch(arrayOf("*/*"))
    }

    private var onWallpaperPicked: ((String?) -> Unit)? = null

    // Mirrors desktop's PickWallpaper (Pages/Home.Settings.cs) — an
    // OpenFileDialog filtered to image types there, SAF's single-document
    // picker here. Same as the DLL picker above, the chosen file only exists
    // as a content:// Uri, so it is copied into app-private storage and the
    // real path handed back; the WebView can then reference it as a file://
    // URL for the CSS background (settings.allowFileAccess is already on).
    private val openWallpaperDocument = registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        val staged = uri?.let { stageWallpaper(it) }
        onWallpaperPicked?.invoke(staged)
        onWallpaperPicked = null
    }

    fun requestWallpaperFile(onResult: (String?) -> Unit) {
        onWallpaperPicked = onResult
        openWallpaperDocument.launch(arrayOf("image/*"))
    }

    /**
     * Copies the picked image to filesDir/wallpaper/custom-bg.<ext>,
     * replacing any previous one. Enforces desktop's own 20 MB ceiling
     * (OnWallpaperPicked) — a phone has less headroom than a desktop, and
     * decoding an arbitrarily large bitmap into a WebView layer is the kind
     * of thing that OOMs on low-end devices.
     */
    private fun stageWallpaper(uri: Uri): String? = runCatching {
        contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            val sizeIdx = cursor.getColumnIndex(android.provider.OpenableColumns.SIZE)
            if (cursor.moveToFirst() && sizeIdx >= 0 && !cursor.isNull(sizeIdx)) {
                if (cursor.getLong(sizeIdx) > 20L * 1024 * 1024) return@runCatching null
            }
        }

        val extension = contentResolver.getType(uri)
            ?.let { android.webkit.MimeTypeMap.getSingleton().getExtensionFromMimeType(it) }
            ?: "png"

        val dir = File(filesDir, "wallpaper").apply { mkdirs() }
        // Only one custom wallpaper is kept, so clear older ones rather than
        // letting a different extension leave the previous file orphaned.
        dir.listFiles()?.forEach { it.delete() }

        val dest = File(dir, "custom-bg.$extension")
        contentResolver.openInputStream(uri)?.use { input ->
            dest.outputStream().use { output -> input.copyTo(output) }
        } ?: return@runCatching null
        dest.absolutePath
    }.getOrNull()

    private fun stageCustomDll(uri: Uri): String? = runCatching {
        val name = contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            val idx = cursor.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
            if (cursor.moveToFirst() && idx >= 0) cursor.getString(idx) else null
        } ?: "custom_client.so"
        val stagingDir = File(filesDir, "custom_dll").apply { mkdirs() }
        val dest = File(stagingDir, name)
        contentResolver.openInputStream(uri)?.use { input -> dest.outputStream().use { output -> input.copyTo(output) } }
        dest.absolutePath
    }.getOrNull()

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Fully immersive: the app draws its own top-bar/footer chrome, so
        // the system status bar and 3-button/gesture nav bar are just dead
        // space at the edges of a phone screen. Swiping from an edge still
        // reveals them temporarily (BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE).
        WindowCompat.setDecorFitsSystemWindows(window, false)
        insetsController = WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }

        webView = WebView(this).apply {
            settings.javaScriptEnabled = true
            settings.domStorageEnabled = true
            settings.allowFileAccess = true
            // The UI is laid out against the viewport meta tag in index.html
            // (device-width, initial-scale=1), not WebView's legacy zoom
            // scaling — pinch/double-tap zoom would just fight the app's own
            // fixed-chrome layout, so it's disabled like a native app's UI.
            settings.useWideViewPort = true
            settings.loadWithOverviewMode = true
            settings.setSupportZoom(false)
            settings.builtInZoomControls = false
            addJavascriptInterface(AndroidBridge(this@MainActivity, this), "AndroidBridge")
            loadUrl("file:///android_asset/www/index.html")
        }
        setContentView(webView)

        // Reconnects presence across app restarts. No-ops unless the user
        // has explicitly turned it on AND stored a token — see
        // DiscordRpcService's doc comment for why it is opt-in.
        DiscordRpcService.startIfEnabled(this)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) insetsController.hide(WindowInsetsCompat.Type.systemBars())
    }

    // Lightweight session-timer heuristic for the Stats panel (see
    // js/app.js's recordLaunchStart()/onResumeFromGame()): Android has no
    // callback for "the app I launched via startActivity() has closed", so
    // this is the best available signal — Bedrock/Java Edition run as their
    // own Activity on top of this one, and this Activity's onResume fires
    // again once the user backs out of it, same approximation real
    // launchers without a play-time API from the OS have to make.
    override fun onResume() {
        super.onResume()
        if (::webView.isInitialized) {
            webView.evaluateJavascript("window.App && window.App.onResumeFromGame && window.App.onResumeFromGame()", null)
        }
    }
}

private class AndroidBridge(private val activity: MainActivity, private val webView: WebView) {

    private val prefs = activity.getSharedPreferences("glacier_settings", 0)

    // Same legacy Microsoft Live OAuth flow the desktop app's own
    // LiveAuthWindow.xaml.cs/LiveAuthService.cs use — client_id and the
    // "desktop" redirect URI are both public, non-secret constants already
    // committed in this repo (they identify the community-launcher OAuth
    // flow itself, not a per-developer credential). A second WebView in a
    // Dialog hosts the Microsoft sign-in pages; once navigation reaches the
    // redirect URI, the "code" query param is pulled out here (server-side,
    // no custom URL scheme needed) and handed back to the main page's JS,
    // which does the rest (token exchange, XBL/XSTS, Minecraft auth) via
    // fetch() calls — see js/xboxauth.js.
    @JavascriptInterface
    fun signInMicrosoft() {
        activity.runOnUiThread {
            val authUrl = Uri.parse("https://login.live.com/oauth20_authorize.srf")
                .buildUpon()
                .appendQueryParameter("client_id", "00000000402b5328")
                .appendQueryParameter("response_type", "code")
                .appendQueryParameter("scope", "service::user.auth.xboxlive.com::MBI_SSL")
                .appendQueryParameter("redirect_uri", "https://login.live.com/oauth20_desktop.srf")
                .appendQueryParameter("display", "touch")
                .build()
                .toString()

            val dialog = Dialog(activity, android.R.style.Theme_Black_NoTitleBar_Fullscreen)
            val authWebView = WebView(activity).apply {
                settings.javaScriptEnabled = true
                settings.domStorageEnabled = true
            }
            dialog.setContentView(authWebView, ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
            dialog.setOnCancelListener { notifySignInResult(null, "Sign-in was cancelled.") }

            authWebView.webViewClient = object : WebViewClient() {
                override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                    val uri = request.url
                    if (uri.toString().startsWith("https://login.live.com/oauth20_desktop.srf")) {
                        val code = uri.getQueryParameter("code")
                        val error = uri.getQueryParameter("error_description") ?: uri.getQueryParameter("error")
                        dialog.dismiss()
                        if (code != null) notifySignInResult(code, null)
                        else notifySignInResult(null, error ?: "Sign-in failed.")
                        return true
                    }
                    return false
                }
            }
            authWebView.loadUrl(authUrl)
            dialog.show()
        }
    }

    private fun notifySignInResult(code: String?, error: String?) {
        activity.runOnUiThread {
            val js = if (code != null)
                "window.MicrosoftAuth && window.MicrosoftAuth._onCode(${org.json.JSONObject.quote(code)})"
            else
                "window.MicrosoftAuth && window.MicrosoftAuth._onError(${org.json.JSONObject.quote(error ?: "Sign-in failed.")})"
            webView.evaluateJavascript(js, null)
        }
    }

    // Same OAuth2 authorization-code flow the desktop app's own
    // OpenDiscordOAuth() (Pages/Home.Panels.cs) uses against a real local
    // HTTP listener on 127.0.0.1:5000/callback — this is just an "identify"
    // scope login for the profile switcher's username/avatar, unrelated to
    // Discord Rich Presence (which rides a local Discord-desktop IPC pipe
    // that has no Android equivalent, see ClientInjectionService.kt's own
    // caveat for the same class of limitation). client_id/client_secret
    // here are the same non-secret-in-practice constants already committed
    // in plain text in Pages/Home.Panels.cs — porting them here doesn't
    // change their exposure. A second WebView in a Dialog hosts Discord's
    // real login page; once navigation reaches the redirect URI, the
    // "code" query param is pulled out here and handed back to the main
    // page's JS, which does the token exchange + profile fetch via
    // fetch() — see js/discordauth.js.
    @JavascriptInterface
    fun signInDiscord() {
        activity.runOnUiThread {
            val authUrl = Uri.parse("https://discord.com/api/oauth2/authorize")
                .buildUpon()
                .appendQueryParameter("client_id", "1482726422094024779")
                .appendQueryParameter("response_type", "code")
                .appendQueryParameter("redirect_uri", "http://localhost:5000/callback")
                .appendQueryParameter("scope", "identify")
                .build()
                .toString()

            val dialog = Dialog(activity, android.R.style.Theme_Black_NoTitleBar_Fullscreen)
            val authWebView = WebView(activity).apply {
                settings.javaScriptEnabled = true
                settings.domStorageEnabled = true
            }
            dialog.setContentView(authWebView, ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT))
            dialog.setOnCancelListener { notifyDiscordSignInResult(null, "Sign-in was cancelled.") }

            authWebView.webViewClient = object : WebViewClient() {
                override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                    val uri = request.url
                    if (uri.toString().startsWith("http://localhost:5000/callback")) {
                        val code = uri.getQueryParameter("code")
                        val error = uri.getQueryParameter("error_description") ?: uri.getQueryParameter("error")
                        dialog.dismiss()
                        if (code != null) notifyDiscordSignInResult(code, null)
                        else notifyDiscordSignInResult(null, error ?: "Sign-in failed.")
                        return true
                    }
                    return false
                }
            }
            authWebView.loadUrl(authUrl)
            dialog.show()
        }
    }

    private fun notifyDiscordSignInResult(code: String?, error: String?) {
        activity.runOnUiThread {
            val js = if (code != null)
                "window.DiscordAuth && window.DiscordAuth._onCode(${org.json.JSONObject.quote(code)})"
            else
                "window.DiscordAuth && window.DiscordAuth._onError(${org.json.JSONObject.quote(error ?: "Sign-in failed.")})"
            webView.evaluateJavascript(js, null)
        }
    }

    @JavascriptInterface
    fun isRootAvailable(): Boolean = ClientInjectionService.isRootAvailable()

    @JavascriptInterface
    fun attemptInject(path: String): String =
        ClientInjectionService.attemptInject(path).message

    @JavascriptInterface
    fun launchJavaEdition() {
        activity.runOnUiThread { JavaEditionBridge.launch(activity) }
    }

    @JavascriptInterface
    fun launchJavaEditionVersion(versionId: String) {
        activity.runOnUiThread { JavaEditionBridge.launch(activity, versionId) }
    }

    @JavascriptInterface
    fun launchBedrock() {
        activity.runOnUiThread {
            val intent = activity.packageManager.getLaunchIntentForPackage("com.mojang.minecraftpe")
            if (intent != null) activity.startActivity(intent)
        }
    }

    @JavascriptInterface
    fun getSettingsJson(): String = prefs.getString("settings_json", "{}") ?: "{}"

    @JavascriptInterface
    fun saveSettingsJson(json: String) {
        prefs.edit { putString("settings_json", json) }
    }

    @JavascriptInterface
    fun curseForgeApiKey(): String =
        BuildConfig.CURSEFORGE_API_KEY.ifBlank {
            runCatching {
                org.json.JSONObject(getSettingsJson()).optString("curseForgeApiKeyOverride", "")
            }.getOrDefault("")
        }

    @JavascriptInterface
    fun appVersionName(): String = BuildConfig.VERSION_NAME

    // Downloads the update APK named by js/updater.js's GitHub-releases check
    // and hands it to the system package installer — see
    // LauncherUpdateService.kt for why Android can't silently self-replace
    // the way AutoUpdateService.cs's exe swap does.
    @JavascriptInterface
    fun downloadAndInstallUpdate(url: String, tag: String) {
        activity.runOnUiThread {
            LauncherUpdateService.downloadAndInstall(
                context = activity,
                url = url,
                tag = tag,
                onProgress = { pct ->
                    activity.runOnUiThread {
                        webView.evaluateJavascript("window.LauncherUpdate && window.LauncherUpdate._onProgress($pct)", null)
                    }
                },
                onError = { message ->
                    activity.runOnUiThread {
                        webView.evaluateJavascript("window.LauncherUpdate && window.LauncherUpdate._onError(${org.json.JSONObject.quote(message)})", null)
                    }
                },
            )
        }
    }

    // Bedrock world listing (Home.Worlds.cs's read half) — see
    // BedrockStorageService.kt/BedrockNbt.kt for why this needs a
    // one-time SAF grant instead of a plain file path.
    @JavascriptInterface
    fun hasBedrockStorageAccess(): Boolean = BedrockStorageService.hasAccess(activity)

    @JavascriptInterface
    fun requestBedrockStorageAccess() {
        activity.runOnUiThread {
            activity.requestBedrockStorageAccess { granted ->
                webView.evaluateJavascript("window.BedrockStorage && window.BedrockStorage._onAccessResult($granted)", null)
            }
        }
    }

    /**
     * Version-aware explanation for the storage-access UI, so a failure that
     * the OS makes impossible isn't presented as something the user did
     * wrong. Android 13 closed the Android/data subdirectory loophole the
     * picker relies on.
     */
    @JavascriptInterface
    fun bedrockStorageBlockedByPlatform(): Boolean = BedrockStorageService.accessBlockedByPlatform()

    @JavascriptInterface
    fun listBedrockWorlds(): String = BedrockStorageService.listWorlds(activity)

    @JavascriptInterface
    fun listBedrockPacks(kind: String): String = BedrockStorageService.listPacks(activity, kind)

    @JavascriptInterface
    fun listBedrockScreenshots(): String = BedrockStorageService.listScreenshots(activity)

    @JavascriptInterface
    fun openBedrockFolder(name: String) {
        activity.runOnUiThread {
            val uriString = BedrockStorageService.folderUri(activity, name) ?: return@runOnUiThread
            runCatching {
                activity.startActivity(
                    Intent(Intent.ACTION_VIEW)
                        .setDataAndType(Uri.parse(uriString), "vnd.android.document/directory")
                        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),
                )
            }
        }
    }

    @JavascriptInterface
    fun listBedrockBackups(): String = BedrockBackupService.listBackups(activity)

    @JavascriptInterface
    fun createBedrockBackup(): String = BedrockBackupService.createBackup(activity)

    @JavascriptInterface
    fun deleteBedrockBackup(fileName: String): Boolean = BedrockBackupService.deleteBackup(activity, fileName)

    // Java multi-instance management (Home.Panels.cs's Modpack "Install" and
    // several Java Addons actions are disabled on Android for lack of this —
    // see JavaInstanceService.kt for how it reuses Pojav's own profile system).
    @JavascriptInterface
    fun listJavaInstances(): String = JavaInstanceService.list()

    @JavascriptInterface
    fun createJavaInstance(name: String, versionId: String): String = JavaInstanceService.create(name, versionId)

    @JavascriptInterface
    fun renameJavaInstance(id: String, newName: String): Boolean = JavaInstanceService.rename(id, newName)

    @JavascriptInterface
    fun deleteJavaInstance(id: String): Boolean = JavaInstanceService.delete(id)

    @JavascriptInterface
    fun setActiveJavaInstance(id: String): Boolean = JavaInstanceService.setActive(id)

    // Modrinth-only modpack install (see ModpackInstallService.kt for why
    // CurseForge and loader installation aren't covered here yet).
    @JavascriptInterface
    fun installModrinthPack(mrpackUrl: String, packName: String): String =
        ModpackInstallService.installModrinthPack(activity, mrpackUrl, packName)

    // Custom Bedrock client .so picker — mirrors desktop's PickDllFile/
    // CopyDllPath/ClearCustomDll (Home.Settings.cs/Home.Clients.cs). The
    // staged path is only usable via ClientInjectionService's existing
    // root-based staging fallback (see its own doc comment); this doesn't
    // change what injection can actually do, only lets a user reach it with
    // a real file instead of no picker existing at all.
    @JavascriptInterface
    fun pickCustomDllFile() {
        activity.runOnUiThread {
            activity.requestCustomDllFile { path ->
                val js = if (path != null)
                    "window.CustomDllPicker && window.CustomDllPicker._onPicked(${org.json.JSONObject.quote(path)})"
                else
                    "window.CustomDllPicker && window.CustomDllPicker._onPicked(null)"
                webView.evaluateJavascript(js, null)
            }
        }
    }

    // Custom wallpaper — desktop's PickWallpaper/ResetWallpaper
    // (Pages/Home.Settings.cs). The staged path is returned as a file:// URL
    // because that is what CSS background-image needs; the WebView is
    // already configured with allowFileAccess = true, and the file lives in
    // app-private storage that no other app can read.
    @JavascriptInterface
    fun pickWallpaper() {
        activity.runOnUiThread {
            activity.requestWallpaperFile { path ->
                val js = if (path != null)
                    "window.Theme && window.Theme._onWallpaperPicked(${org.json.JSONObject.quote("file://$path")})"
                else
                    "window.Theme && window.Theme._onWallpaperPicked(null)"
                webView.evaluateJavascript(js, null)
            }
        }
    }

    /** Current wallpaper as a file:// URL, or "" when none is set. */
    @JavascriptInterface
    fun customBackgroundUrl(): String {
        val dir = File(activity.filesDir, "wallpaper")
        val file = dir.listFiles()?.firstOrNull { it.isFile } ?: return ""
        return "file://${file.absolutePath}"
    }

    /** Desktop's ResetWallpaper: clears the choice and falls back to the bundled bg. */
    @JavascriptInterface
    fun resetWallpaper() {
        runCatching { File(activity.filesDir, "wallpaper").listFiles()?.forEach { it.delete() } }
    }

    // Skin Library "Add PNG" (desktop's own file-dialog import).
    @JavascriptInterface
    fun pickSkinPng() {
        activity.runOnUiThread {
            activity.requestSkinPng { dataUrl ->
                val js = if (dataUrl != null)
                    "window.SkinLibrary && window.SkinLibrary._onPngPicked(${org.json.JSONObject.quote(dataUrl)})"
                else
                    "window.SkinLibrary && window.SkinLibrary._onPngPicked(null)"
                webView.evaluateJavascript(js, null)
            }
        }
    }

    // level.dat editor (desktop's LevelDatEditorService.cs). Keyed by the
    // world id listBedrockWorlds() reports, not its folderUri — see
    // BedrockStorageService.worldDir for why.
    @JavascriptInterface
    fun levelDatSummary(worldId: String): String = LevelDatService.summary(activity, worldId)

    @JavascriptInterface
    fun saveLevelDat(worldId: String, patchJson: String): String =
        LevelDatService.save(activity, worldId, patchJson)

    // Java asset folders (desktop's Open*Folder shortcuts). Java data is a
    // real path under Pojav's storage, not behind SAF — see
    // JavaInstanceService.listAssetFolders.
    @JavascriptInterface
    fun listJavaAssetFolders(): String = JavaInstanceService.listAssetFolders()

    /** Best-effort, like desktop's own shortcut: silently does nothing if no file manager handles it. */
    @JavascriptInterface
    fun openJavaAssetFolder(folder: String) {
        activity.runOnUiThread {
            val uri = JavaInstanceService.assetFolderDocumentUri(folder) ?: return@runOnUiThread
            runCatching {
                activity.startActivity(
                    Intent(Intent.ACTION_VIEW)
                        .setDataAndType(Uri.parse(uri), "vnd.android.document/directory")
                        .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION),
                )
            }
        }
    }

    // ── Java Tools (desktop's JavaInstanceService.cs) ────────────────
    // Each returns the produced path, or "" when there was nothing to do,
    // so the UI can report the real outcome instead of guessing.

    @JavascriptInterface
    fun backupJavaSaves(): String = JavaInstanceService.backupSaves() ?: ""

    @JavascriptInterface
    fun exportJavaModpack(): String = JavaInstanceService.exportModpack() ?: ""

    @JavascriptInterface
    fun duplicateJavaInstance(id: String): String = JavaInstanceService.duplicate(id) ?: ""

    // ── Discord Rich Presence ────────────────────────────────────────
    //
    // The desktop app's DiscordRpcService.cs uses a local IPC pipe to the
    // Discord desktop client, which does not exist on Android; the only
    // on-device route is a Gateway WebSocket authenticated with the user's
    // own account token, which is self-botting. Hence opt-in, off by
    // default, and the token is never fetched automatically. See
    // DiscordRpcService.kt.

    @JavascriptInterface
    fun discordRpcEnabled(): Boolean = DiscordRpcService.isEnabled(activity)

    /**
     * True once a token has been stored. Deliberately reports only whether
     * one exists rather than returning it: the token is full account
     * credentials, and there is no reason to hand it back out into the
     * WebView's DOM where any page script could read it.
     */
    @JavascriptInterface
    fun discordRpcHasToken(): Boolean = DiscordRpcService.savedToken(activity).isNotBlank()

    /** Blank [token] keeps the stored one, so re-enabling needs no re-paste. */
    @JavascriptInterface
    fun discordRpcConfigure(enabled: Boolean, token: String) {
        DiscordRpcService.configure(activity, enabled, token)
    }

    @JavascriptInterface
    fun discordRpcStatus(): String = DiscordRpcService.statusJson()

    @JavascriptInterface
    fun discordRpcSetIdle() = DiscordRpcService.setIdlePresence()

    @JavascriptInterface
    fun discordRpcSetBedrock(versionTag: String, clientName: String) =
        DiscordRpcService.setBedrockPresence(versionTag, clientName)

    @JavascriptInterface
    fun discordRpcSetJava(versionId: String, variant: String) =
        DiscordRpcService.setJavaPresence(versionId, variant)

    @JavascriptInterface
    fun openUrl(url: String) {
        activity.runOnUiThread {
            runCatching { activity.startActivity(Intent(Intent.ACTION_VIEW, android.net.Uri.parse(url))) }
        }
    }

    // Desktop's .window-controls close button — the minimize/maximize/
    // fullscreen ones alongside it have no Android equivalent (no
    // windowing) and aren't ported; close still means something: exit.
    @JavascriptInterface
    fun closeApp() {
        activity.runOnUiThread { activity.finishAffinity() }
    }
}
