# 排序常數掃描（2026-09-12）

`sentence` 裡個人 n-gram 的插值常數（λ 0.8 / K 8 / 封頂 0.5 / 三元折扣 0.75）與 `correction` 裡的敲錯代價（換位 5 / 鄰鍵 5 / 多鍵 5.5 /
少鍵 5.5、個人折扣上限 3、整段糾錯 5）此前只按量級定過，沒有用資料驗過。這次把它們做成引擎可設（`Engine::set_interpolation` /
`set_typo_costs`，CLI `--tune 名=值`），在凍結日誌上掃了一遍。**結論：全部在平臺區，一個都不改。**

## 尺子

- 凍結日誌：2026-09-12 早上的 `input-log.jsonl` 副本，27516 行，可評上屏 12886 條（詞 9241 / 整句 3376 / 英文 269）。
- 回放：`qingjian-cli --replay`，記憶體學習從零起，雲聯想關（配置副本 `[predict] enabled = false`），不帶神經重排（每組 7 秒，四路並行）。
  最後兩組帶神經重排（`--neural data/models/hanzhang-zhiwei`，Metal，每組十分鐘）複核。
- 指標：各來源首選命中數（報告行末的「命中數」，比百分比細）。
- 掃參前先確認重構零變化：改後二進位制預設引數的報告與改前逐行相同。
- 這把尺子的偏差見 `docs/plan/model-eval.md`：整句「選了」多半是當時基線自己的輸出，天然偏向現值；本輪又發現回放要把原樣上屏也走一遍
  `take_raw`，否則個人英文詞學不到、英文首選低 10 個點（72.5% → 82.9%）。這一改在第四輪之後，所以下表前三輪的基線是 詞 8033 / 整句 3043，
  第四輪起是 8032 / 3045。

## 單軸與組合（不帶神經重排，相對預設的命中數差）

| 設定 | 詞 | 整句 |
|---|---|---|
| `cap=0.3` | +5 | -14 |
| `cap=0.45` | +5 | -6 |
| `cap=0.4` | +7 | -6 |
| `cap=0.55` | +0 | +0 |
| `cap=0.6` | +0 | -2 |
| `cap=0.7` | -9 | -33 |
| `cap=0.9` | -81 | -87 |
| `correction=3` | +2 | -36 |
| `correction=4.5` | +6 | -10 |
| `correction=4` | +5 | -21 |
| `correction=6` | -11 | -3 |
| `correction=8` | -37 | -24 |
| `discount=0.5` | +15 | -10 |
| `discount=0.9` | +1 | +2 |
| `extra=4,missing=4` | +14 | -81 |
| `extra=4.5,missing=4.5` | +7 | -65 |
| `extra=5,missing=5` | +3 | -10 |
| `extra=6.5,missing=6.5` | -8 | -7 |
| `extra=7,missing=7` | -19 | -16 |
| `k=16` | -1 | -2 |
| `k=2` | +0 | -8 |
| `k=32` | -9 | -8 |
| `k=4` | +1 | -3 |
| `lambda=0.5` | +0 | -7 |
| `lambda=0.65` | -2 | +4 |
| `lambda=0.7,transpose=6,substitute=6,extra=6,missing=6` | -7 | +15 |
| `lambda=0.75,substitute=5.5` | +2 | +18 |
| `lambda=0.75,substitute=6` | +3 | +9 |
| `lambda=0.75,transpose=5.5,substitute=5.5,extra=5.5,missing=5.5` | +2 | +18 |
| `lambda=0.75,transpose=6,substitute=6,extra=6,missing=6` | -2 | +14 |
| `lambda=0.75` | +3 | +10 |
| `lambda=0.7` | -1 | +10 |
| `lambda=0.9` | +0 | -30 |
| `lambda=1.0` | -59 | -125 |
| `substitute=5.5,extra=6,missing=6` | -5 | +17 |
| `substitute=5.5` | +0 | +13 |
| `transpose=3.5,substitute=3.5` | +4 | -81 |
| `transpose=4,substitute=4` | +1 | -40 |
| `transpose=4.5,substitute=4.5` | +2 | -11 |
| `transpose=5,substitute=6` | +1 | +8 |
| `transpose=5.5,substitute=5.5,extra=5.5,missing=5.5` | +0 | +13 |
| `transpose=5.5,substitute=5.5` | +0 | +13 |
| `transpose=6,substitute=5` | +0 | +0 |
| `transpose=6,substitute=6,discount=0.9` | -1 | +5 |
| `transpose=6,substitute=6,extra=6,missing=6` | -3 | +14 |
| `transpose=6,substitute=6,lambda=0.65,discount=0.9` | -1 | +0 |
| `transpose=6,substitute=6,lambda=0.65` | -1 | +2 |
| `transpose=6,substitute=6` | +1 | +8 |
| `transpose=6.5,substitute=6.5` | +2 | +2 |
| `transpose=7,substitute=7` | -2 | -3 |
| `typo-cap=1` | -2 | -2 |
| `typo-cap=2` | +0 | +0 |
| `typo-cap=4` | +0 | +0 |

