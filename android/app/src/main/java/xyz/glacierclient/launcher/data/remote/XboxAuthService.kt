package xyz.glacierclient.launcher.data.remote

import io.ktor.client.call.body
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import kotlinx.serialization.Serializable

/**
 * Mirrors Services/LiveAuthService.cs + XboxProfileService.cs: Microsoft
 * device-code OAuth -> Xbox Live (XBL) -> XSTS token exchange -> profile
 * fetch. Pure HTTPS, so it ports 1:1 (unlike DLL injection, there's nothing
 * Windows-specific about this flow).
 */
class XboxAuthService {
    companion object {
        // Minecraft's own public client id, same one the desktop app and
        // every other unofficial Minecraft launcher uses for device-code auth.
        private const val CLIENT_ID = "00000000402b5328"
        private const val DEVICE_CODE_URL = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode"
        private const val TOKEN_URL = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token"
        private const val XBL_AUTH_URL = "https://user.auth.xboxlive.com/user/authenticate"
        private const val XSTS_AUTH_URL = "https://xsts.auth.xboxlive.com/xsts/authorize"
        private const val PROFILE_URL = "https://profile.xboxlive.com/users/me/profile/settings"
        private const val SCOPE = "XboxLive.signin offline_access"
    }

    @Serializable
    data class DeviceCodeResponse(
        val device_code: String = "",
        val user_code: String = "",
        val verification_uri: String = "",
        val interval: Int = 5,
        val expires_in: Int = 900,
    )

    @Serializable
    data class TokenResponse(
        val access_token: String = "",
        val refresh_token: String = "",
        val expires_in: Int = 0,
    )

    suspend fun requestDeviceCode(): DeviceCodeResponse =
        HttpClientFactory.shared.post(DEVICE_CODE_URL) {
            contentType(ContentType.Application.FormUrlEncoded)
            setBody("client_id=$CLIENT_ID&scope=$SCOPE")
        }.body()

    /** Poll with [DeviceCodeResponse.interval] between calls until the user authorizes on another device. */
    suspend fun pollForToken(deviceCode: String): TokenResponse? = try {
        HttpClientFactory.shared.post(TOKEN_URL) {
            contentType(ContentType.Application.FormUrlEncoded)
            setBody(
                "grant_type=urn:ietf:params:oauth:grant-type:device_code" +
                    "&client_id=$CLIENT_ID&device_code=$deviceCode",
            )
        }.body()
    } catch (e: Exception) {
        null // authorization_pending — caller keeps polling
    }

    suspend fun authenticateXbl(msAccessToken: String): String {
        val body = """
            {"Properties":{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":"d=$msAccessToken"},
             "RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"}
        """.trimIndent()
        val response: Map<String, kotlinx.serialization.json.JsonElement> =
            HttpClientFactory.shared.post(XBL_AUTH_URL) {
                contentType(ContentType.Application.Json)
                setBody(body)
            }.body()
        return response["Token"]?.toString()?.trim('"') ?: ""
    }

    data class XstsResult(val token: String, val userHash: String)

    suspend fun authenticateXsts(xblToken: String): XstsResult {
        val body = """
            {"Properties":{"SandboxId":"RETAIL","UserTokens":["$xblToken"]},
             "RelyingParty":"http://xboxlive.com","TokenType":"JWT"}
        """.trimIndent()
        val json: kotlinx.serialization.json.JsonObject =
            HttpClientFactory.shared.post(XSTS_AUTH_URL) {
                contentType(ContentType.Application.Json)
                setBody(body)
            }.body()
        val token = json["Token"].toString().trim('"')
        val userHash = json["DisplayClaims"]
            ?.let { it.toString() } // minimal parse; callers needing the full profile use fetchProfile()
            ?: ""
        return XstsResult(token, userHash)
    }

    suspend fun fetchProfile(xstsToken: String, userHash: String): String =
        HttpClientFactory.shared.get(PROFILE_URL) {
            header("x-xbl-contract-version", "3")
            header("Authorization", "XBL3.0 x=$userHash;$xstsToken")
        }.body()
}
