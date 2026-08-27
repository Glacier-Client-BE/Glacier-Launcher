package xyz.glacierclient.launcher

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import xyz.glacierclient.launcher.ui.GlacierLauncherApp
import xyz.glacierclient.launcher.ui.theme.GlacierTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            GlacierTheme {
                GlacierLauncherApp()
            }
        }
    }
}
