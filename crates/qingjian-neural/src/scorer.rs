use std::path::Path;
use std::sync::Mutex;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use qingjian_format::{Container, Kind, Metadata};

use crate::model::PrefixCache;
use crate::vocab::EOS;
use crate::{CharLm, ModelConfig, NeuralError, Vocab, find_model, qjm};

/// 載入好的模型 + 字表：給「前文 + 候選」打分。
pub struct CharScorer {
    model: CharLm,
    vocab: Vocab,

    /// `.qjm` 的 `META`（名稱 / 許可證 / 署名）；三件套目錄沒有。
    metadata: Option<Metadata>,

    /// 最近一段前文的 K / V 快取（前文 token 與快取）：一次組句裡前文不變，候選換了只算候選。
    cache: Mutex<Option<(Vec<u32>, PrefixCache)>>,
}

impl CharScorer {
    /// 載入模型。`path` 是 `.qjm` 單檔案、三件套目錄（`model.safetensors` / `config.json` / `vocab.json`），
    /// 或裝著其中之一的目錄（按 [`find_model`] 挑）。
    pub fn load(path: &Path) -> Result<Self, NeuralError> {
        let source = if path.is_dir() {
            find_model(path).ok_or_else(|| NeuralError::NotFound(path.to_owned()))?
        } else {
            path.to_owned()
        };
        let device = default_device()?;
        let dtype = weight_dtype(&device);
        let scorer = if source.is_dir() {
            Self::load_directory(&source, dtype, device)?
        } else {
            Self::load_packed(&source, dtype, device)?
        };
        tracing::info!(
            source = %source.display(),
            name = scorer.metadata.as_ref().map(|m| m.name.as_str()).unwrap_or_default(),
            layers = scorer.model.config().n_layer,
            hidden = scorer.model.config().n_embd,
            vocab = scorer.vocab.len(),
            "神经语言模型已加载"
        );
        Ok(scorer)
    }

