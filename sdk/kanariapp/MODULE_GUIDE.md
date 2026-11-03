# Standard Module Pattern (kanariapp)

Every feature module (Deepbook, Escrow, Template, ...) follows the same layout
so new modules can be added by copying the `Template` scaffold.

## File layout

```
app/src/main/java/com/jamesatomc/kanariapp/
  network/
    <Module>/
      <Module>Client.kt          # package: network  (build*/submit* extensions on KanariClient)
      <Module>WalletViewModel.kt # package: wallet  (WalletViewModel extensions, zkLogin-aware)
      <Module>Models.kt          # package: network.models  (data classes)
  ui/screens/
    <Module>Screen.kt            # Compose screen, takes WalletViewModel
```

Reference implementations: `network/Deepbook/*`, `network/Escrow/*`,
`network/Template/*`.

## Steps to add a new module

1. Copy `network/Template/` to `network/<Module>/` and rename files.
2. Replace `TEMPLATE_PACKAGE`, `TEMPLATE_MODULE`, function names, args in `<Module>Client.kt`.
3. Update `<Module>WalletViewModel.kt` action methods and view helpers.
4. Add data classes to `network/models/<Module>Models.kt`.
5. Implement `<Module>Screen.kt` Compose UI using `viewModel.<action>(...)`.
6. Register the screen in `ui/screens/Screens.kt` / navigation (see `DeepBookScreen` usage).
7. Add module constants to `network/models` if needed (package id, registry id).

## Conventions

- Build/submit extensions live in package `network` (so `KanariClient` extensions resolve).
- Wallet extensions live in package `wallet` (so `WalletViewModel` extensions resolve).
- Models live in package `network.models`.
- Screens import extensions explicitly:
  `import com.jamesatomc.kanariapp.wallet.<actionFunction>`
- zkLogin: branch on `record.curveType == "ZkLogin"` and use
  `ZkLoginAuth.loadSession` + `submitXxxTxZkLogin`, mirroring Deepbook.
- View calls use `client.callViewFunction(package, module, function, typeArgs, args)`
  and decode via `toViewULong()` / `toViewAddress()`.

## Build check

```
.\gradlew.bat :app:compileDebugKotlin
```
