#ifndef FLUTTER_PLUGIN_KANARI_CRYPTO_PLUGIN_H_
#define FLUTTER_PLUGIN_KANARI_CRYPTO_PLUGIN_H_

#include <flutter/plugin_registrar_windows.h>

#ifdef FLUTTER_PLUGIN_SYMBOL
#define FLUTTER_PLUGIN_EXPORT __declspec(dllexport)
#else
#define FLUTTER_PLUGIN_EXPORT
#endif

#if defined(__cplusplus)
extern "C" {
#endif

FLUTTER_PLUGIN_EXPORT void KanariCryptoPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar);

#if defined(__cplusplus)
}
#endif

#endif  // FLUTTER_PLUGIN_KANARI_CRYPTO_PLUGIN_H_