讀法：

- λ：0.7–0.75 比 0.8 整句多 10 條（0.3 個點），0.9 起掉，1.0（不要個人一元）掉 125 條。K 從 2 到 32 全平。封頂 0.4–0.6 平，0.7 起掉，
  0.9 掉 80 多條：個人資料壓死靜態模型是真的壞。三元折扣 0.5 詞 +15 / 整句 −10，0.9 平。
- 敲錯代價：鄰鍵（substitute）是唯一有反應的軸，5 → 5.5 整句 +13、6 +8、6.5 +2；換位單獨動沒變化（`transpose=6` 與預設逐條相同）。
  多鍵 / 少鍵往便宜走整句掉得快（4 掉 81 條）、詞略漲，5.5 就是峰。折扣上限 1 / 2 / 3 / 4 幾乎沒差：回放從零學，個人敲錯表攢不到幾次，
  這一項在回放裡不可辨。整段糾錯代價 5 是峰（3 掉 36、8 掉 24）。
- 最好的組合 `lambda=0.75,substitute=5.5`：詞 +2 / 整句 +18（0.5 個點）。

## 帶神經重排複核

殼裡現在模型是開著的，插值與代價都在重排之前起作用，最終要看重排之後的數字：

| 設定 | 詞 | 整句 | 英文 |
|---|---|---|---|
| 預設 | 8085 | 3018 | 223 |
| `lambda=0.75,substitute=5.5` | 8080 | 3022 | 223 |

差 −5 / +4，是噪聲。不帶重排時的 +18 也只有 0.5 個點、在尺子偏差之內，所以不改。

順帶：神經重排在這把尺子上詞 +53、整句 −27——整句掉是已知偏差（日誌裡的整句是舊基線接受的輸出），整句質量看 `data/eval/sentences.tsv`。

## 沒命中的到底是什麼

常數不是瓶頸，那瓶頸在哪：

- 整句不在候選 328 條：16 條 青簡（專名，詞庫沒有）；294 條與現在首選同長、只是同音換字（184 條差一字、112 條差兩字），
  這是上下文的活（神經重排、個人 n-gram），而且其中不少「選了」本身是當時接受的錯句；18 條長度不同，是切分 / 簡拼尾巴。
- 詞沒命中 1208 條：742 條是 5 鍵以上、當時從詞候選裡選的短語（導航欄 / 候選框 / 對齊 / 我繃不住了），多半是詞庫缺詞；
  最常重複的鍵 `ba`（45 條）看上下文、`wode`（34 條）是詞庫缺「我的」（todo 裡已有）。
- 英文 46 條沒命中裡 33 條是排在第 2–4 位（hello → helloWorld、com → commit、down → download），個人英文詞的加權可以再看；13 條不在候選是
  當時敲錯的拼寫（trasnformer）與個人詞。

下一步按這個順序更值：從日誌挖詞庫缺詞（短語 + 專名），個人 n-gram 與神經重排在同音換字上的淨效果（`rescored` 欄位攢夠後看），
`retype` 事件攢夠後再回頭看敲錯代價與折扣上限。

## 怎麼復現

```bash
cp ~/Library/Application\ Support/Qingjian/input-log.jsonl /tmp/frozen.jsonl
# 配置副本里把 [predict] enabled 改成 false
cargo build --release -p qingjian-cli
printf '\nlambda=0.75\nsubstitute=5.5\n' > /tmp/settings.txt
tools/eval/sweep.sh /tmp/frozen.jsonl /tmp/config.replay.toml /tmp/settings.txt /tmp/sweep.tsv
```

同一份凍結日誌、同一個二進位制才能比；日誌是活的，改了尺子（比如原樣上屏進學習）要連預設一起重跑。
