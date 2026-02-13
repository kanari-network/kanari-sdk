#include "include/kanari_crypto/kanari_crypto_plugin.h"

#include <flutter/plugin_registrar_windows.h>
#include <memory>

class KanariCryptoPlugin : public flutter::Plugin {
 public:
  static void RegisterWithRegistrar(flutter::PluginRegistrarWindows *registrar);

  KanariCryptoPlugin();

  virtual ~KanariCryptoPlugin();

  KanariCryptoPlugin(const KanariCryptoPlugin&) = delete;
  KanariCryptoPlugin& operator=(const KanariCryptoPlugin&) = delete;
};

// static
void KanariCryptoPlugin::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows *registrar) {
  auto plugin = std::make_unique<KanariCryptoPlugin>();
  registrar->AddPlugin(std::move(plugin));
}

KanariCryptoPlugin::KanariCryptoPlugin() {}

KanariCryptoPlugin::~KanariCryptoPlugin() {}

void KanariCryptoPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  KanariCryptoPlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
