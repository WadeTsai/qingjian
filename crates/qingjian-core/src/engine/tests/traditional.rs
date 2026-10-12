//! 繁體模式下各來源的候選都要轉：詞庫詞之外，自定義短語與模型直接生成的整句也是簡體出來的。
//! 雲端詞的對映見 `cloud`。

use super::*;
use crate::CustomPhrase;
use crate::sentence::SentenceScorer;

/// 假打分器：不打分，只會把整段按鍵「生成」成一句固定的簡體文字。
struct Generates(&'static str);

impl SentenceScorer for Generates {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        vec![-1.0; texts.len()]
    }

    fn generate(&self, _keys: &str, _beam: usize, _max_chars: usize) -> Vec<String> {
        vec![self.0.to_owned()]
    }
}

#[test]
fn custom_phrases_are_converted_in_traditional_mode() {
    let mut engine = engine();
    engine
        .set_custom_phrases(vec![CustomPhrase {
            code: "sd".into(),
            text: "设定".into(),
            position: 1,
            enabled: true,
        }])
        .unwrap();
    engine.set_traditional_mode(true);
    engine.set_input("sd");
    let query = engine.query().unwrap();
    let custom = query
        .candidates
        .items
        .iter()
        .find(|c| matches!(c.kind, CandidateKind::Custom(_)))
        .expect("自定義短語在候選裡");
    assert_eq!(custom.text, "設定");
}

#[test]
fn generated_sentences_are_converted_in_traditional_mode() {
    let mut engine =
        engine().with_sentence_scorer(Box::new(Generates("设定docker界面")), Some(0.5), None, None);
    engine.set_traditional_mode(true);
    // 中間夾著英文段，詞圖讀不通整段輸入，模型生成的整句才會進候選
    engine.set_input("shedingdockerjiemian");
    let query = engine.query().unwrap();
    let generated = query
        .candidates
        .items
        .iter()
        .find(|c| c.kind == CandidateKind::Generated)
        .expect("模型生成的整句在候選裡");
    assert_eq!(generated.text, "設定docker界面");
}
