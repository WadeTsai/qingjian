//! 真實外掛與 Server 的 watcher、UI、提交重入；檢查消費結果及應用最終文本。
#include "qingjian.h"
#include "support/server.h"
#include "support/input.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputmethodentry.h>
#include <fcitx/userinterfacemanager.h>
#include <cassert>

namespace {
bool keyboardVisible = false;
void type(fcitx::QingjianEngine &engine, Input &context, const std::string &text) {
    for (auto c : text) assert(engine.process(&context, fcitx::Key(static_cast<fcitx::KeySym>(c))));
}
qingjian::Session *session(Input &context) {
    return static_cast<qingjian::Session *>(context.property("qingjian-session"));
}
}
// 和既有虛擬鍵盤測試相同，僅控制桌面可見性；IPC 與 Rust Server 均為真實實現。
bool fcitx::UserInterfaceManager::isVirtualKeyboardVisible() const { return keyboardVisible; }

int main(int argc, char **argv) {
    assert(argc == 2);
    const std::string scenario = argv[1];
    Server server;
    char program[] = "qingjian-reentrant", disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments); instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    fcitx::InputMethodEntry entry("qingjian", "qingjian", "zh_CN", "qingjian");
    auto a = std::make_unique<Input>(instance.inputContextManager());
    auto b = std::make_unique<Input>(instance.inputContextManager());
    auto c = std::make_unique<Input>(instance.inputContextManager());
    a->focusIn();
    if (scenario.rfind("vkeyboard-", 0) == 0) {
        type(engine, *a, "n"); type(engine, *b, "h");
        const auto generation = session(*b)->generation;
        if (scenario == "vkeyboard-focusout") {
            a->focusOut(); // 框架負責 clientPreedit 的唯一一次提交。
            fcitx::FocusOutEvent event(a.get()); engine.deactivate(entry, event);
        } else {
            fcitx::InputContextEvent event(a.get(), fcitx::EventType::InputContextInputMethodDeactivated);
            engine.deactivate(entry, event);
        }
        assert(a->committed == "n" && a->preedit().empty());
        assert(session(*a)->opened && session(*a)->displayIdentity.is_null());
        keyboardVisible = true;
        instance.postEvent(fcitx::VirtualKeyboardVisibilityChangedEvent());
        assert(engine.process(b.get(), fcitx::Key(FcitxKey_a)));
        assert(b->preedit() == "ha" && b->committed.empty());
        type(engine, *b, "o ");
        assert(b->committed == "好" && a->committed == "n");
        assert(session(*b)->generation == generation && server.sockets() == 2);
        return 0;
    }
    if (scenario == "capability-destroy") {
        type(engine, *a, "ni"); type(engine, *b, "hao");
        bool reached = false;
        a->preeditHook = [&] {
            reached = true;
            assert(session(*a)->capabilities.value("sensitive", false) && a->preedit().empty() && a->committed.empty());
            a.reset(); // capability watcher → syncPrivacy → clear 的當前上下文銷燬。
        };
        a->setCapabilityFlags(fcitx::CapabilityFlags(fcitx::CapabilityFlag::Preedit) | fcitx::CapabilityFlag::Sensitive);
        assert(reached && !a);
        assert(engine.process(b.get(), fcitx::Key(FcitxKey_space)));
        assert(b->committed == "好" && server.sockets() == 2);
        return 0;
    }
    if (scenario == "disconnect-destroy") {
        type(engine, *a, "ni"); type(engine, *b, "hao"); type(engine, *c, "ni");
        auto shared = session(*c)->owner.lock();
        // 按實際安全引用快照順序選擇 A/B，確保銷燬的是尚待清窗的下一物件。
        std::vector<Input *> order;
        for (const auto &[id, reference] : shared->contexts) {
            (void)id;
            auto *context = reference.get();
            if (context == a.get() || context == b.get()) order.push_back(static_cast<Input *>(context));
        }
        assert(order.size() == 2);
        auto &first = order[0] == a.get() ? a : b;
        auto &second = order[1] == a.get() ? a : b;
        bool reached = false;
        first->preeditHook = [&] {
            reached = true;
            assert(shared->clearing && !shared->connection.connected());
            assert(!session(*first)->opened && !session(*second)->opened && !session(*c)->opened);
            assert(first->committed.empty() && second->committed.empty());
            second.reset();
            assert(!engine.process(c.get(), fcitx::Key(FcitxKey_x)));
            c->commitString("x"); // 清理重入的鍵只透傳一次。
            assert(!shared->connection.connected());
        };
        const auto generation = shared->generation;
        server.stop();
        assert(!engine.process(c.get(), fcitx::Key(FcitxKey_space)));
        c->commitString(" ");
        assert(reached && !second && c->committed == "x " && c->preedit().empty());
        assert(!shared->clearing && shared->generation == generation);
        server.start(); type(engine, *c, "ni ");
        assert(c->committed == "x 你" && shared->generation > generation && server.sockets() == 2);
        return 0;
    }
    if (scenario == "commit-reentry") {
        type(engine, *a, "ni");
        bool reached = false;
        a->commitHook = [&] {
            reached = true;
            assert(a->committed == "你");
            fcitx::InputContextEvent reset(a.get(), fcitx::EventType::InputContextReset); engine.reset(entry, reset);
            a->setCapabilityFlags(fcitx::CapabilityFlags(fcitx::CapabilityFlag::Preedit) | fcitx::CapabilityFlag::Sensitive);
            type(engine, *b, "hao ");
        };
        assert(engine.process(a.get(), fcitx::Key(FcitxKey_space)));
        assert(reached && a->committed == "你" && b->committed == "好");
        type(engine, *a, "ni ");
        assert(a->committed == "你你" && session(*a)->capabilities.value("sensitive", false) && !session(*b)->capabilities.value("sensitive", false));
        return 0;
    }
    assert(scenario == "ui-reentry" || scenario == "ui-destroy");
    type(engine, *a, "ni");
    bool reached = false;
    auto watcher = instance.watchEvent(fcitx::EventType::InputContextUpdateUI, fcitx::EventWatcherPhase::PreInputMethod, [&](fcitx::Event &event) {
        if (reached || static_cast<fcitx::InputContextEvent &>(event).inputContext() != a.get()) return;
        reached = true;
        assert(a->committed.empty());
        if (scenario == "ui-destroy") a.reset();
        else {
            fcitx::InputContextEvent reset(a.get(), fcitx::EventType::InputContextReset); engine.reset(entry, reset);
            a->setCapabilityFlags(fcitx::CapabilityFlags(fcitx::CapabilityFlag::Preedit) | fcitx::CapabilityFlag::Sensitive);
        }
        type(engine, *b, "hao ");
    });
    auto *original = a.get();
    assert(engine.process(original, fcitx::Key(FcitxKey_space)));
    assert(reached && b->committed == "好");
    if (a) {
        assert(a->committed.empty()); // UI 回撥撤銷了舊生命週期，但原鍵不能透傳。
        type(engine, *a, "ni "); assert(a->committed == "你");
    }
    type(engine, *b, "ni "); assert(b->committed == "好你" && server.sockets() == 2);
}
