package xyz.glacierclient.launcher.service

import android.content.Context
import android.os.Build
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Android counterpart of the desktop app's Services/DiscordRpcService.cs.
 *
 * The desktop service uses the DiscordRPC NuGet package, which speaks the
 * real Rich Presence protocol: a local IPC pipe to the **Discord desktop
 * client**. Android has no Discord desktop client, so that protocol has no
 * endpoint to talk to and cannot be ported as-is. The only way for a phone
 * to set its own presence is to hold an authenticated WebSocket to the
 * Discord Gateway and send PRESENCE_UPDATE over it, which requires a user
 * account token.
 *
 * That is self-botting. Discord's Terms of Service prohibit automating a
 * user account, and accounts have been terminated for it. Consequently:
 *
 *  - this is **opt-in and off by default** (see [isEnabled]),
 *  - it never obtains a token on its own. The existing Discord OAuth login
 *    (MainActivity.signInDiscord) grants only the "identify" scope, and no
 *    OAuth scope or bot token can set a *user's* presence — so the user has
 *    to paste their own account token deliberately,
 *  - the UI that collects it states the ban risk plainly rather than
 *    burying it.
 *
 * The presence payloads themselves mirror DiscordRpcService.cs exactly —
 * same application id, same asset keys, same Details/State wording — so a
 * user running both sees one consistent presence.
 */
object DiscordRpcService {

    /** Same Discord application as the desktop app, so the uploaded Rich Presence art resolves. */
    private const val APPLICATION_ID = "1482726422094024779"

    private const val GATEWAY_URL = "wss://gateway.discord.gg/?v=10&encoding=json"

    private const val PREFS = "glacier_settings"
    private const val KEY_ENABLED = "discord_rpc_enabled"
    private const val KEY_TOKEN = "discord_rpc_token"

    // Gateway opcodes (https://discord.com/developers/docs/topics/gateway).
    private const val OP_DISPATCH = 0
    private const val OP_HEARTBEAT = 1
    private const val OP_IDENTIFY = 2
    private const val OP_PRESENCE_UPDATE = 3
    private const val OP_RECONNECT = 7
    private const val OP_INVALID_SESSION = 9
    private const val OP_HELLO = 10
    private const val OP_HEARTBEAT_ACK = 11

    private val http: OkHttpClient by lazy {
        OkHttpClient.Builder()
            // The Gateway is a long-lived push connection: it is legitimately
            // silent between heartbeats, so no read timeout. Liveness is
            // tracked by heartbeat ACKs instead (see [awaitingAck]).
            .readTimeout(0, TimeUnit.MILLISECONDS)
            .pingInterval(0, TimeUnit.MILLISECONDS)
            .build()
    }

    private val scheduler: ScheduledExecutorService =
        Executors.newSingleThreadScheduledExecutor { r -> Thread(r, "glacier-discord-rpc").apply { isDaemon = true } }

    private var socket: WebSocket? = null
    private var heartbeatTask: ScheduledFuture<*>? = null
    private var reconnectTask: ScheduledFuture<*>? = null

    private var token: String = ""
    private var lastSequence: Int? = null
    private val awaitingAck = AtomicBoolean(false)
    private var reconnectAttempts = 0

    /** The presence to (re)apply once the Gateway is ready; also re-sent after a reconnect. */
    private var pendingActivity: JSONObject? = null

    @Volatile
    private var connected = false

    @Volatile
    private var lastError: String = ""

    // ── Configuration ────────────────────────────────────────────────────

    fun isEnabled(context: Context): Boolean = prefs(context).getBoolean(KEY_ENABLED, false)

    fun savedToken(context: Context): String = prefs(context).getString(KEY_TOKEN, "") ?: ""

    /**
     * Turns presence on or off and persists the choice. A blank [userToken]
     * keeps whatever token was already stored, so the UI can re-enable
     * without making the user paste it again.
     */
    fun configure(context: Context, enabled: Boolean, userToken: String?) {
        val resolved = userToken?.takeIf { it.isNotBlank() }?.trim() ?: savedToken(context)
        prefs(context).edit()
            .putBoolean(KEY_ENABLED, enabled)
            .putString(KEY_TOKEN, resolved)
            .apply()

        if (enabled && resolved.isNotBlank()) start(resolved) else stop()
    }

    /** Mirrors DiscordRpcService.cs's Start(): connects only when enabled and a token exists. */
    fun startIfEnabled(context: Context) {
        val stored = savedToken(context)
        if (isEnabled(context) && stored.isNotBlank()) start(stored)
    }

    fun statusJson(): String = JSONObject()
        .put("connected", connected)
        .put("error", lastError)
        .toString()

    // ── Presence states (mirroring DiscordRpcService.cs) ─────────────────

