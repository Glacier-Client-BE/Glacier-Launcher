@file:Suppress("unused")
package xyz.glacierclient.launcher.utils

import android.content.Context
import androidx.annotation.WorkerThread
import com.microsoft.identity.client.AcquireTokenInteractiveParameters
import com.microsoft.identity.client.AuthenticationCallback
import com.microsoft.identity.client.IAccount
import com.microsoft.identity.client.IAuthenticationResult
import com.microsoft.identity.client.IPublicClientApplication
import com.microsoft.identity.client.ISingleAccountPublicClientApplication
import com.microsoft.identity.client.PublicClientApplication
import com.microsoft.identity.client.SilentAuthenticationCallback
import com.microsoft.identity.client.exception.MsalException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException

/**
 * Alternative, Microsoft/Xbox-account path for proving Minecraft Bedrock
 * ownership, sitting ALONGSIDE (never replacing) the Google-account path in
 * GPlayAPI.kt/PlayStoreValidator.kt. A user picks whichever account type
 * they actually own Bedrock through — Play Store purchase (GPlayAPI) or a
 * Microsoft account with Bedrock/Xbox entitlement (this file).
 *
 * The MainActivity.signInMicrosoft() WebView flow already committed in this
 * repo drives Microsoft's legacy oauth20_authorize.srf code flow and hands
 * the "code" back to JS (js/xboxauth.js) for the rest. This file is a
 * *separate*, MSAL-based interactive flow: Microsoft's official
 * com.microsoft.identity.client Android library handles the browser/
 * WebView chrome and token cache itself, and this object does the public,
 * standard Xbox Live v3.0 XBL -> XSTS token exchange documented at
 * https://wiki.vg/Microsoft_Authentication_Scheme (a community wiki
 * describing Mojang's own published auth flow — not proprietary to any
 * third-party app). Nothing here was derived from, or needs, any
 * decompiled third-party APK; it's built directly against MSAL's public
 * API and Xbox Live's public token endpoints.
 *
 * The resulting XSTS user hash + token (and MSAL's own refresh token,
 * which MSAL persists in its own encrypted cache — this object additionally
 * mirrors the XSTS token pair into a private SharedPreferences file so the
 * rest of the app can check "is a Microsoft account signed in" the same
 * synchronous way GPlayAPI.getAuthDataOrNull() does for Google accounts)
 * lives in "msAccountData", which — like GPlayAPI's "accountData" — is
 * excluded from Android backup/device-transfer in backup_rules.xml and
 * data_extraction_rules.xml so it can never leave the device via a cloud
 * backup or "transfer to new phone" restore.
 */
object MicrosoftAuth {
    private const val PREFS_NAME = "msAccountData"

    // ---------------------------------------------------------------------
    // Azure AD app registration. This MUST be a real client ID from an
    // Azure AD app registration owned by the Glacier project (portal.azure.com
    // -> App registrations -> New registration -> add the "Xbox Live"
    // delegated permission / add https://login.microsoftonline.com/consumers
    // as the authority, and register a "Mobile and desktop applications"
    // platform with the msal<CLIENT_ID>://auth redirect). Only the project
    // owner can create this — it is intentionally left as a placeholder
    // rather than a fabricated-looking value; sign-in will fail with a
    // clear MsalException until it's filled in.
    //
    // TODO(project owner): replace with the real Azure AD application
    // (client) ID, and update AndroidManifest.xml's BrowserTabActivity
    // intent-filter data scheme (msal<CLIENT_ID>) to match.
    const val AZURE_CLIENT_ID = "00000000-0000-0000-0000-000000000000"

    private const val AUTHORITY = "https://login.microsoftonline.com/consumers"
    private const val XBL_SCOPE = "XboxLive.signin offline_access"

    private val httpClient = OkHttpClient()
    private val jsonMedia = "application/json".toMediaType()

    data class XboxSession(
        val xstsToken: String,
        val userHash: String,
        val minecraftEntitled: Boolean,
    )

    private var pca: ISingleAccountPublicClientApplication? = null

    /**
     * Interactive MSAL sign-in (browser/Custom Tabs chrome, handled entirely
     * by MSAL) followed by the standard XBL -> XSTS exchange. Must be called
     * from a coroutine; the interactive prompt itself still needs the
     * Activity for its launch, passed as [activity].
     */
    suspend fun signIn(activity: android.app.Activity): XboxSession = withContext(Dispatchers.Main) {
        val app = withContext(Dispatchers.IO) {
            PublicClientApplication.createSingleAccountPublicClientApplication(
                activity,
                xyz.glacierclient.launcher.R.raw.msal_config,
            )
        }
        pca = app

        val msResult = suspendCancellableCoroutine<IAuthenticationResult> { cont ->
            val params = AcquireTokenInteractiveParameters.Builder()
                .startAuthorizationFromActivity(activity)
                .withScopes(listOf("XboxLive.signin", "offline_access"))
                .withCallback(object : AuthenticationCallback {
                    override fun onSuccess(result: IAuthenticationResult) = cont.resume(result)
                    override fun onError(exception: MsalException) = cont.resumeWithException(exception)
                    override fun onCancel() = cont.resumeWithException(
                        IllegalStateException("Microsoft sign-in was cancelled.")
                    )
                })
                .build()
            app.acquireToken(params)
        }

        withContext(Dispatchers.IO) {
            exchangeForXsts(msResult.accessToken, activity.applicationContext)
        }
    }

