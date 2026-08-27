package xyz.glacierclient.launcher.data.remote

import io.ktor.client.HttpClient
import io.ktor.client.engine.android.Android
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.json.Json

/** Mirrors GlacierLauncher.Services.HttpFactory — a single shared client, one User-Agent. */
object HttpClientFactory {
    val shared: HttpClient by lazy {
        HttpClient(Android) {
            install(ContentNegotiation) {
                json(Json { ignoreUnknownKeys = true })
            }
            engine {
                connectTimeout = 15_000
                socketTimeout = 30_000
            }
        }
    }
}
