//! 組句期間輪詢：Server 回新版本的換序幀就重畫一次並回報，同版本不重畫也不重複回報。
#include "qingjian.h"
#include "support/context.h"
#include "support/mock.h"
#include <fcitx/inputcontextmanager.h>
#include <fcitx/inputpanel.h>
int main() {
    qingjian::test::Mock mock("rescore");
    char program[] = "qingjian-test"; char disable[] = "--disable=all";
    char *arguments[] = {program, disable, nullptr};
    fcitx::Instance instance(2, arguments);
    instance.initialize();
    fcitx::QingjianEngine engine(&instance.addonManager());
    auto owned = std::make_unique<Context>(instance.inputContextManager());
    auto &context = *owned;
    context.setCapabilityFlags(fcitx::CapabilityFlag::Preedit);
    context.focusIn();
    assert(engine.process(&context, fcitx::Key(FcitxKey_n)));
    auto candidates = context.inputPanel().candidateList();
    assert(candidates && candidates->candidate(2).text().toString() == "你好");
    // 事件迴圈只能 exec 一次：300 ms 時換序幀早到了（外掛約每 80 ms Poll 一次，第一條就回換序幀），
    // 斷言後銷燬上下文，再跑一拍讓排隊的 CloseSession 發出去。
    int fired = 0;
    // 精度給 1 ms：預設的 250 ms 會讓這一拍晚到，300 ms 內的 Poll 次數就數不準。
    auto timer = instance.eventLoop().addTimeEvent(CLOCK_MONOTONIC, fcitx::now(CLOCK_MONOTONIC) + 300000, 1000,
        [&](fcitx::EventSourceTime *source, uint64_t) {
            if (++fired == 1) {
                auto list = owned->inputPanel().candidateList();
                assert(list && list->candidate(0).text().toString() == "你好");
                owned.reset();
                source->setNextInterval(30000);
                source->setOneShot();
                return true;
            }
            instance.eventLoop().exit();
            return false;
        });
    instance.eventLoop().exec();
    assert(fired == 2);
    mock.join();
    size_t polls = 0, acknowledged = 0;
    for (const auto &message : mock.messages) {
        if (message.contains("Poll")) ++polls;
        if (message.contains("DisplayAcknowledged")) ++acknowledged;
    }
    assert(polls >= 2);
    // 按鍵那幀回報一次、換序那幀回報一次；同版本的 Poll 不重畫也不重複回報。
    assert(acknowledged == 2);
}