    /// 三件套目錄：權重 mmap 給 candle。
    fn load_directory(dir: &Path, dtype: DType, device: Device) -> Result<Self, NeuralError> {
        let config_path = dir.join(qjm::CONFIG_FILE);
        let text = std::fs::read_to_string(&config_path).map_err(|source| NeuralError::Io {
            path: config_path.clone(),
            source,
        })?;
        let cfg = ModelConfig::from_json(&text, &config_path)?;
        let vocab = Vocab::load(&dir.join(qjm::VOCAB_FILE))?;
        // SAFETY：mmap 的權重檔案在模型存活期間不改動
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[dir.join(qjm::WEIGHTS_FILE)], dtype, &device)?
        };
        Self::assemble(cfg, vocab, vb, device, None)
    }

    /// `.qjm`：容器 mmap 一次，三節各自切片；張量搬上裝置後容器就可以丟了。
    fn load_packed(path: &Path, dtype: DType, device: Device) -> Result<Self, NeuralError> {
        let container = Container::open(path, Kind::Model)?;
        let cfg = ModelConfig::from_json(&container.text(qjm::CONFIG_TAG)?, path)?;
        let vocab = Vocab::from_json(&container.text(qjm::VOCAB_TAG)?, path)?;
        let vb =
            VarBuilder::from_slice_safetensors(container.bytes(qjm::WEIGHTS_TAG)?, dtype, &device)?;
        Self::assemble(cfg, vocab, vb, device, Some(container.metadata().clone()))
    }

    fn assemble(
        cfg: ModelConfig,
        vocab: Vocab,
        vb: VarBuilder,
        device: Device,
        metadata: Option<Metadata>,
    ) -> Result<Self, NeuralError> {
        if vocab.len() != cfg.vocab_size {
            return Err(NeuralError::Corrupt("vocab size differs from config"));
        }
        let model = CharLm::load(vb, cfg, device)?;
        Ok(Self {
            model,
            vocab,
            metadata,
            cache: Mutex::new(None),
        })
    }

    /// 載入好的模型本體：逐字生成一類的實驗直接用它。
    pub fn model(&self) -> &CharLm {
        &self.model
    }

    /// `.qjm` 帶的後設資料（名稱 / 許可證 / 署名 / 引數量）；三件套目錄載入的沒有。
    pub fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    pub fn vocab(&self) -> &Vocab {
        &self.vocab
    }

    /// 教師強制計算 `log P(候選 | 完整拼音)`，只累加候選字元，不計 EOS。
    /// 與字級重排使用相同的求和口徑；超長輸入報錯，不截斷使用者拼音。
    pub fn score_p2c(&self, keys: &str, texts: &[&str]) -> Result<Vec<f64>, NeuralError> {
        let sep = self.vocab.sep().ok_or(NeuralError::Corrupt(
            "Hanzhang Tongbian model requires a <sep> token in its vocabulary",
        ))?;
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let head: Vec<u32> = std::iter::once(EOS)
            .chain(self.vocab.encode(keys))
            .collect();
        let tails: Vec<Vec<u32>> = texts.iter().map(|text| self.vocab.encode(text)).collect();
        let width = tails.iter().map(Vec::len).max().unwrap_or(0).max(1);
        if head.len() + width > self.model.config().context {
            return Err(NeuralError::Corrupt("P2C scoring exceeds model context"));
        }
        let batch = tails.len();
        let mut flat = vec![0u32; batch * width];
        let mut targets = vec![0u32; batch * width];
        for (row, tail) in tails.iter().enumerate() {
            flat[row * width] = sep;
            if !tail.is_empty() {
                flat[row * width + 1..row * width + tail.len()]
                    .copy_from_slice(&tail[..tail.len() - 1]);
                targets[row * width..row * width + tail.len()].copy_from_slice(tail);
            }
        }
        let idx = Tensor::from_vec(flat, (batch, width), self.model.device())?;
        let targets = Tensor::from_vec(targets, (batch, width, 1), self.model.device())?;
        let mut guard = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.as_ref().is_none_or(|(ids, _)| ids != &head) {
            let cache = self.model.prefix_cache(&head)?;
            *guard = Some((head, cache));
        }
        let (_, cache) = guard.as_ref().expect("filled above");
        let lp = self.model.log_probs_after(cache, &idx)?;
        drop(guard);
        let picked = lp.gather(&targets, 2)?.squeeze(2)?.to_vec2::<f32>()?;
        Ok(tails
            .iter()
            .zip(picked)
            .map(|(tail, row)| row[..tail.len()].iter().map(|&v| f64::from(v)).sum())
            .collect())
    }

    /// 每個候選接在 `context` 後面的 `log P(候選 | 前文)`，按字累加。
    /// 序列是 `<eos> + 前文 + 候選`；前文（不含最後一個 token）的 K / V 走快取，同一段前文只算一次，
    /// 每個候選只算「前文最後一個 token + 候選」這一小段；幾個候選拼成一個 batch、末尾補 0 對齊，一次前向。
    /// 前文加最長候選超過模型上下文時前文從左邊截（與訓練指令碼 `score.py` 一致）。
    pub fn score(&self, context: &str, texts: &[&str]) -> Result<Vec<f64>, NeuralError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let limit = self.model.config().context;
        let full: Vec<u32> = std::iter::once(EOS)
            .chain(self.vocab.encode(context))
            .collect();
        // 候選最長不能超過上下文減一（還要留前文的最後一個 token）：再長的從開頭截
        let tails: Vec<Vec<u32>> = texts
            .iter()
            .map(|text| {
                let mut ids = self.vocab.encode(text);
                if ids.len() > limit - 1 {
                    ids.drain(..ids.len() - (limit - 1));
                }
                ids
            })
            .collect();
        let longest = tails.iter().map(Vec::len).max().unwrap_or(0);
        let width = longest + 1;
        // 快取的是前文去掉最後一個 token 的部分，最後一個 token 放進每一行的開頭，它的輸出分佈給候選第一個字用
        let (last, head) = full.split_last().expect("has eos");
        let head = &head[head.len().saturating_sub(limit - width)..];
        let batch = tails.len();
        let mut flat = vec![0u32; batch * width];
        let mut targets = vec![0u32; batch * width];
        for (row, tail) in tails.iter().enumerate() {
            flat[row * width] = *last;
            flat[row * width + 1..row * width + 1 + tail.len()].copy_from_slice(tail);
            targets[row * width..row * width + tail.len()].copy_from_slice(tail);
        }
        let idx = Tensor::from_vec(flat, (batch, width), self.model.device())?;
        let targets = Tensor::from_vec(targets, (batch, width, 1), self.model.device())?;
        let mut guard = self
            .cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.as_ref().is_none_or(|(ids, _)| ids != head) {
            *guard = Some((head.to_vec(), self.model.prefix_cache(head)?));
        }
        let (_, cache) = guard.as_ref().expect("filled above");
        let lp = self.model.log_probs_after(cache, &idx)?;
        drop(guard);
        let picked = lp.gather(&targets, 2)?.squeeze(2)?.to_vec2::<f32>()?;
        Ok(tails
            .iter()
            .zip(picked)
            .map(|(tail, row)| row[..tail.len()].iter().map(|&v| f64::from(v)).sum())
            .collect())
    }
}

