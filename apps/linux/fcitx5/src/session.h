//! Fcitx 上下文只儲存會話身份與展示事實，輸入狀態儲存在 Server。
#pragma once
#include "ipc/shared.h"
#include <fcitx/inputcontextproperty.h>
#include <fcitx-utils/event.h>
namespace qingjian {
struct Session final : fcitx::InputContextProperty {
    Session(std::shared_ptr<SharedConnection> shared, uint64_t number)
        : id(number), owner(shared) {}
    ~Session() override {
        if (auto shared = owner.lock()) {
            if (opened) shared->retire(id);
            else shared->contexts.erase(id);
        }
    }

    /// 外掛生命週期內遞增，永不復用。
    uint64_t id;

    /// 析構只撤銷登記並排隊關閉，不阻塞或訪問已析構 Engine。
    std::weak_ptr<SharedConnection> owner;

    bool opened = false;

    bool focused = false;

    /// 最近提交給客戶端的預編輯；框架失焦提交事實依賴此欄位。
    bool clientPreedit = false;

    uint64_t revision = 0;

    uint64_t generation = 0;

    /// 框架邊界代次；與純顯示失敗分開，使重入後的舊提交可被撤銷。
    uint64_t lifecycle = 0;

    nlohmann::json displayIdentity;

    /// 僅快取 Server 已確認的能力；尚未成功傳送不能視為同步。
    nlohmann::json capabilities;

    std::string preeditMode = "both";
};
}
