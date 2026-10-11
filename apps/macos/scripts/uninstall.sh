#!/bin/sh
# 解除安裝青簡：刪掉輸入法本體（系統級 /Library 與使用者級 ~/Library 兩處都看），加 --purge 連學習資料、配置、日誌一起刪。
# 這個指令碼會打進 .app 的 Resources 裡，裝了 pkg 的使用者直接執行它。
set -u
purge=0
[ "${1:-}" = "--purge" ] && purge=1

pkill -x qingjian-macos 2>/dev/null || true

if [ -d "/Library/Input Methods/Qingjian.app" ]; then
    echo "删除 /Library/Input Methods/Qingjian.app（需要管理员密码）"
    sudo rm -rf "/Library/Input Methods/Qingjian.app"
    sudo pkgutil --forget app.qingjian.inputmethod >/dev/null 2>&1 || true
fi
if [ -d "$HOME/Library/Input Methods/Qingjian.app" ]; then
    echo "删除 ~/Library/Input Methods/Qingjian.app"
    rm -rf "$HOME/Library/Input Methods/Qingjian.app"
fi

if [ "$purge" = 1 ]; then
    rm -rf "$HOME/Library/Application Support/Qingjian" "$HOME/Library/Logs/Qingjian"
    echo "学习数据、配置与日志已删除"
else
    echo "保留了 ~/Library/Application Support/Qingjian（学习数据、配置）与 ~/Library/Logs/Qingjian；要一起删请加 --purge"
fi
echo "已卸载。输入法列表里的「青简」条目会在注销再登录后消失。"
