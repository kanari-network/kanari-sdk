package com.jamesatomc.kanariapp.ui.screens

sealed class Screen(val route: String) {
    object Welcome : Screen("welcome")
    object Login : Screen("login")
    object Register : Screen("register")
    object Main : Screen("main")
    object Dashboard : Screen("dashboard")
    object Send : Screen("send")
    object Receive : Screen("receive")
    object Settings : Screen("settings")
    object WalletGeneration : Screen("wallet_generation")
    object Unlock : Screen("unlock")
}