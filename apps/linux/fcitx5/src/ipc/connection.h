//! 有界 Unix socket 傳輸；任何錯誤都關閉連線，下一次按鍵重新握手。
#pragma once
#include <nlohmann/json.hpp>
namespace qingjian {
class Connection {
public:
    Connection() = default;
    Connection(const Connection &) = delete;
    Connection &operator=(const Connection &) = delete;
    ~Connection();
    bool open();
    bool send(const nlohmann::json &message, nlohmann::json *response = nullptr);
    bool connected() const { return fd_ >= 0; }
    /// 只供事件迴圈觀察斷線；收發仍統一通過 send。
    int fd() const { return fd_; }
    void close();
private:
    /// 當前連線；-1 表示未連線。
    int fd_ = -1;
};
}
