//! 解释器历史与文本处理：姓名宏替换、已读状态持久化、对白回卷历史、CG 解锁记录。

use std::collections::HashSet;

use super::types::{dbg, BacklogItem};
use super::Interp;
use crate::vars::Value;
use gal_config::style::DialogStyle;
use gal_text::font::FontBook;

impl Interp {
    /// app 每帧：待显台词交 FontBook 断行；做 {hero}/{you} 替换
    pub fn apply_pending(&mut self, fonts: &mut FontBook, style: &DialogStyle) {
        if let Some(text) = self.pending_text.take() {
            let text = self.replace_names(&text);
            self.last_text = text.chars().take(18).collect();
            // 推入回卷历史（最多保留 100 条）
            if self.backlog.len() >= 100 {
                self.backlog.remove(0);
            }
            self.backlog.push(BacklogItem { name: self.cur_name.clone(), text: text.clone() });
            self.mark_cur_line_read();
            if dbg() {
                eprintln!(
                    "[dbg] @{}:{} {}",
                    self.file,
                    self.stop_line.map(|l| l + 1).unwrap_or(0),
                    text.chars().take(12).collect::<String>()
                );
            }
            if let Ok(font) = fonts.font(style.font_size) {
                let max_w = (style.box_rect[2] - 96) as u32;
                self.tw.set_text(&text, font, max_w, style.lines_per_page);
            }
        }
    }

    /// 标记当前等待行已读（sf.readLines 持久化）
    pub fn mark_cur_line_read(&mut self) {
        if let Some(line) = self.stop_line {
            let key = format!("{}:{}", self.file, line);
            let mut read_set = self.get_read_lines();
            if read_set.insert(key) {
                let joined = read_set.into_iter().collect::<Vec<_>>().join(",");
                self.vars.sf.insert("readLines".into(), Value::Str(joined));
            }
        }
    }

    /// 当前等待行是否已被阅读过
    pub fn is_cur_line_read(&self) -> bool {
        if let Some(line) = self.stop_line {
            let key = format!("{}:{}", self.file, line);
            self.get_read_lines().contains(&key)
        } else {
            false
        }
    }

    pub(crate) fn get_read_lines(&self) -> HashSet<String> {
        self.vars
            .sf
            .get("readLines")
            .map(|v| {
                v.as_str().split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect()
            })
            .unwrap_or_default()
    }

    /// {hero}->f.heroName、{you}->sf.playerName（空回退配置默认）
    pub fn replace_names(&self, text: &str) -> String {
        let hero = match self.vars.get("f.heroName") {
            Some(Value::Str(s)) if !s.is_empty() => s.clone(),
            _ => self.hero_default.clone(),
        };
        let you = match self.vars.get("sf.playerName") {
            Some(Value::Str(s)) if !s.is_empty() => s.clone(),
            _ => self.you_default.clone(),
        };
        text.replace("{hero}", &hero).replace("{you}", &you)
    }

    /// cg 指令解锁记录（sf.cgs 累加表，鉴赏用）
    pub(crate) fn unlock_cg(&mut self, storage: &str) {
        let list = self.vars.sf.entry("cgs".into()).or_insert_with(|| Value::Str(String::new()));
        if let Value::Str(s) = list {
            if !s.split(',').any(|x| x == storage) {
                if !s.is_empty() {
                    s.push(',');
                }
                s.push_str(storage);
            }
        }
    }
}