/// 權重與中間量的精度：Metal 上預設 f16（與 f32 打分一致，視訊記憶體減一半、略快），CPU 上 f32（candle 的 CPU f16 矩陣乘慢）；
/// 環境變數 `QINGJIAN_NEURAL_DTYPE=f32|f16` 可強制。
fn weight_dtype(device: &Device) -> DType {
    match std::env::var("QINGJIAN_NEURAL_DTYPE").as_deref() {
        Ok("f16") => DType::F16,
        Ok("f32") => DType::F32,
        _ if device.is_metal() => DType::F16,
        _ => DType::F32,
    }
}

#[cfg(not(feature = "metal"))]
fn default_device() -> Result<Device, NeuralError> {
    Ok(Device::Cpu)
}

#[cfg(feature = "metal")]
fn default_device() -> Result<Device, NeuralError> {
    // candle-metal 在建立裝置時會使用此類；舊版 macOS 缺少它會直接 panic。
    if objc2::runtime::AnyClass::get(c"MTLResidencySetDescriptor").is_none() {
        tracing::warn!("系统不支持 Metal residency set，本地模型改用 CPU");
        return Ok(Device::Cpu);
    }
    Ok(Device::new_metal(0)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 隨包知微模型的三件套；沒有就跳過這些測試。
    fn export_dir() -> Option<std::path::PathBuf> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/hanzhang-zhiwei");
        dir.join(qjm::WEIGHTS_FILE).exists().then_some(dir)
    }

    /// 用未拆字首的完整前向核對位置偏移、批次補齊、快取切換和長度邊界。
    #[test]
    fn p2c_cached_scores_match_full_forward() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/hanzhang-tongbian");
        if !dir.join(qjm::WEIGHTS_FILE).exists() {
            return;
        }
        let scorer = CharScorer::load(&dir).unwrap();
        for (keys, texts) in [
            ("jiekou", vec!["接口", "借口", "接", ""]),
            (
                "womenmingtiankaihui",
                vec!["我们明天开会", "我们明天", "我门明天开会"],
            ),
            ("jiekou", vec!["接口", "借口"]),
        ] {
            let cached = scorer.score_p2c(keys, &texts).unwrap();
            for (text, score) in texts.iter().zip(&cached) {
                let mut ids = vec![EOS];
                ids.extend(scorer.vocab.encode(keys));
                ids.push(scorer.vocab.sep().unwrap());
                let offset = ids.len() - 1;
                let tail = scorer.vocab.encode(text);
                ids.extend(&tail);
                let length = ids.len();
                let idx = Tensor::from_vec(ids, (1, length), scorer.model.device()).unwrap();
                let lp = scorer
                    .model
                    .log_probs(&idx)
                    .unwrap()
                    .to_vec3::<f32>()
                    .unwrap();
                let full: f64 = tail
                    .iter()
                    .enumerate()
                    .map(|(i, &id)| f64::from(lp[0][offset + i][id as usize]))
                    .sum();
                assert!(
                    (full - score).abs() < 0.08,
                    "{keys} {text}: {full} vs {score}"
                );
                eprintln!("P2C_CHECK {keys} {text:?} {score:.6} full={full:.6}");
            }
        }
        assert!(scorer.score_p2c(&"a".repeat(128), &["字"]).is_err());
        assert!(scorer.score_p2c("a", &[]).unwrap().is_empty());
    }

    /// 與訓練指令碼 `score.py` 對拍：同一模型、同一序列，log 機率要一致（數值差在 fp16 權重轉 f32 的誤差內）。
    #[test]
    fn matches_the_python_scorer() {
        let Some(dir) = export_dir() else {
            eprintln!("没有导出的模型，跳过");
            return;
        };
        let device = default_device().unwrap();
        let scorer = CharScorer::load_directory(&dir, weight_dtype(&device), device).unwrap();
        let scores = scorer
            .score("我今天想去", &["上海", "伤害", "吃饭"])
            .unwrap();
        assert!((scores[0] - -4.981).abs() < 0.05, "{scores:?}");
        assert!((scores[1] - -13.113).abs() < 0.05, "{scores:?}");
        assert!((scores[2] - -6.491).abs() < 0.05, "{scores:?}");
        // 單獨算與批次算一致
        let alone = scorer.score("我今天想去", &["伤害"]).unwrap();
        assert!(
            (alone[0] - scores[1]).abs() < 1e-3,
            "{alone:?} vs {scores:?}"
        );
        // 前文超長時從左截，不報錯
        let long: String = "很长的前文。".repeat(40);
        assert!(scorer.score(&long, &["上海"]).unwrap()[0] < 0.0);
        // 換過前文再換回來（快取重算）結果不變；候選比上下文還長也不報錯
        let again = scorer.score("我今天想去", &["上海"]).unwrap();
        assert!((again[0] - scores[0]).abs() < 1e-3, "{again:?}");
        let huge: String = "字".repeat(300);
        assert!(scorer.score("", &[huge.as_str()]).unwrap()[0] < 0.0);
    }

    /// 三件套打成 `.qjm` 再載入，分數與目錄載入一致，後設資料原樣帶回；目錄裡有 `.qjm` 時 `load(目錄)` 挑的是它。
    #[test]
    fn packed_file_scores_like_the_directory() {
        let Some(dir) = export_dir() else {
            eprintln!("没有导出的模型，跳过");
            return;
        };
        let out_dir = std::env::temp_dir().join("qingjian-neural-tests/packed");
        std::fs::create_dir_all(&out_dir).unwrap();
        let out = out_dir.join("model.qjm");
        let metadata = Metadata {
            name: "测试模型".to_owned(),
            license: "CC-BY-SA-4.0".to_owned(),
            ..Metadata::default()
        };
        let parameters = qjm::pack(&dir, &out, &metadata).unwrap();
        assert!(parameters > 1_000_000, "{parameters}");

        let packed = CharScorer::load(&out).unwrap();
        let meta = packed.metadata().unwrap();
        assert_eq!(meta.name, "测试模型");
        assert_eq!(meta.entries, parameters);
        // 與 load 同一裝置同一精度（workspace 一起測時 metal feature 會被統一開啟）
        let device = default_device().unwrap();
        let by_dir = CharScorer::load_directory(&dir, weight_dtype(&device), device).unwrap();
        let texts = ["上海", "伤害", "吃饭"];
        let a = packed.score("我今天想去", &texts).unwrap();
        let b = by_dir.score("我今天想去", &texts).unwrap();
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-4, "{a:?} vs {b:?}");
        }
        assert_eq!(find_model(&out_dir), Some(out.clone()));
        assert!(CharScorer::load(&out_dir).unwrap().metadata().is_some());

        // 別的 .qj 種類不認
        let error = CharScorer::load(&dir.join("../generated/lm.qj"))
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(
            error.contains("wrong data kind") || error.contains("io error"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(&out_dir);
    }
}

