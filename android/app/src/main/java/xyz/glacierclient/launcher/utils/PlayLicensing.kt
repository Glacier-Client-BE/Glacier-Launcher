@file:Suppress("unused")
package xyz.glacierclient.launcher.utils

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import com.android.vending.licensing.ILicenseResultListener
import com.android.vending.licensing.ILicensingService
import kotlinx.coroutines.suspendCancellableCoroutine
import xyz.glacierclient.launcher.BuildConfig
import java.security.KeyFactory
import java.security.PublicKey
import java.security.Signature
import java.security.spec.X509EncodedKeySpec
import java.util.concurrent.TimeUnit
import kotlin.coroutines.resume

/**
 * Google's OFFICIAL Play Licensing Service check (historically distributed
 * as source-you-copy-in under com.google.android.vending.licensing / "LVL"
 * — Google never shipped it as a Maven artifact, and this sandbox has no
 * network path to fetch that sample source, so ILicensingService.aidl /
 * ILicenseResultListener.aidl above are a minimal, hand-written equivalent
 * against the same public AIDL/Binder contract the Play Store app exposes,
 * not a copy of anyone's proprietary code).
 *
 * This answers a narrow, specific question: was THIS app — Glacier
 * Launcher itself — legitimately obtained through the Play Store (vs.
 * sideloaded/pirated)? It says nothing about Bedrock ownership; a
 * Google-account/Play-purchase check for that (GPlayApi/PurchaseHelper,
 * needing a stored account AAS/master token) was tried and reverted, see
 * docs/audit/TODO.md and MicrosoftAuth.kt's doc comment for the
 * credential-free alternative that replaced it.
 *
 * Requires com.android.vending (the Play Store app) to be installed and
 * signed in, and — for a real result — Glacier's own BuildConfig.
 * PLAY_LICENSING_PUBLIC_KEY to hold the RSA public key Play Console shows
 * for *this* app's listing (see build.gradle.kts's comment on where that
 * comes from). Until Glacier is published, that constant stays the
 * placeholder "PLAY_LICENSING_PUBLIC_KEY_PLACEHOLDER" and signature
 * verification below fails closed (LicenseResult.Error), which is the
 * correct behavior for an unconfigured/pre-launch build — never silently
 * treat an unverifiable response as licensed.
 */
object PlayLicensing {
    private const val VENDING_PACKAGE = "com.android.vending"
    private const val SERVICE_ACTION = "com.android.vending.licensing.ILicensingService"
    private const val RESPONSE_TIMEOUT_SECONDS = 10L

    sealed class LicenseResult {
        data object Licensed : LicenseResult()
        data object NotLicensed : LicenseResult()
        data class Error(val message: String) : LicenseResult()
    }

    /** Play's own numeric response codes, per the public LVL contract. */
    private const val LICENSED = 0x0
    private const val NOT_LICENSED = 0x1
    private const val LICENSED_OLD_KEY = 0x2
    private const val ERROR_NOT_MARKET_MANAGED = 0x3
    private const val ERROR_SERVER_FAILURE = 0x4
    private const val ERROR_OVER_QUOTA = 0x5
    private const val ERROR_CONTACTING_SERVER = 0x101
    private const val ERROR_INVALID_PACKAGE_NAME = 0x102
    private const val ERROR_NON_MATCHING_UID = 0x103

