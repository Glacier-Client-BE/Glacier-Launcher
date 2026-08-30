@file:Suppress("unused")
package xyz.glacierclient.launcher.utils

import android.content.Context
import com.aurora.gplayapi.helpers.AppDetailsHelper
import com.aurora.gplayapi.helpers.PurchaseHelper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Checks whether the Google account whose AAS token is stored in this app's
 * "accountData" SharedPreferences (see GPlayAPI.getAuthData) has purchased/
 * owns com.mojang.minecraftpe on the Play Store, using the same GPlayApi
 * client GPlayAPI.getApks already wraps.
 *
 * Reinstated at explicit project-owner direction after an earlier revert —
 * see docs/audit/TODO.md for the risk this accepts (a Google account AAS/
 * master token stored client-side) and the mitigations in place.
 */
object PlayStoreValidator {
    private const val BEDROCK_PACKAGE = "com.mojang.minecraftpe"

    sealed class Result {
        data object Owned : Result()
        data object NotOwned : Result()
        data class Error(val message: String) : Result()
    }

    /**
     * AppDetailsHelper's app-details response carries the account's own
     * purchase/offer state for the package (e.g. "Install" vs "Buy"), which
     * is the direct ownership signal; PurchaseHelper.purchase() is used as
     * a fallback confirmation because a successful delivery response (a
     * non-empty file list, no exception) also implies ownership, and a
     * denial throws.
     */
    suspend fun checkOwnership(context: Context): Result {
        return withContext(Dispatchers.IO) {
            try {
                val authData = GPlayAPI.getAuthDataOrNull(context)
                    ?: return@withContext Result.Error("No Google account signed in.")

                val app = try {
                    AppDetailsHelper(authData).getAppByPackageName(BEDROCK_PACKAGE)
                } catch (e: Exception) {
                    null
                }

                if (app != null && !app.isFree) {
                    return@withContext Result.Owned
                }
                if (app != null && app.isFree) {
                    // A "free" listing on this account's storefront still
                    // needs an actual purchase/delivery record for a paid
                    // app like Bedrock that's regionally priced or bundled
                    // — fall through to the PurchaseHelper check instead of
                    // trusting isFree alone.
                }

                val files = PurchaseHelper(authData).purchase(BEDROCK_PACKAGE, 0, 0)
                if (files.isNotEmpty()) Result.Owned else Result.NotOwned
            } catch (e: Exception) {
                Result.Error(e.message ?: e.javaClass.simpleName)
            }
        }
    }
}
