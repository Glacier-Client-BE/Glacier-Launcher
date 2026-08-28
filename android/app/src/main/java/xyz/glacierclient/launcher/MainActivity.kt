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
import androidx.core.content.edit
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
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

    private lateinit var insetsController: WindowInsetsControllerCompat

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
            addJavascriptInterface(AndroidBridge(this@MainActivity, this), "AndroidBridge")
            loadUrl("file:///android_asset/www/index.html")
        }
        setContentView(webView)
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) insetsController.hide(WindowInsetsCompat.Type.systemBars())
    }
}

private class AndroidBridge(private val activity: ComponentActivity, private val webView: WebView) {

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
    fun launchJavaEditionVersion(versionId: String) {
        activity.runOnUiThread { JavaEditionBridge.launch(activity, versionId) }
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