    /** DiscordRpcService.cs SetIdlePresence(). */
    fun setIdlePresence() = applyActivity(
        details = "In the Launcher",
        state = "Selecting a version",
        largeImage = "glacier_logo",
        largeText = "Glacier Launcher",
    )

    /** DiscordRpcService.cs SetInGamePresence(). */
    fun setBedrockPresence(versionTag: String?, clientName: String?) {
        val tag = versionTag.orEmpty()
        val state = when (clientName) {
            "Flarial Client" -> "Using Flarial"
            "OderSo Client" -> if (tag.isEmpty()) "Using OderSo" else "Using OderSo · $tag"
            "Custom DLL" -> if (tag.isEmpty()) "Using a custom DLL" else "Using $tag"
            "Vanilla" -> "Playing Vanilla"
            else -> if (tag.isEmpty()) "Using Latite" else "Using Latite · $tag"
        }
        applyActivity(
            details = "Playing Minecraft Bedrock",
            state = state,
            largeImage = "minecraft_icon",
            largeText = "Minecraft Bedrock",
            smallImage = "glacier_logo",
            smallText = "Glacier Launcher",
        )
    }

    /** DiscordRpcService.cs SetJavaInGamePresence(). */
    fun setJavaPresence(versionId: String?, variant: String?) {
        val version = versionId.orEmpty()
        val state = when (variant) {
            "Lunar" -> if (version.isEmpty()) "Using Lunar Client" else "Lunar · $version"
            "Badlion" -> if (version.isEmpty()) "Using Badlion" else "Badlion · $version"
            else -> if (version.isEmpty()) "In the menu" else "Playing $version"
        }
        applyActivity(
            details = "Playing Minecraft Java",
            state = state,
            largeImage = "minecraft_java",
            largeText = "Minecraft Java $version".trimEnd(),
            smallImage = "glacier_logo",
            smallText = "Glacier Launcher",
        )
    }

    // ── Connection lifecycle ─────────────────────────────────────────────

    @Synchronized
    private fun start(userToken: String) {
        if (socket != null) return
        token = userToken
        lastError = ""
        if (pendingActivity == null) {
            // Match the desktop's Start(), which sets idle presence immediately.
            setIdlePresence()
        }
        openSocket()
    }

    @Synchronized
    fun stop() {
        reconnectTask?.cancel(false)
        reconnectTask = null
        stopHeartbeat()
        connected = false
        reconnectAttempts = 0
        lastSequence = null
        pendingActivity = null
        // Cleared so a late callback from the socket being closed here
        // cannot pass scheduleReconnect()'s token check and revive it.
        token = ""
        socket?.close(1000, "stopped")
        socket = null
    }

    private fun openSocket() {
        val request = Request.Builder().url(GATEWAY_URL).build()
        socket = http.newWebSocket(request, GatewayListener)
    }

    // Outer members are qualified with DiscordRpcService throughout: a nested
    // `object` gets no implicit reference to the enclosing object's members
    // the way an `inner class` does to its outer class.
    private object GatewayListener : WebSocketListener() {
        override fun onMessage(webSocket: WebSocket, text: String) {
            runCatching { DiscordRpcService.handlePayload(webSocket, JSONObject(text)) }
        }

        override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
            DiscordRpcService.lastError = t.message ?: "Gateway connection failed"
            DiscordRpcService.onDisconnected(webSocket)
        }

