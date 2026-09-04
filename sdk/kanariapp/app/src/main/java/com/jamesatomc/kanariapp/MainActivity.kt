package com.jamesatomc.kanariapp

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.*
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.fragment.app.FragmentActivity
import com.jamesatomc.kanariapp.ui.screens.*
import com.jamesatomc.kanariapp.ui.theme.KanariAppTheme
import com.jamesatomc.kanariapp.compose.KeyGenerationScreen
import com.jamesatomc.kanariapp.wallet.WalletViewModel

class MainActivity : FragmentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        System.setProperty("jna.nosys", "true")

        enableEdgeToEdge()
        setContent {
            val viewModel: WalletViewModel = viewModel()
            val themeMode by viewModel.themeMode.collectAsStateWithLifecycle()
            KanariAppTheme(themeMode = themeMode) {
                MainNavigation(viewModel = viewModel)
            }
        }
    }
}

@Composable
fun MainNavigation(viewModel: WalletViewModel) {
    val navController = rememberNavController()
    val isUnlocked by viewModel.isUnlocked.collectAsStateWithLifecycle()
    val wallets by viewModel.wallets.collectAsStateWithLifecycle()

    // Start destination logic
    val startDestination = remember(isUnlocked, wallets) {
        if (isUnlocked) Screen.Main.route else Screen.Welcome.route
    }

    NavHost(navController = navController, startDestination = startDestination) {
        composable(Screen.Welcome.route) {
            WelcomeScreen(
                onNavigateToLogin = { navController.navigate(Screen.Login.route) },
                onNavigateToRegister = { navController.navigate(Screen.Register.route) },
                onNavigateToWalletGen = { navController.navigate(Screen.WalletGeneration.route) },
                onNavigateToUnlock = { navController.navigate(Screen.Unlock.route) }
            )
        }
        composable(Screen.Login.route) {
            LoginScreen(
                onLoginSuccess = {
                    navController.navigate(Screen.Main.route) {
                        popUpTo(Screen.Welcome.route) { inclusive = true }
                    }
                },
                onNavigateToRegister = { navController.navigate(Screen.Register.route) },
                onNavigateToWalletGen = { navController.navigate(Screen.WalletGeneration.route) }
            )
        }
        composable(Screen.Register.route) {
            RegisterScreen(
                onRegisterSuccess = { navController.navigate(Screen.Login.route) },
                onNavigateToLogin = { navController.popBackStack() }
            )
        }
        composable(Screen.Main.route) {
            MainScreen(
                navController = navController,
                viewModel = viewModel,
                onLogout = {
                    viewModel.logout()
                    navController.navigate(Screen.Welcome.route) {
                        popUpTo(Screen.Main.route) { inclusive = true }
                    }
                }
            )
        }
        composable(Screen.Dashboard.route) {
            DashboardScreen(
                viewModel = viewModel,
                onNavigateToReceive = { navController.navigate(Screen.Receive.route) },
                onNavigateToSettings = { navController.navigate(Screen.Settings.route) },
                onNavigateToSend = { /* Handled in MainScreen bottom nav */ },
                onNavigateToWalletGen = { navController.navigate(Screen.WalletGeneration.route) }
            )
        }
        composable(Screen.Receive.route) {
            ReceiveScreen(viewModel = viewModel, onBack = { navController.popBackStack() })
        }
        composable(Screen.Settings.route) {
            SettingsScreen(
                viewModel = viewModel,
                onLogout = {
                    viewModel.logout()
                    navController.navigate(Screen.Welcome.route) {
                        popUpTo(Screen.Dashboard.route) { inclusive = true }
                    }
                },
                onBack = { navController.popBackStack() }
            )
        }
        composable(Screen.WalletGeneration.route) {
            KeyGenerationScreen(onBack = { navController.popBackStack() })
        }
        composable(Screen.Unlock.route) {
            UnlockScreen(
                viewModel = viewModel,
                onUnlockSuccess = {
                    navController.navigate(Screen.Main.route) {
                        popUpTo(Screen.Welcome.route) { inclusive = true }
                    }
                },
                onBack = { navController.popBackStack() }
            )
        }
    }
}
