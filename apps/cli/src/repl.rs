//! 互動模式：拼音 → 候選；數字 → 上屏；`:raw` 把上一次輸入原樣上屏（相當於回車）；`:q` 退出。

use std::io::{self, BufRead, Write};

use qingjian_core::{Engine, Query};

use crate::display;
use crate::error::CliError;

pub fn run(engine: &mut Engine, limit: usize) -> Result<(), CliError> {
    eprintln!(
        "输入拼音查询候选，输入序号上屏，:raw 原样上屏上一次输入（回车），:del N 删掉第 N 个候选，:q 退出。"
    );
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut last: Option<Query> = None;
    loop {
        print!("> ");
        stdout.flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == ":q" || line == ":quit" {
            break;
        }
        if let Ok(index) = line.parse::<usize>() {
            commit(engine, last.take(), index);
            continue;
        }
        if line == ":raw" {
            last = None;
            println!("原样上屏: {}", engine.take_raw());
            continue;
        }
        if let Some(index) = line
            .strip_prefix(":del ")
            .and_then(|rest| rest.trim().parse::<usize>().ok())
        {
            forget(engine, last.as_ref(), index);
            continue;
        }
        last = display::show(engine, line, limit);
    }
    Ok(())
}

/// 刪掉上一次查詢的第 `index` 個（從 1 數）候選：使用者詞整個刪、詞庫詞清學習。
fn forget(engine: &mut Engine, last: Option<&Query>, index: usize) {
    match last.and_then(|q| q.candidates.items.get(index.wrapping_sub(1))) {
        Some(candidate) => {
            let forgotten = engine.forget(candidate);
            println!("删除 {}: {forgotten:?}", candidate.text);
        }
        None => println!("没有第 {index} 个候选"),
    }
}

/// 把上一次查詢的第 `index` 個（從 1 數）候選上屏。
fn commit(engine: &mut Engine, last: Option<Query>, index: usize) {
    match last.and_then(|q| q.candidates.items.into_iter().nth(index.wrapping_sub(1))) {
        Some(candidate) => {
            println!("上屏: {}", engine.commit(&candidate));
        }
        None => println!("没有第 {index} 个候选"),
    }
}