    /** Standard, publicly documented XBL -> XSTS -> (Minecraft entitlement) chain. */
    @WorkerThread
    private fun exchangeForXsts(msAccessToken: String, context: Context): XboxSession {
        val xblBody = JSONObject().apply {
            put("Properties", JSONObject().apply {
                put("AuthMethod", "RPS")
                put("SiteName", "user.auth.xboxlive.com")
                put("RpsTicket", "d=$msAccessToken")
            })
            put("RelyingParty", "http://auth.xboxlive.com")
            put("TokenType", "JWT")
        }
        val xblResp = postJson("https://user.auth.xboxlive.com/user/authenticate", xblBody)
        val xblToken = xblResp.getString("Token")
        val uhs = xblResp.getJSONObject("DisplayClaims")
            .getJSONArray("xui").getJSONObject(0).getString("uhs")

        val xstsBody = JSONObject().apply {
            put("Properties", JSONObject().apply {
                put("SandboxId", "RETAIL")
                put("UserTokens", org.json.JSONArray().put(xblToken))
            })
            put("RelyingParty", "rp://api.minecraftservices.com/")
            put("TokenType", "JWT")
        }
        val xstsResp = postJson("https://xsts.auth.xboxlive.com/xsts/authorize", xstsBody)
        val xstsToken = xstsResp.getString("Token")
        val userHash = xstsResp.getJSONObject("DisplayClaims")
            .getJSONArray("xui").getJSONObject(0).getString("uhs")

        // Minecraft Services entitlement check is a further hop (log in with
        // XSTS -> minecraftservices.com/authentication/login_with_xbox ->
        // GET /entitlements/mcstore) that needs a live minecraftservices.com
        // round trip; left as "unknown, treat as not-yet-verified" here
        // rather than a real network call this file always makes on sign-in
        // — callers that need the hard entitlement check should call
        // checkMinecraftEntitlement() explicitly.
        persistSession(context, xstsToken, userHash)
        return XboxSession(xstsToken, userHash, minecraftEntitled = false)
    }

    @WorkerThread
    private fun postJson(url: String, body: JSONObject): JSONObject {
        val request = Request.Builder()
            .url(url)
            .addHeader("Accept", "application/json")
            .post(body.toString().toRequestBody(jsonMedia))
            .build()
        httpClient.newCall(request).execute().use { resp ->
            val text = resp.body?.string().orEmpty()
            if (!resp.isSuccessful) {
                throw IllegalStateException("Xbox Live auth failed (${resp.code}): $text")
            }
            return JSONObject(text)
        }
    }

    /**
     * Full entitlement check against Minecraft Services (equivalent purpose
     * to PlayStoreValidator.checkOwnership, but for the Microsoft-account
     * path): login_with_xbox -> mcToken -> GET /entitlements/mcstore.
     */
    @WorkerThread
    fun checkMinecraftEntitlement(context: Context): Boolean {
        val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val xstsToken = prefs.getString("xstsToken", null) ?: return false
        val userHash = prefs.getString("userHash", null) ?: return false

        val loginBody = JSONObject().put("identityToken", "XBL3.0 x=$userHash;$xstsToken")
        val loginReq = Request.Builder()
            .url("https://api.minecraftservices.com/authentication/login_with_xbox")
            .post(loginBody.toString().toRequestBody(jsonMedia))
            .build()
        val mcToken = httpClient.newCall(loginReq).execute().use { resp ->
            if (!resp.isSuccessful) return false
            JSONObject(resp.body?.string().orEmpty()).getString("access_token")
        }

        val entReq = Request.Builder()
            .url("https://api.minecraftservices.com/entitlements/mcstore")
            .addHeader("Authorization", "Bearer $mcToken")
            .get()
            .build()
        return httpClient.newCall(entReq).execute().use { resp ->
            if (!resp.isSuccessful) return false
            val items = JSONObject(resp.body?.string().orEmpty()).optJSONArray("items")
            (items?.length() ?: 0) > 0
        }
    }

    private fun persistSession(context: Context, xstsToken: String, userHash: String) {
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putString("xstsToken", xstsToken)
            .putString("userHash", userHash)
            .apply()
    }

    fun isSignedIn(context: Context): Boolean {
        val prefs = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        return prefs.contains("xstsToken") && prefs.contains("userHash")
    }

    fun signOut(context: Context) {
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit().clear().apply()
        pca?.let { app ->
            app.currentAccountAsync(object : ISingleAccountPublicClientApplication.CurrentAccountCallback {
                override fun onAccountLoaded(activeAccount: IAccount?) {
                    activeAccount?.let { app.signOut(object : ISingleAccountPublicClientApplication.SignOutCallback {
                        override fun onSignOut() {}
                        override fun onError(exception: MsalException) {}
                    }) }
                }
                override fun onAccountChanged(priorAccount: IAccount?, currentAccount: IAccount?) {}
                override fun onError(exception: MsalException) {}
            })
        }
    }
}
