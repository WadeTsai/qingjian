//! 整句候選：把詞圖裡重排過的前幾條路徑變成候選，以及整句轉換本身。

use super::*;

impl Engine {
    /// 整句候選。沒有英文尾段時是整段拼音的轉換（[`Self::plain_sentence`]），排在開頭的英文候選之後。
    /// 有英文尾段且英文讀法勝出（`head_wins`）時，頭段的轉換加上那個詞排第一（`woxiangxuehaorust` → 我想學好rust），
    /// 整段也能讀成拼音的再把拼音讀法的整句放在第二；英文讀法輸了就不出（`diaoyong` 不出 掉Yong），
    /// 免得把真正要的候選往後擠。
    /// `typos` 為假時詞圖裡不加敲錯邊（整段一處編輯的糾錯已經生效，不在糾正後的拼音上再猜第二處）。
    pub(in crate::engine) fn insert_sentence(
        &self,
        items: &mut Vec<Candidate>,
        segmentations: &[Segmentation],
        typos: bool,
        english_tail: Option<&EnglishTail>,
        head_wins: bool,
    ) {
        let Some(best) = segmentations.first() else {
            return;
        };
        let keys = self.composition.scope();
        let first_segmentation = |text: &str| parser::segment(text).ok()?.into_iter().next();
        match english_tail {
            Some(tail) if head_wins => {
                if let Some(mixed) = self.mixed_sentence(best, tail, typos) {
                    items.insert(0, mixed);
                }
                // 整段讀成拼音的那條本來就是「另一種讀法」，只給一條
                if tail.competes
                    && let Some(full) = first_segmentation(keys)
                    && let Some(plain) = self.plain_sentence(items, &full, typos, false).pop()
                {
                    let position = items.len().min(1);
                    items.insert(position, plain);
                }
            }
            _ => {
                // 整段本身就是英文詞（`database`）：備選整句會把那條英文候選再往後擠，只給一條
                let alternates = !self.scope_is_english_word();
                let sentences = self.plain_sentence(items, best, typos, alternates);
                let position = leading_english(items);
                // 詞圖讀不通整段時模型直接生成的整句排在詞圖那幾條前面：這時詞圖給的是把英文段
                // 硬讀成拼音的結果（`yongdockerbushuhenfangbian` → 用的哦乘客仍不熟很方便），排它前面沒有可惜的
                // `typos` 為假就是拼寫糾錯已經生效（見呼叫處），那時 `best` 是糾正後的切分
                let generated: Vec<Candidate> = self
                    .generated_sentence_candidates(best, keys, !typos)
                    .into_iter()
                    .filter(|g| {
                        !items
                            .iter()
                            .chain(sentences.iter())
                            .any(|c| c.text == g.text)
                    })
                    .collect();
                let generated_count = generated.len();
                for (offset, candidate) in generated.into_iter().enumerate() {
                    items.insert((position + offset).min(items.len()), candidate);
                }
                for (offset, plain) in sentences.into_iter().enumerate() {
                    items.insert(
                        (position + generated_count + offset).min(items.len()),
                        plain,
                    );
                }
            }
        }
    }