    suspend fun checkLicense(context: Context): LicenseResult {
        val publicKey = decodePublicKey()
            ?: return LicenseResult.Error(
                "PLAY_LICENSING_PUBLIC_KEY is unset (see build.gradle.kts) — cannot verify " +
                    "signed license responses until the project owner fills in Glacier's real " +
                    "Play Console licensing key."
            )

        return try {
            suspendCancellableCoroutine { cont ->
                var resumed = false
                fun finish(result: LicenseResult) {
                    if (!resumed) {
                        resumed = true
                        cont.resume(result)
                    }
                }

                // lateinit var, not val: onServiceConnected below needs to
                // unbind *this same connection* from inside its own
                // callback, but a val's initializer can't reference the val
                // being initialized (connection wouldn't exist yet at that
                // point in the object expression). Assigning after
                // construction closes that loop — by the time
                // onServiceConnected actually runs (async, post-bindService)
                // connection is long since assigned.
                lateinit var connection: ServiceConnection
                connection = object : ServiceConnection {
                    override fun onServiceConnected(name: ComponentName, binder: IBinder) {
                        val service = ILicensingService.Stub.asInterface(binder)
                        val nonce = java.security.SecureRandom().nextLong()
                        val listener = object : ILicenseResultListener.Stub() {
                            override fun verifyLicense(responseCode: Int, signedData: String?, signature: String?) {
                                finish(interpretResponse(responseCode, signedData, signature, publicKey, nonce, context.packageName))
                                try { context.applicationContext.unbindService(connection) } catch (_: Exception) {}
                            }
                        }
                        try {
                            service.checkLicense(nonce, context.packageName, listener)
                        } catch (e: Exception) {
                            finish(LicenseResult.Error("checkLicense() call failed: ${e.message}"))
                        }
                    }

                    override fun onServiceDisconnected(name: ComponentName) {
                        finish(LicenseResult.Error("Play Store licensing service disconnected."))
                    }
                }

                val intent = Intent(SERVICE_ACTION).apply { setPackage(VENDING_PACKAGE) }
                val bound = try {
                    context.applicationContext.bindService(intent, connection, Context.BIND_AUTO_CREATE)
                } catch (e: Exception) {
                    false
                }
                if (!bound) {
                    finish(LicenseResult.Error("Could not bind Play Store licensing service — is the Play Store app installed?"))
                    return@suspendCancellableCoroutine
                }

                cont.invokeOnCancellation {
                    try { context.applicationContext.unbindService(connection) } catch (_: Exception) {}
                }
            }
        } catch (e: Exception) {
            LicenseResult.Error(e.message ?: "Unknown licensing error.")
        }
    }

    private fun interpretResponse(
        responseCode: Int,
        signedData: String?,
        signature: String?,
        publicKey: PublicKey,
        expectedNonce: Long,
        expectedPackage: String,
    ): LicenseResult {
        return when (responseCode) {
            LICENSED, LICENSED_OLD_KEY -> {
                if (signedData == null || signature == null) {
                    return LicenseResult.Error("Licensed response missing signed payload.")
                }
                if (!verifySignature(signedData, signature, publicKey)) {
                    return LicenseResult.Error("Signature verification failed — response may be tampered.")
                }
                // Per the public LVL contract, signedData is a "|"-joined
                // string: responseCode|nonce|packageName|versionCode|userId|...
                val fields = signedData.split("|")
                val nonceOk = fields.getOrNull(1)?.toLongOrNull() == expectedNonce
                val packageOk = fields.getOrNull(2) == expectedPackage
                if (!nonceOk || !packageOk) {
                    LicenseResult.Error("Signed response nonce/package mismatch — possible replay attack.")
                } else {
                    LicenseResult.Licensed
                }
            }
            NOT_LICENSED -> LicenseResult.NotLicensed
            ERROR_NOT_MARKET_MANAGED -> LicenseResult.Error("App is not published/managed by Play — expected for a pre-launch build.")
            ERROR_SERVER_FAILURE -> LicenseResult.Error("Play licensing server failure.")
            ERROR_OVER_QUOTA -> LicenseResult.Error("Licensing check quota exceeded.")
            ERROR_CONTACTING_SERVER -> LicenseResult.Error("Could not contact Play licensing server.")
            ERROR_INVALID_PACKAGE_NAME -> LicenseResult.Error("Invalid package name for licensing check.")
            ERROR_NON_MATCHING_UID -> LicenseResult.Error("UID does not match the installed package's UID.")
            else -> LicenseResult.Error("Unrecognized licensing response code: $responseCode")
        }
    }

    private fun verifySignature(signedData: String, signatureBase64: String, publicKey: PublicKey): Boolean {
        return try {
            val signature = Signature.getInstance("SHA1withRSA").apply {
                initVerify(publicKey)
                update(signedData.toByteArray())
            }
            signature.verify(android.util.Base64.decode(signatureBase64, android.util.Base64.DEFAULT))
        } catch (e: Exception) {
            false
        }
    }

    private fun decodePublicKey(): PublicKey? {
        val encoded = BuildConfig.PLAY_LICENSING_PUBLIC_KEY
        if (encoded.isBlank() || encoded == "PLAY_LICENSING_PUBLIC_KEY_PLACEHOLDER") return null
        return try {
            val keyBytes = android.util.Base64.decode(encoded, android.util.Base64.DEFAULT)
            KeyFactory.getInstance("RSA").generatePublic(X509EncodedKeySpec(keyBytes))
        } catch (e: Exception) {
            null
        }
    }
}
