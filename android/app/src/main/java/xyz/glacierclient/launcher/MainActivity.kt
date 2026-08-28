package xyz.glacierclient.launcher

import android.annotation.SuppressLint
import android.content.Intent
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.ComponentActivity
import androidx.core.content.edit
import xyz.glacierclient.launcher.service.ClientInjectionService
import xyz.glacierclient.launcher.service.JavaEditionBridge

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

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val webView = WebView(this).apply {
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
            addJavascriptInterface(AndroidBridge(this@MainActivity), "AndroidBridge")
            loadUrl("file:///android_asset/www/index.html")
        }
        setContentView(webView)
    }
}

private class AndroidBridge(private val activity: ComponentActivity) {

    private val prefs = activity.getSharedPreferences("glacier_settings", 0)

    @JavascriptInterface
    fun isRootAvailable(): Boolean = ClientInjectionService.isRootAvailable()

    @JavascriptInterface
    fun attemptInject(path: String): String =
        ClientInjectionService.attemptInject(path).message

    @JavascriptInterface
    fun isJavaEditionInstalled(): Boolean = JavaEditionBridge.isInstalled(activity)

    @JavascriptInterface
    fun launchJavaEdition() {
        activity.runOnUiThread { JavaEditionBridge.launch(activity) }
    }

    @JavascriptInterface
    fun hasBundledJavaEditionInstaller(): Boolean = JavaEditionBridge.hasBundledInstaller(activity)

    @JavascriptInterface
    fun installJavaEditionCompanion() {
        activity.runOnUiThread { JavaEditionBridge.installBundled(activity) }
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

    @JavascriptInterface
    fun openUrl(url: String) {
        activity.runOnUiThread {
            runCatching { activity.startActivity(Intent(Intent.ACTION_VIEW, android.net.Uri.parse(url))) }
        }
    }
}