        override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
            // 4004 is Discord's "authentication failed" — a wrong or expired
            // token. Retrying cannot fix it and would just hammer the
            // Gateway, so surface it and stay down until reconfigured.
            if (code == 4004) {
                DiscordRpcService.lastError = "Discord rejected the token (it may be wrong or expired)."
                synchronized(DiscordRpcService) {
                    DiscordRpcService.stopHeartbeat()
                    DiscordRpcService.connected = false
                    DiscordRpcService.socket = null
                }
                return
            }
            DiscordRpcService.onDisconnected(webSocket)
        }
    }

    private fun handlePayload(webSocket: WebSocket, payload: JSONObject) {
        if (!payload.isNull("s")) lastSequence = payload.optInt("s")

        when (payload.optInt("op", -1)) {
            OP_HELLO -> {
                val interval = payload.optJSONObject("d")?.optLong("heartbeat_interval", 41_250L) ?: 41_250L
                startHeartbeat(webSocket, interval)
                webSocket.send(identifyPayload())
            }

            OP_HEARTBEAT -> {
                // The Gateway can ask for an immediate heartbeat out of band.
                webSocket.send(heartbeatPayload())
            }

            OP_HEARTBEAT_ACK -> {
                awaitingAck.set(false)
            }

            OP_DISPATCH -> {
                if (payload.optString("t") == "READY") {
                    connected = true
                    lastError = ""
                    reconnectAttempts = 0
                    // Presence cannot be sent before READY; send whatever
                    // state the launcher is in now.
                    pendingActivity?.let { webSocket.send(presencePayload(it)) }
                }
            }

            // The Gateway is asking us to reconnect, or has invalidated the
            // session. Either way the fix is the same: drop this socket and
            // establish a fresh one, which re-IDENTIFYs and re-sends presence.
            OP_RECONNECT, OP_INVALID_SESSION -> {
                webSocket.close(1000, "reconnecting")
                onDisconnected(webSocket)
            }
        }
    }

    private fun onDisconnected(webSocket: WebSocket) = synchronized(DiscordRpcService) {
        // Ignore callbacks from a socket we already replaced or shut down.
        if (socket !== webSocket) return
        stopHeartbeat()
        connected = false
        socket = null
        scheduleReconnect()
    }

    private fun scheduleReconnect() {
        if (token.isBlank()) return
        reconnectTask?.cancel(false)
        // Exponential backoff capped at 5 minutes, so a persistent outage
        // (or a revoked token) doesn't spin on the network or the battery.
        val delaySeconds = minOf(300L, (1L shl minOf(reconnectAttempts, 8)) * 5L)
        reconnectAttempts++
        reconnectTask = scheduler.schedule({
            synchronized(DiscordRpcService) { if (socket == null && token.isNotBlank()) openSocket() }
        }, delaySeconds, TimeUnit.SECONDS)
    }

    private fun startHeartbeat(webSocket: WebSocket, intervalMs: Long) {
        stopHeartbeat()
        awaitingAck.set(false)
        heartbeatTask = scheduler.scheduleAtFixedRate({
            // A heartbeat that was never ACKed means a zombie connection —
            // the socket looks open but the Gateway is gone. Discord's own
            // guidance is to tear it down and reconnect rather than keep
            // sending into it.
            if (awaitingAck.getAndSet(true)) {
                webSocket.close(4000, "heartbeat not acknowledged")
                onDisconnected(webSocket)
                return@scheduleAtFixedRate
            }
            runCatching { webSocket.send(heartbeatPayload()) }
        }, intervalMs, intervalMs, TimeUnit.MILLISECONDS)
    }

    private fun stopHeartbeat() {
        heartbeatTask?.cancel(false)
        heartbeatTask = null
        awaitingAck.set(false)
    }

    // ── Payload builders ─────────────────────────────────────────────────

    private fun heartbeatPayload(): String = JSONObject()
        .put("op", OP_HEARTBEAT)
        .put("d", lastSequence ?: JSONObject.NULL)
        .toString()

    private fun identifyPayload(): String = JSONObject()
        .put("op", OP_IDENTIFY)
        .put(
            "d",
            JSONObject()
                .put("token", token)
                // A user IDENTIFY carries client properties rather than the
                // `intents` a bot sends; the Gateway rejects the payload
                // without them.
                .put(
                    "properties",
                    JSONObject()
                        .put("os", "Android")
                        .put("browser", "Discord Android")
                        .put("device", Build.MODEL ?: "Android"),
                )
                .put("compress", false)
                .put("presence", pendingActivity?.let { presenceData(it) } ?: JSONObject.NULL),
        )
        .toString()

    private fun presencePayload(activity: JSONObject): String = JSONObject()
        .put("op", OP_PRESENCE_UPDATE)
        .put("d", presenceData(activity))
        .toString()

    private fun presenceData(activity: JSONObject): JSONObject = JSONObject()
        .put("since", 0)
        .put("activities", JSONArray().put(activity))
        .put("status", "online")
        .put("afk", false)

    /**
     * Builds the activity object and pushes it if the Gateway is ready,
     * otherwise stores it for [OP_DISPATCH]'s READY handler to send. Matches
     * the desktop's RichPresence: type 0 ("Playing"), Timestamps.Now, and
     * the application id that owns the asset keys.
     */
    private fun applyActivity(
        details: String,
        state: String,
        largeImage: String,
        largeText: String,
        smallImage: String? = null,
        smallText: String? = null,
    ) {
        val assets = JSONObject()
            .put("large_image", largeImage)
            .put("large_text", largeText)
        if (smallImage != null) assets.put("small_image", smallImage)
        if (smallText != null) assets.put("small_text", smallText)

        val activity = JSONObject()
            .put("name", "Glacier Launcher")
            .put("type", 0)
            .put("application_id", APPLICATION_ID)
            .put("details", details)
            .put("state", state)
            .put("timestamps", JSONObject().put("start", System.currentTimeMillis()))
            .put("assets", assets)

        synchronized(this) {
            pendingActivity = activity
            val ws = socket
            if (connected && ws != null) runCatching { ws.send(presencePayload(activity)) }
        }
    }

    private fun prefs(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
