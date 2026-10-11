#!/usr/bin/env bash
# 讓官網（qingjian-team/qingjian-web，Cloudflare Workers Builds 按倉庫提交自動構建）重新構建：
# 往它的 src/content/upstream.json 寫一筆「本次發版的標籤 / 文件提交」並推一個提交。
# 官網構建時拉最新 Release 的 releases.json，文件按記下的提交號拉主倉庫 docs/user；只在發版時呼叫，文件只隨發版更新。
#
# 用法：tools/release/bump-website.sh（在 release.yml 裡跑，讀 GITHUB_REF_NAME 標籤與 GITHUB_SHA）
# 需要環境變數 QINGJIAN_WEB_TOKEN：對 qingjian-web 有 Contents: write 的 fine-grained PAT。
set -euo pipefail

token=${QINGJIAN_WEB_TOKEN:?缺 QINGJIAN_WEB_TOKEN}
repo=${QINGJIAN_WEB_REPO:-qingjian-team/qingjian-web}
sha=${GITHUB_SHA:-$(git rev-parse HEAD)}
tag=${GITHUB_REF_NAME:?缺 GITHUB_REF_NAME（版本标签）}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
git clone -q --depth 1 "https://x-access-token:${token}@github.com/${repo}.git" "$work/web"
cd "$work/web"

file=src/content/upstream.json
mkdir -p "$(dirname "$file")"
message="同步主仓库：发版 ${tag}"
python3 - "$file" "$tag" "$sha" <<'PY'
import json, sys, datetime
path, release, sha = sys.argv[1:4]
json.dump(
    {"release": release, "docs": sha, "updated": datetime.datetime.now(datetime.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")},
    open(path, "w", encoding="utf-8"),
    ensure_ascii=False,
    indent=2,
)
open(path, "a", encoding="utf-8").write("\n")
PY
git add "$file"
if git diff --cached --quiet; then
  echo "官网仓库无需更新"
  exit 0
fi
git -c user.name="qingjian-ci" -c user.email="ci@qingjian.app" commit -q -m "$message" -m "主仓库提交 ${sha}"
git push -q origin HEAD
echo "已推送到 ${repo}：${message}"
