# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""把词汇等级表转成青简的 `levels-<语言>.tsv`（`词\t等级`，`# levels` 行給出等級從易到難的順序）。

英文：CEFR-J Wordlist 1.5（A1–B2）+ Octanove Vocabulary Profile（C1/C2），`headword,pos,CEFR,…`，
      斜杠分隔的拼写变体各算一条，同一个词多个词性取最低等级。
日文：elzup/jlpt-word-list 的 N5–N1（源自 tanos.co.uk），`expression,reading,meaning,tags`，
      `; ` 分隔的写法各算一条，去掉 `～` 与括号说明，同一个词出现在多级取最易的一级。

用法：
    uv run tools/corpus/levels.py en data/levels/cefrj-vocabulary-profile-1.5.csv \
        data/levels/octanove-vocabulary-profile-c1c2-1.0.csv -o assets/levels/levels-en.tsv
    uv run tools/corpus/levels.py ja data/levels/jlpt-n5.csv data/levels/jlpt-n4.csv \
        data/levels/jlpt-n3.csv data/levels/jlpt-n2.csv data/levels/jlpt-n1.csv -o assets/levels/levels-ja.tsv
"""

from __future__ import annotations

import argparse
import csv
import re
from pathlib import Path

EN_LEVELS = ["A1", "A2", "B1", "B2", "C1", "C2"]
JA_LEVELS = ["N5", "N4", "N3", "N2", "N1"]

EN_HEADER = [
    "# 英文詞彙等級（CEFR）。來源：The CEFR-J Wordlist Version 1.5, compiled by Yukio Tono, Tokyo University of Foreign Studies,",
    "# retrieved from http://www.cefr-j.org/download.html（A1–B2，研究與商業用途免費，須署名）；",
    "# Octanove Vocabulary Profile C1/C2 1.0（CC BY-SA 4.0，https://github.com/openlanguageprofiles/olp-en-cefrj）。",
    "# 由 tools/corpus/levels.py 生成。格式：詞\\t等級；`# levels` 行是等級從易到難的順序。",
]
JA_HEADER = [
    "# 日文詞彙等級（JLPT）。來源：tanos.co.uk 的 JLPT 詞表（Jonathan Waller，CC BY），",
    "# 經 elzup/jlpt-word-list（MIT，https://github.com/elzup/jlpt-word-list）整理為 CSV。",
    "# 由 tools/corpus/levels.py 生成。格式：詞\\t等級；`# levels` 行是等級從易到難的順序。",
]


def clean_english(headword: str) -> list[str]:
    """`a.m./A.M./am/AM` → 各拼写；统一小写。"""
    words = []
    for part in headword.split("/"):
        word = part.strip().lower()
        if word and word not in words:
            words.append(word)
    return words


def clean_japanese(expression: str) -> list[str]:
    """`足; 脚` → 两条；`～円` → `円`；`こす (みずを～)` → `こす`。"""
    words = []
    for part in re.split(r"[;；]", expression):
        word = re.sub(r"[（(].*?[）)]", "", part)
        word = word.replace("～", "").replace("〜", "").strip()
        if word and word not in words:
            words.append(word)
    return words


def convert(language: str, sources: list[Path]) -> tuple[list[str], dict[str, int], list[str]]:
    if language == "en":
        levels, header = EN_LEVELS, EN_HEADER
    else:
        levels, header = JA_LEVELS, JA_HEADER
    rank = {level: index for index, level in enumerate(levels)}
    table: dict[str, int] = {}
    for source in sources:
        with source.open(encoding="utf-8", newline="") as handle:
            for row in csv.DictReader(handle):
                if language == "en":
                    level = row["CEFR"].strip()
                    words = clean_english(row["headword"])
                else:
                    tags = row["tags"].split()
                    # 檔名定級（n5.csv 裡的就是 N5），tags 只用來兜底
                    match = re.search(r"n([1-5])", source.stem)
                    level = f"N{match.group(1)}" if match else next(
                        (t.replace("JLPT_N", "N") for t in tags if t.startswith("JLPT_N")), ""
                    )
                    words = clean_japanese(row["expression"])
                if level not in rank:
                    continue
                for word in words:
                    table[word] = min(table.get(word, rank[level]), rank[level])
    return levels, table, header


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("language", choices=["en", "ja"])
    parser.add_argument("sources", nargs="+", type=Path)
    parser.add_argument("-o", "--output", type=Path, required=True)
    args = parser.parse_args()
    levels, table, header = convert(args.language, args.sources)
    lines = header + ["# levels\t" + "\t".join(levels)]
    for word, index in sorted(table.items(), key=lambda item: (item[1], item[0])):
        lines.append(f"{word}\t{levels[index]}")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")
    counts = {level: 0 for level in levels}
    for index in table.values():
        counts[levels[index]] += 1
    print(f"{args.output}: {len(table)} 词 {counts}")


if __name__ == "__main__":
    main()
