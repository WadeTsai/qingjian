#!/usr/bin/env bash
# //! 僅刪除本安裝清單中的檔案，保留配置、詞庫、學習資料。
set -euo pipefail
install_prefix="$HOME/.local"
while (($#)); do
  case "$1" in
    --prefix) install_prefix=${2:?--prefix 需要路径}; shift 2 ;;
    --help) echo '用法：uninstall.sh [--prefix 安装时的绝对目录]'; exit 0 ;;
    *) echo "未知参数：$1" >&2; exit 2 ;;
  esac
done
python3 "$(dirname -- "${BASH_SOURCE[0]}")/files.py" uninstall "$install_prefix"
echo '卸载完成；请手动结束 Server 并重启 Fcitx5，用户数据已保留。'
