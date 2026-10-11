//! 問字模式答案的過濾：模型有時不答字，而是把拼音問題整句「翻譯」回來（`?san'ge'zhi'shi'shen'me'zi` → 三個直是什麼字）。

/// `answer` 是不是在複述問題而不是作答：與本地轉出的問題一樣、包含問題的大半，或者問題不短而答案跟它一樣長。
/// `guess` 為空（本地轉不出問題）時只按「答案裡帶疑問詞」判斷。
pub fn restates_question(answer: &str, guess: &str) -> bool {
    let answer = answer.trim();
    if answer.is_empty() {
        return true;
    }
    if answer == guess {
        return true;
    }
    let answer_len = answer.chars().count();
    let guess_len = guess.chars().count();
    if guess_len >= 4 && answer_len >= guess_len {
        return true;
    }
    // 問題的後半截（… 是什麼字 / 怎麼讀 / 什麼意思）出現在答案裡
    if guess_len >= 4 {
        let tail: String = guess.chars().skip(guess_len / 2).collect();
        if answer.contains(&tail) {
            return true;
        }
    }
    ["什么", "怎么", "哪个", "是不是", "吗"]
        .iter()
        .any(|word| answer_len >= 3 && answer.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restated_questions_are_dropped_and_real_answers_kept() {
        let guess = "三个直是什么字";
        assert!(restates_question("三个直是什么字", guess));
        assert!(restates_question("三个直是什么字？", guess));
        assert!(restates_question("三个直叠在一起是什么字", guess));
        assert!(restates_question("", guess));
        assert!(!restates_question("矗", guess));
        assert!(!restates_question("矗 chù", guess));
    }

    #[test]
    fn short_questions_keep_short_answers() {
        assert!(!restates_question("水", ""));
        assert!(!restates_question("谁", "水"));
        assert!(!restates_question("巴黎", "法国首都"));
        assert!(!restates_question("Paris", ""));
        assert!(restates_question("法国首都是什么", ""));
    }
}
