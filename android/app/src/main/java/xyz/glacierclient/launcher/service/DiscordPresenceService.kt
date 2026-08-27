package xyz.glacierclient.launcher.service

import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import xyz.glacierclient.launcher.data.remote.HttpClientFactory

/**
 * Android has no equivalent of the desktop DiscordRPC IPC pipe — that only
 * exists between a local Discord *desktop* client and a local game process.
 * There is no supported way for a third-party Android app to set a Discord
 * user's Rich Presence at all (Discord's official Game SDK is
 * Windows/macOS/Linux only, and mobile Discord doesn't expose an IPC socket).
 *
 * This is a deliberately smaller, honest substitute: if the user supplies
 * their own Discord webhook URL (Settings > Discord), we can post a
 * "now playing" message to a channel of their choosing. It is NOT the same
 * feature as native Rich Presence, and is opt-in/off by default.
 */
class DiscordPresenceService {
    suspend fun postNowPlaying(webhookUrl: String, message: String) {
        if (webhookUrl.isBlank()) return
        HttpClientFactory.shared.post(webhookUrl) {
            contentType(ContentType.Application.Json)
            setBody("""{"content":${'"'}${message.replace("\"", "\\\"")}${'"'}}""")
        }
    }
}
