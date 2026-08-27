package xyz.glacierclient.launcher.ui

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CloudDownload
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Photo
import androidx.compose.material.icons.filled.Public
import androidx.compose.material.icons.filled.Save
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.SportsEsports
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import xyz.glacierclient.launcher.ui.screens.BackupsScreen
import xyz.glacierclient.launcher.ui.screens.ClientsScreen
import xyz.glacierclient.launcher.ui.screens.CurseForgeScreen
import xyz.glacierclient.launcher.ui.screens.HomeScreen
import xyz.glacierclient.launcher.ui.screens.JavaEditionScreen
import xyz.glacierclient.launcher.ui.screens.PacksScreen
import xyz.glacierclient.launcher.ui.screens.ScreenshotsScreen
import xyz.glacierclient.launcher.ui.screens.SettingsScreen
import xyz.glacierclient.launcher.ui.screens.WorldsScreen

// Mirrors the desktop app's Pages/Home.Panels.cs tab set, plus a Java Edition tab
// (bridges to the rebranded Pojav companion app) and CurseForge/Screenshots browsers.
private data class NavItem(val route: String, val label: String, val icon: androidx.compose.ui.graphics.vector.ImageVector)

private val navItems = listOf(
    NavItem("home", "Home", Icons.Filled.Home),
    NavItem("clients", "Clients", Icons.Filled.Extension),
    NavItem("java", "Java", Icons.Filled.SportsEsports),
    NavItem("curseforge", "CurseForge", Icons.Filled.CloudDownload),
    NavItem("worlds", "Worlds", Icons.Filled.Public),
    NavItem("packs", "Packs", Icons.Filled.Save),
    NavItem("screenshots", "Shots", Icons.Filled.Photo),
    NavItem("backups", "Backups", Icons.Filled.Save),
    NavItem("settings", "Settings", Icons.Filled.Settings),
)

@Composable
fun GlacierLauncherApp() {
    val navController = rememberNavController()

    Scaffold(
        bottomBar = {
            NavigationBar {
                val backStackEntry by navController.currentBackStackEntryAsState()
                val currentRoute = backStackEntry?.destination
                navItems.forEach { item ->
                    NavigationBarItem(
                        selected = currentRoute?.hierarchy?.any { it.route == item.route } == true,
                        onClick = {
                            navController.navigate(item.route) {
                                popUpTo(navController.graph.findStartDestination().id) { saveState = true }
                                launchSingleTop = true
                                restoreState = true
                            }
                        },
                        icon = { Icon(item.icon, contentDescription = item.label) },
                        label = { Text(item.label) },
                    )
                }
            }
        },
    ) { padding ->
        NavHost(
            navController = navController,
            startDestination = "home",
            modifier = androidx.compose.ui.Modifier.padding(padding),
        ) {
            composable("home") { HomeScreen() }
            composable("clients") { ClientsScreen() }
            composable("java") { JavaEditionScreen() }
            composable("curseforge") { CurseForgeScreen() }
            composable("worlds") { WorldsScreen() }
            composable("packs") { PacksScreen() }
            composable("screenshots") { ScreenshotsScreen() }
            composable("backups") { BackupsScreen() }
            composable("settings") { SettingsScreen() }
        }
    }
}
