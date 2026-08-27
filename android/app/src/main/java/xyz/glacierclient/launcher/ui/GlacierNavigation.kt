package xyz.glacierclient.launcher.ui

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Cable
import androidx.compose.material.icons.filled.Extension
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Inventory2
import androidx.compose.material.icons.filled.Layers
import androidx.compose.material.icons.filled.Photo
import androidx.compose.material.icons.filled.Public
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Widgets
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
import xyz.glacierclient.launcher.ui.screens.CreditsScreen
import xyz.glacierclient.launcher.ui.screens.CurseForgeScreen
import xyz.glacierclient.launcher.ui.screens.HomeScreen
import xyz.glacierclient.launcher.ui.screens.InstancesScreen
import xyz.glacierclient.launcher.ui.screens.PacksScreen
import xyz.glacierclient.launcher.ui.screens.ScreenshotsScreen
import xyz.glacierclient.launcher.ui.screens.ServersScreen
import xyz.glacierclient.launcher.ui.screens.SettingsScreen
import xyz.glacierclient.launcher.ui.screens.WorldsScreen

/**
 * Bottom tab set matches Pages/Home.razor's Bedrock "panel-tabs" row exactly,
 * same 11 destinations in the same order — Settings, Clients, Addons,
 * Servers, MC Versions, Worlds, Packs, Backups, Instances, Photos, Credits —
 * plus Home as the start destination (reached in the desktop app via each
 * panel's chevron "back" button rather than a tab). "MC Versions" (mcversions
 * = Bedrock version manager, distinct from the "versions" client-release
 * panel) still needs its own screen; it currently routes to Clients as a
 * placeholder — see android/README.md's status list.
 */
private data class NavItem(val route: String, val label: String, val icon: androidx.compose.ui.graphics.vector.ImageVector)

private val navItems = listOf(
    NavItem("settings", "Settings", Icons.Filled.Settings),
    NavItem("clients", "Clients", Icons.Filled.Extension),
    NavItem("addons", "Addons", Icons.Filled.Widgets),
    NavItem("servers", "Servers", Icons.Filled.Cable),
    NavItem("mcversions", "MC Versions", Icons.Filled.Inventory2),
    NavItem("worlds", "Worlds", Icons.Filled.Public),
    NavItem("packs", "Packs", Icons.Filled.Inventory2),
    NavItem("backups", "Backups", Icons.Filled.History),
    NavItem("instances", "Instances", Icons.Filled.Layers),
    NavItem("screenshots", "Photos", Icons.Filled.Photo),
    NavItem("credits", "Credits", Icons.Filled.Favorite),
)

@Composable
fun GlacierLauncherApp() {
    val navController = rememberNavController()

    Scaffold(
        bottomBar = {
            NavigationBar {
                val backStackEntry by navController.currentBackStackEntryAsState()
                val currentRoute = backStackEntry?.destination
                // Home isn't one of these tabs (desktop reaches it via the panel's
                // chevron-down back button); keep the icon here as that back action.
                NavigationBarItem(
                    selected = currentRoute?.hierarchy?.any { it.route == "home" } == true,
                    onClick = {
                        navController.navigate("home") {
                            popUpTo(navController.graph.findStartDestination().id) { saveState = true }
                            launchSingleTop = true
                            restoreState = true
                        }
                    },
                    icon = { Icon(Icons.Filled.Home, contentDescription = "Home") },
                    label = { Text("Home") },
                )
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
            composable("settings") { SettingsScreen() }
            composable("clients") { ClientsScreen() }
            composable("addons") { CurseForgeScreen() }
            composable("servers") { ServersScreen() }
            composable("mcversions") { ClientsScreen() } // TODO: dedicated Bedrock version manager, see README
            composable("worlds") { WorldsScreen() }
            composable("packs") { PacksScreen() }
            composable("backups") { BackupsScreen() }
            composable("instances") { InstancesScreen() }
            composable("screenshots") { ScreenshotsScreen() }
            composable("credits") { CreditsScreen() }
        }
    }
}