#[cfg(test)]
mod latency {
    use super::*;

    /// 延遲探針：前文 64 字 + 8 條約 8 字的候選，一次 batch。`cargo test --release -p qingjian-neural -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn batch_latency() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/hanzhang-zhiwei");
        let scorer = CharScorer::load(&dir).unwrap();
        let context: String =
            "今天下午的会议讨论了输入法的排序问题，大家觉得整句转换还可以再准一些，".repeat(2);
        let context: String = context.chars().take(64).collect();
        let texts = [
            "我们明天再讨论一下",
            "我们明天在讨论一下",
            "我们名天再讨论一下",
            "我门明天再讨论一下",
            "我们明天再讨论以下",
            "我们明天再讨论一夏",
            "我们明天再讨论移下",
            "我们明天再讨论亦下",
        ];
        for (label, ctx, n) in [
            ("空前文 1 条", "", 1),
            ("空前文 8 条", "", 8),
            ("64 字前文 1 条", context.as_str(), 1),
            ("64 字前文 8 条", context.as_str(), 8),
        ] {
            let _ = scorer.score(ctx, &texts[..n]).unwrap();
            let start = std::time::Instant::now();
            for _ in 0..10 {
                let _ = scorer.score(ctx, &texts[..n]).unwrap();
            }
            println!(
                "{label}（前文已缓存）: {:.2} ms",
                start.elapsed().as_secs_f64() * 100.0
            );
            let start = std::time::Instant::now();
            for i in 0..10 {
                // 每次換一段前文，逼它重算快取
                let fresh = format!("{ctx}{i}");
                let _ = scorer.score(&fresh, &texts[..n]).unwrap();
            }
            println!(
                "{label}（前文重算）: {:.2} ms",
                start.elapsed().as_secs_f64() * 100.0
            );
        }
    }
}