    /// 整段拼音的整句候選：最優切分至少兩個音節、且最優路徑不止一個詞時才有（空格上屏的就是它）。
    /// 整段本身就是詞庫裡的詞時不重複；有音節沒轉成字的不算句子。
    /// 詞級候選裡已有同文本同讀音的候選時不出（那條留在詞級排序給它的位置），同文本不同讀音的從 `items` 裡去掉。
    ///
    /// `alternates` 為真且輸入夠長時把重排後的第 2、3 條路徑也給出來（[`SENTENCE_CANDIDATES`] 條封頂）：
    /// 長句錯一個字就得整句拆開重打，一條候選不夠用；短輸入不給，那幾格留給詞級候選更值。
    /// 備選只收真整句（路徑上不止一個詞），「整段讀成一個詞」那種噪聲通道的判斷只認第一條。
    pub(in crate::engine) fn plain_sentence(
        &self,
        items: &mut Vec<Candidate>,
        best: &Segmentation,
        typos: bool,
        alternates: bool,
    ) -> Vec<Candidate> {
        if best.syllables.len() < 2 {
            return Vec::new();
        }
        let wanted = if alternates && best.syllables.len() >= ALTERNATE_MIN_SYLLABLES {
            SENTENCE_CANDIDATES
        } else {
            1
        };
        let mut paths = self.sentence_paths(&best.patterns(), typos, false, wanted);
        if paths.is_empty() {
            return Vec::new();
        }
        // 不按原樣讀的路徑（敲錯邊 / 模糊音）不許壓過「敲的拼音本身就是一個詞」：`jineng` 按 `jin eng` 切時
        // 詞圖裡沒有 技能，敲錯邊讀出 近藤；`ceshi` 讀出 的是。詞級候選裡有音節正好拼成整段輸入的詞時退回原樣的路徑
        if paths[0].altered() {
            let letters = best.joined("");
            let spelled_exactly = items
                .iter()
                .any(|c| c.kind == CandidateKind::Chinese && c.syllables.concat() == letters);
            if spelled_exactly {
                paths = self.sentence_paths(&best.patterns(), false, false, wanted);
            }
        }
        let mut out: Vec<Candidate> = Vec::new();
        for (rank, conversion) in paths.into_iter().enumerate() {
            if out.len() >= wanted {
                break;
            }
            // 最優路徑不合格就一條都不出：它不合格多半是「整段本來就是一個詞」（`nihao` → 你好），
            // 這時越過它去拿第二條路徑，等於把 你號 插到詞庫詞 你好 前面
            let first = rank == 0;
            if conversion.has_placeholder() {
                if first {
                    return Vec::new();
                }
                continue;
            }
            // 整段本來就是一個詞時不出整句；但路徑靠敲錯變體把整段讀成的一個詞（`meiganxi` → 沒關係）是噪聲通道的判斷，
            // 詞級查詢按原樣查不到它，作為普通詞候選插到最前。只讀了一部分（末尾沒打完的音節沒算進去）的不插。
            // 備選只收真整句，單個詞的第 2、3 條路徑是噪聲
            let kind = if conversion.word_count() >= 2 {
                CandidateKind::Sentence
            } else if first
                && conversion.altered()
                && conversion.syllables.len() == best.syllables.len()
            {
                CandidateKind::Chinese
            } else if first {
                return Vec::new();
            } else {
                continue;
            };
            if out.iter().any(|c| c.text == conversion.text) {
                continue;
            }
            // 詞級候選裡已經有同樣的文本：讀音也相同就是同一個候選，不重複插、詞留在詞級排序給它的位置
            //（先是 / 有的 這種整句恰好拼成一個詞的，詞級排序更可信）；讀音不同的是按別的讀音對上的詞
            //（雲端學來的錯讀音使用者詞 `我的 wo di` 靠敲錯變體對上 `wode`），那條不是這個候選，去掉它，整句以正確讀音頂上
            if let Some(index) = items.iter().position(|c| c.text == conversion.text) {
                if items[index].syllables == conversion.syllables {
                    // 首選與詞級候選撞上：整句這邊一條都不出（備選插在它前面會繞到那條詞前面去）
                    if first {
                        return Vec::new();
                    }
                    continue;
                }
                items.remove(index);
            }
            out.push(Candidate {
                text: conversion.text,
                kind,
                syllables: conversion.syllables,
                reading: None,
                translation: None,
                aux_code: None,
            });
        }
        out
    }

    /// 跑一次整句轉換：主詞庫 + 使用者詞（含模糊音與敲錯寫法，命中的按代價扣分），靜態語言模型與個人 n-gram 插值，使用者選擇次數加分。
    /// `typos` 為假時不加敲錯邊。
    pub(in crate::engine) fn convert_sentence(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        typos: bool,
    ) -> Option<Conversion> {
        self.convert_sentence_with(patterns, typos, false)
    }

    /// 同 [`Self::convert_sentence`]，`whole` 為真時末尾單字母也讀（[`sentence::convert_whole`]），只給比分用。
    /// 接了神經重打分器時取前 [`RESCORE_PATHS`] 條路徑，按「路徑分 + λ·(神經分 − 靜態分)」重排（[`Self::rescore_paths`]）：
    /// 神經分替換的是靜態二元模型那部分判斷，個人 n-gram 插值、使用者加分、敲錯代價原樣保留。
    /// 返回重排後的第一條；`score` 始終是靜態尺度的路徑分，神經分不寫進去，跨讀法的比較才還成立。
    pub(in crate::engine) fn convert_sentence_with(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        typos: bool,
        whole: bool,
    ) -> Option<Conversion> {
        self.sentence_paths(patterns, typos, whole, 1)
            .into_iter()
            .next()
    }

    /// 重排後的整句路徑，最好的在前。`want` 是呼叫方要幾條：只要一條時沒接模型就只算一條
    /// （拼寫糾錯對每個候選都要轉一次，多算幾條純是浪費）；要備選時至少算這麼多條。
    pub(in crate::engine) fn sentence_paths(
        &self,
        patterns: &[qingjian_dictionary::SyllablePattern<'_>],
        typos: bool,
        whole: bool,
        want: usize,
    ) -> Vec<Conversion> {
        let dictionaries = self.all_dictionaries();
        let expanded = self.expand_positions(patterns, typos);
        let k = if self.has_sentence_scorer() {
            RESCORE_PATHS.max(want)
        } else {
            want
        };
        let mut paths = sentence::convert_paths(
            &dictionaries,
            &expanded.positions(),
            whole,
            k,
            &*self.language_model,
            self.personal(),
            |text| self.learner.weight(text),
            |index, syllable| expanded.cost(index, syllable),
            &mut self.span_cache.borrow_mut(),
        );
        // 與最優路徑差得太遠的不參與：那種差距多半是個人 n-gram 拉開的
        if paths.len() > 1 {
            let floor = paths[0].score - self.neural_margin;
            paths.retain(|p| p.score >= floor);
            // 條件是這批路徑共同解釋的那段按鍵，不是整個作用域：英文尾巴那條只轉換了 head，
            // 拿整段當條件會讓它憑空背上沒覆蓋的字母
            let keys: String = patterns.iter().map(|p| p.text).collect();
            self.rescore_paths(&mut paths, &keys);
        }
        paths
    }
}
