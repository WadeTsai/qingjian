//! 附加候選：日期時間等快捷項、中英混輸的英文詞與補全、emoji。

use super::*;

impl Engine {
    /// 精確匹配自定義輸入碼時，數字鍵應選擇候選。
    pub(super) fn has_custom_phrase(&self) -> bool {
        !self.english_mode
            && self
                .custom_phrases
                .iter()
                .any(|p| p.enabled && p.code == self.composition.scope())
    }

    /// 所有普通候選完成排序後按輸入碼固定位置。
    pub(super) fn insert_custom_phrases(&self, items: &mut Vec<Candidate>) {
        if self.english_mode {
            return;
        }
        let mut phrases: Vec<_> = self
            .custom_phrases
            .iter()
            .filter(|p| p.enabled && p.code == self.composition.scope())
            .collect();
        phrases.sort_by_key(|p| p.position);
        for phrase in phrases {
            items.insert(
                (phrase.position - 1).min(items.len()),
                Candidate {
                    text: phrase.text.clone(),
                    kind: CandidateKind::Custom(phrase.position),
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                    aux_code: None,
                },
            );
        }
    }

    /// 日期 / 時間 / 星期這類快捷候選插在本地首選之後：`rq` 首選仍是詞庫裡的詞，快捷寫法緊隨其後。
    pub(super) fn insert_shortcuts(&self, items: &mut Vec<Candidate>, scope: &str) {
        let expression_char =
            if self.zhuyin && crate::zhuyin::layout::map_key(self.modes().expression).is_some() {
                '\0'
            } else {
                self.modes().expression
            };
        // 嗯 的標準音 ng 不是拼音音節（音節表不收嘆詞寫法）；整段正好是 ng 時按快捷候選給它，排第一。
        // 只認全拼：雙拼與注音裡這兩個鍵是別的音
        if scope == "ng" && self.shuangpin.is_none() && !self.zhuyin {
            items.insert(
                0,
                Candidate {
                    text: "嗯".to_owned(),
                    kind: CandidateKind::Shortcut,
                    syllables: Vec::new(),
                    reading: None,
                    translation: None,
                    aux_code: None,
                },
            );
        }
        let shortcuts = shortcut::candidates(scope, expression_char, &jiff::Zoned::now());
        if shortcuts.is_empty() {
            return;
        }
        let position = items.len().min(1);
        items.splice(position..position, shortcuts);
    }

    /// 中英混輸：整段輸入是英文詞就把它加進候選。
    /// 預設作為拼音「不像話」（切不動、或除末尾外還有聲母縮寫 / 殘缺音節）時排第一，否則排第二；
    /// 開了中文優先（`chinese_first`）整句 / 首箇中文候選已經在前，英文詞排第二。沒有中文候選時總在第一。
    /// 兩字母的全大寫縮寫（`mp` → MP、`bm` → BM）是個例外：整段太短、幾乎總是在打中文（門票 / 編碼），
    /// 這種讓中文先；超過兩個字母的正文英文（cargo / rust）照舊——按詞頻一刀切會把它們一起擠掉。
    /// 例外只管**沒選過**的英文詞：使用者選過的照舊排第一（選過 OK，下次敲 `ok` 還是 OK 在前）。
    pub(super) fn insert_english(&self, items: &mut Vec<Candidate>, unlikely_pinyin: bool) {
        if !self.english_in_chinese {
            return;
        }
        let lists = self.english_lists();
        if lists.is_empty() {
            return;
        }
        let text = self.composition.scope();
        if text.contains('\'') {
            return;
        }
        let english_candidate = |word: &str| Candidate {
            text: word.to_owned(),
            kind: CandidateKind::English,
            syllables: Vec::new(),
            reading: None,
            translation: None,
            aux_code: None,
        };
        let word = lists.iter().find_map(|words| words.get(text));
        // 這段字母下使用者選中文詞（`key` → 可以）比選英文詞的次數多：中文詞留在第一，英文讓到後面；
        // 拼音再不像話也是他自己教的
        let chosen = items
            .first()
            .filter(|c| c.kind == CandidateKind::Chinese)
            .map_or(0, |c| self.learner.choice_weight(text, &c.text));
        let english_weight = word.map_or(0, |w| self.learner.weight(w));
        // 兩字母全大寫縮寫（mp → MP、bm → BM）讓中文先：整段太短，幾乎總是在打中文。
        // 但**不壓過學習記錄**：`ok` / `pc` / `ll` 同樣滿足「兩個字母的全大寫縮寫」，一刀切會把它們一起
        // 翻成中文；而且選過 OK 的使用者下次敲 `ok` 本該還是它排第一。所以這條只對使用者**沒選過**的英文詞生效
        //（`record` 對英文候選也記次數，所以 `english_weight` 就是「選過沒有」）。
        let short_acronym = english_weight == 0
            && text.len() <= 2
            && word.is_some_and(|word| word.chars().all(|c| c.is_ascii_uppercase()));
        let english_first =
            !self.chinese_first && unlikely_pinyin && chosen <= english_weight && !short_acronym;
        let mut position = if items.is_empty() || english_first {
            0
        } else {
            1
        };
        if let Some(word) = word {
            items.insert(position, english_candidate(word));
            position += 1;
        }
        // 英文補全：拼音不像話時（`compa` 切成 co'm'pa），整段多半是在打英文詞的前面幾個字母，補全緊跟在精確詞之後；
        // 個人詞表在前，兩張表裡都有的只出一次。
        if unlikely_pinyin && text.len() >= MIN_COMPLETION_LETTERS {
            let mut budget = ENGLISH_COMPLETIONS;
            for words in &lists {
                for word in words.complete(text, budget) {
                    if items
                        .iter()
                        .any(|c| c.kind == CandidateKind::English && c.text == word)
                    {
                        continue;
                    }
                    items.insert(position, english_candidate(word));
                    position += 1;
                    budget -= 1;
                }
                if budget == 0 {
                    break;
                }
            }
        }
    }

