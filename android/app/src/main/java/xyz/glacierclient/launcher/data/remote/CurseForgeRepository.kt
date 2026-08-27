package xyz.glacierclient.launcher.data.remote

import io.ktor.client.call.body
import io.ktor.client.request.get
import io.ktor.client.request.header
import io.ktor.client.request.parameter
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * Mirrors Services/CurseForgeService.cs — same base URL and game/class ids,
 * so browsing behaves identically to the desktop app. Requires a CurseForge
 * API key (build-time secret, see android/README.md), same as desktop.
 */
class CurseForgeRepository(private val apiKey: String) {
    companion object {
        const val BASE_URL = "https://api.curseforge.com"
        const val GAME_ID_BEDROCK = 78022
        const val GAME_ID_JAVA = 432

        const val JAVA_CLASS_MODS = 6
        const val JAVA_CLASS_MODPACKS = 4471
        const val JAVA_CLASS_RESOURCE_PACKS = 12
        const val JAVA_CLASS_WORLDS = 17
        const val JAVA_CLASS_SHADER_PACKS = 6552

        const val BEDROCK_CLASS_ADDONS = 4984
        const val BEDROCK_CLASS_MAPS = 6913
        const val BEDROCK_CLASS_SKINS = 6925
        const val BEDROCK_CLASS_TEXTURE_PACKS = 6929
        const val BEDROCK_CLASS_SCRIPTS = 6940
    }

    /** Mirrors the desktop CurseForge.AvailableCategories chip row for the Bedrock game. */
    data class Category(val label: String, val classId: Int)

    val bedrockCategories = listOf(
        Category("Addons", BEDROCK_CLASS_ADDONS),
        Category("Maps", BEDROCK_CLASS_MAPS),
        Category("Skins", BEDROCK_CLASS_SKINS),
        Category("Texture Packs", BEDROCK_CLASS_TEXTURE_PACKS),
        Category("Scripts", BEDROCK_CLASS_SCRIPTS),
    )

    val isAvailable: Boolean get() = apiKey.isNotBlank()

    @Serializable
    data class Pagination(val index: Int = 0, val pageSize: Int = 0, val resultCount: Int = 0, val totalCount: Int = 0)

    @Serializable
    data class SearchResponse(val data: List<CurseForgeMod> = emptyList(), val pagination: Pagination = Pagination())

    @Serializable
    data class CurseForgeMod(
        val id: Int = 0,
        val name: String = "",
        val summary: String = "",
        val downloadCount: Long = 0,
        @SerialName("logo") val logo: CurseForgeLogo? = null,
        @SerialName("latestFiles") val latestFiles: List<CurseForgeFile> = emptyList(),
    )

    @Serializable
    data class CurseForgeLogo(val thumbnailUrl: String = "")

    @Serializable
    data class CurseForgeFile(
        val id: Int = 0,
        val fileName: String = "",
        val downloadUrl: String? = null,
    )

    suspend fun search(gameId: Int, classId: Int, query: String, index: Int = 0, pageSize: Int = 20): SearchResponse =
        HttpClientFactory.shared.get("$BASE_URL/v1/mods/search") {
            header("x-api-key", apiKey)
            parameter("gameId", gameId)
            parameter("classId", classId)
            parameter("searchFilter", query)
            parameter("index", index)
            parameter("pageSize", pageSize)
            parameter("sortField", 2) // popularity, matches desktop default sort
            parameter("sortOrder", "desc")
        }.body()
}
