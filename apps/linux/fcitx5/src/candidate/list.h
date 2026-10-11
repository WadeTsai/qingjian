//! 當前頁使用 CommonCandidateList；UI 翻頁按鈕回 Server 獲取下一頁。
#pragma once
#include <fcitx/candidatelist.h>
#include <functional>
namespace qingjian {
class List final : public fcitx::CommonCandidateList {
public:
    List(int page, int pages, std::function<void(bool)> turn) : page_(page), pages_(pages), turn_(std::move(turn)) {}
    bool hasPrev() const override { return page_ > 0; }
    bool hasNext() const override { return page_ + 1 < pages_; }
    int totalPages() const override { return pages_; }
    int currentPage() const override { return page_; }
    void prev() override { if (hasPrev()) turn_(false); }
    void next() override { if (hasNext()) turn_(true); }
private:
    /// Server 當前頁。
    int page_;
    /// Server 總頁數。
    int pages_;
    /// 翻頁請求。
    std::function<void(bool)> turn_;
};
}