    /// 給英文候選用的詞表，個人的在前、隨包的在後；一張都沒有就是空。
    /// 整段作用域本身就是個英文詞（`database`、`agent`）：使用者多半在打那個詞。
    pub(in crate::engine) fn scope_is_english_word(&self) -> bool {
        let scope = self.composition.scope();
        self.english_in_chinese
            && !scope.is_empty()
            && self
                .english_lists()
                .iter()
                .any(|words| words.get(scope).is_some())
    }

    pub(super) fn english_lists(&self) -> Vec<&WordList> {
        self.learner
            .user_english()
            .into_iter()
            .chain(self.english.as_ref())
            .collect()
    }

    /// emoji 候選：前幾個中文候選裡有配 emoji 的，emoji 緊跟在那個詞後面，右側標註它對應的詞。
    /// 詞後面緊挨著的英文詞候選（中文優先時 `key` → 可以、key）不被 emoji 擠開，emoji 排在它之後。
    pub(super) fn insert_emoji(&self, items: &mut Vec<Candidate>) {
        let Some(table) = self.emoji.as_ref().filter(|_| self.emoji_candidates) else {
            return;
        };
        let mut inserted = 0;
        let mut index = 0;
        let mut scanned = 0;
        while index < items.len() && scanned < EMOJI_SCAN && inserted < EMOJI_TOTAL {
            let item = &items[index];
            index += 1;
            if !matches!(
                item.kind,
                CandidateKind::Chinese | CandidateKind::Sentence | CandidateKind::English
            ) {
                continue;
            }
            scanned += 1;
            // 英文詞按小寫查英文表（smile → 😀）
            let key = if item.kind == CandidateKind::English {
                item.text.to_lowercase()
            } else {
                item.text.clone()
            };
            let emojis = table.lookup(&key);
            if emojis.is_empty() {
                continue;
            }
            let word = item.text.clone();
            let syllables = item.syllables.clone();
            if item.kind != CandidateKind::English {
                while items
                    .get(index)
                    .is_some_and(|next| next.kind == CandidateKind::English)
                {
                    index += 1;
                }
            }
            for emoji in emojis.iter().take(EMOJI_PER_WORD) {
                if inserted >= EMOJI_TOTAL {
                    break;
                }
                items.insert(
                    index,
                    Candidate {
                        text: emoji.clone(),
                        kind: CandidateKind::Emoji,
                        syllables: syllables.clone(),
                        reading: Some(word.clone()),
                        translation: None,
                        aux_code: None,
                    },
                );
                index += 1;
                inserted += 1;
            }
        }
    }
}
