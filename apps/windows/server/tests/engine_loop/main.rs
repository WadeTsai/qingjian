//! 不經傳輸層，直接把協議訊息餵給 Router 的閉環測試；用樣例詞庫，跨平臺可跑。 按主題分模組，公共零件在 `support`。

mod aux_code;
mod click;
mod composing;
mod english;
mod modes;
mod profile;
mod rescoring;
mod shortcuts;
mod status;
mod support;
mod zhuyin;
