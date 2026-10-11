//! Fcitx KeySym 與共享協議鍵碼之間的對映。
#pragma once
#include <fcitx-utils/key.h>
#include <nlohmann/json.hpp>
namespace qingjian {
nlohmann::json mapKey(const fcitx::Key &key);
}
