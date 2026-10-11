//! 查詢與呈現：重新整理候選、畫候選窗、整句補全、翻頁與高亮。

use super::*;

impl QingjianInputController {
    /// 按當前緩衝區重新查候選、更新 marked text，回到第一頁並重畫候選視窗。
    pub(super) fn refresh(&self, client: TextClient<'_>) {
        // 本地整句模型要看游標前文：一段組句只在第一鍵讀一次（組句中它不變；應用偶爾不回話也不至於讓前文來回換），
        // 讀應用文本要等應用回話，放在借 Host 之外（見 request_prediction）
        // 模型還在後臺載入也讀：接上時會補這一輪的重排，前文得先備好
        let wants_context = host::with(|h| {
            h.attach_loaded_model();
            (h.engine.has_sentence_scorer() || h.model_loading())
                && h.engine.composition().text().chars().count() == 1
        })
        .unwrap_or(false);
        let before = if wants_context && !secure_input::enabled() {
            Some(
                client
                    .surrounding_text(RESCORE_LOOKBACK, 0)
                    .map(|text| text.before),
            )
        } else {
            None
        };
        let Some((marked, cursor, inline)) = host::with(|h| {
            if let Some(before) = before {
                h.engine.set_rescoring_context(before);
            }
            // 查詢失敗（整段切不動）時退回顯示原始字母
            let mut marked = h.engine.composition().text().to_owned();
            let mut cursor = h.engine.composition().cursor();
            let mut preedit = Preedit::plain(&marked, cursor);
            let candidates = h
                .engine
                .query()
                .map(|mut query| {
                    h.engine.annotate(&mut query.candidates);
                    marked = query.marked_text();
                    cursor = query.marked_cursor();
                    preedit =
                        Preedit::from_marked(&query.marked_segments(), query.segments_cursor());
                    query.candidates.items
                })
                .unwrap_or_default();
            h.reset_session(preedit, candidates);
            h.schedule_rescoring();
            (marked, cursor, h.preedit_mode.inline())
        }) else {
            return;
        };
        // 配置成只在候選視窗顯示拼音時，應用裡不放 marked text（游標位置仍按插入點取）
        if inline {
            client.set_marked_text(&marked, cursor);
        } else {
            client.set_marked_text("", 0);
        }
        // 先發聯想再畫：發出去就留好雲端槽位，畫出來的第一幀本地候選就已經在最終位置
        if !marked.is_empty() {
            let candidates = host::with(|h| h.session.layout.local().to_vec()).unwrap_or_default();
            self.request_prediction(client, &candidates);
        }
        self.render(client);
    }

    /// 緩衝區裡只有一個 `?` 而使用者按了別的鍵：把它還原成問號上屏（中文遵循標點設定、英文半形）、清空緩衝區。
    /// 返回是否發生了還原。
    pub(super) fn restore_bare_question(&self, client: TextClient<'_>) -> bool {
        let english = modifiers::caps_lock_on();
        let restored = host::with(|h| {
            let mark = h.engine.restore_bare_question(english)?;
            h.cancel_prediction();
            Some(mark)
        })
        .flatten();
        let Some(mark) = restored else {
            return false;
        };
        client.insert_text(&mark);
        self.refresh(client);
        true
    }

    /// 記下游標位置並按會話狀態重畫候選視窗。
    pub(super) fn render(&self, client: TextClient<'_>) {
        let anchor = client.caret_rect();
        host::with(|h| {
            h.anchor = anchor;
            h.render();
        });
    }

    /// 發一次聯想請求。Secure Input 裡絕不發；沒接聯想器時是空操作。
    ///
    /// 讀上下文要等應用回話，這段時間 IMK 可能把 `deactivateServer:` 之類的回撥插進來，
    /// 所以分兩次借 Host：先拿策略、放開借用去讀、再借回來發請求。
    pub(super) fn request_prediction(&self, client: TextClient<'_>, candidates: &[Candidate]) {
        let policy = host::with(|h| {
            if !h.engine.prediction_enabled() {
                return None;
            }
            if secure_input::enabled() {
                tracing::debug!("Secure Input 中，不联想");
                h.cancel_prediction();
                return None;
            }
            Some(h.engine.prediction_policy())
        })
        .flatten();
        let Some(policy) = policy else {
            return;
        };
        let surrounding = client.surrounding_text(policy.before, policy.after);
        host::with(|h| {
            tracing::debug!(
                has_context = surrounding.is_some(),
                pinyin = h.engine.composition().scope(),
                "联想请求"
            );
            match h.engine.request_prediction(surrounding, candidates) {
                Some(_) => h.await_prediction(),
                None => h.cancel_prediction(),
            }
        });
    }

    /// 接受組句中的整句補全：作用域內的拼音作廢，句子上屏。沒有補全返回 false。
    pub(super) fn accept_sentence(&self, client: TextClient<'_>) -> bool {
        let Some(text) = host::with(|h| h.sentence.take()).flatten() else {
            return false;
        };
        host::with(|h| h.engine.accept_prediction(&text));
        tracing::debug!(%text, "接受整句补全");
        client.insert_text(&text);
        self.refresh(client);
        true
    }

    /// 上下鍵。高亮逐個移動，越過頁邊自動翻頁；橫排矩陣開著（`[general] horizontal_grid`）時改為：
    /// 單行先展開成矩陣，在矩陣裡換行（視口跟著滾）。
    pub(super) fn move_highlight(&self, delta: isize, client: TextClient<'_>) {
        let changed = host::with(|h| {
            if h.grid_keys() {
                h.session.move_rows(delta)
            } else {
                h.session.move_highlight(delta)
            }
        })
        .unwrap_or(false);
        if changed {
            self.render(client);
        }
    }

    /// 左右鍵。橫排矩陣開著而且有候選時歸候選：單行裡逐個移動高亮（到頁邊自動翻頁，不展開），展開後按閱讀順序跨行移動——
    /// 上下鍵被矩陣佔了，單行裡只剩左右鍵能挪高亮。返回是不是歸了候選（不是的話呼叫方照舊移動拼音游標）。
    pub(super) fn move_cells(&self, delta: isize, client: TextClient<'_>) -> bool {
        let handled = host::with(|h| {
            if !h.grid_keys() || h.session.layout.is_empty() {
                return None;
            }
            Some(if h.session.grid.is_some() {
                h.session.move_cells(delta)
            } else {
                h.session.move_highlight(delta)
            })
        })
        .flatten();
        if handled == Some(true) {
            self.render(client);
        }
        handled.is_some()
    }

    /// 每個鍵都問一次應用標識（activateServer 時程序剛拉起可能還拿不到），變了才告訴 Engine。
    pub(super) fn note_application(&self, client: &TextClient<'_>) {
        let app = client.bundle_identifier();
        host::with(|h| {
            if h.engine.application() != app.as_deref() {
                h.engine.set_application(app);
            }
        });
    }

    /// 翻頁，高亮落到新頁第一項。已在首頁 / 末頁時不動。
    pub(super) fn turn_page(&self, delta: isize, client: TextClient<'_>) -> bool {
        let turned = host::with(|h| {
            // 矩陣展開著：翻頁鍵一次翻一屏
            let turned = if h.session.grid.is_some() {
                h.session.move_screens(delta)
            } else {
                h.session.turn_page(delta)
            };
            if turned {
                h.engine.note_page_turn();
            }
            turned
        })
        .unwrap_or(false);
        if turned {
            self.render(client);
        }
        true
    }
}
