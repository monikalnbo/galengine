//! 打字机状态机：逐字显示；点击三段语义（补全 → 翻页 → 行完）。
//! 文本按「排版单元」保存（普通字符 / ruby 组），揭示计数按字符，ruby 随基底一起出现。

use sdl2::ttf::Font;

use crate::layout::{self, Unit};

#[derive(Debug, PartialEq)]
pub enum ClickResult {
    RevealAll,
    NextPage,
    LineFinished,
}

pub struct Typewriter {
    pages: Vec<Vec<Vec<Unit>>>,
    page: usize,
    revealed: usize,
    acc_ms: f32,
    pub interval_ms: f32,
}

impl Typewriter {
    pub fn new(interval_ms: f32) -> Self {
        Self { pages: vec![vec![vec![]]], page: 0, revealed: 0, acc_ms: 0.0, interval_ms }
    }

    /// 按单元排版（ruby 尺寸参与占宽计算）
    pub fn set_units(
        &mut self,
        text: &str,
        font: &Font,
        max_width: u32,
        lines_per_page: usize,
        ruby_size: u16,
    ) {
        let lines = layout::wrap_units(text, font, max_width, ruby_size);
        if std::env::var("ES_DEBUG").is_ok() {
            let ruby = lines.iter().flatten().filter(|u| matches!(u, Unit::Ruby(..))).count();
            if ruby > 0 {
                eprintln!("[dbg ruby] 注音组 {ruby}（ruby_size {ruby_size}px）");
            }
        }
        self.pages = layout::paginate(lines, lines_per_page.max(1));
        self.page = 0;
        self.revealed = 0;
        self.acc_ms = 0.0;
    }

    /// 旧接口：ruby 尺寸给固定 14px（不影响无 ruby 文本的排版）
    pub fn set_text(&mut self, text: &str, font: &Font, max_width: u32, lines_per_page: usize) {
        self.set_units(text, font, max_width, lines_per_page, 14)
    }

    pub fn clear(&mut self) {
        self.pages = vec![vec![vec![]]];
        self.page = 0;
        self.revealed = 0;
        self.acc_ms = 0.0;
    }

    pub fn tick(&mut self, dt_ms: f32) {
        if self.page_full() {
            return;
        }
        self.acc_ms += dt_ms;
        while self.acc_ms >= self.interval_ms && !self.page_full() {
            self.revealed += 1;
            self.acc_ms -= self.interval_ms;
        }
        if self.page_full() {
            self.acc_ms = 0.0;
        }
    }

    /// 当前页总字符数（换行计 1；ruby 组计 1）
    fn page_chars(&self) -> usize {
        self.pages[self.page].iter().map(|l| line_chars(l)).sum::<usize>()
            + self.pages[self.page].len().saturating_sub(1)
    }

    pub fn page_full(&self) -> bool {
        self.revealed >= self.page_chars()
    }

    pub fn line_finished(&self) -> bool {
        self.page == self.pages.len() - 1 && self.page_full()
    }

    pub fn click(&mut self) -> ClickResult {
        if !self.page_full() {
            self.revealed = self.page_chars();
            self.acc_ms = 0.0;
            return ClickResult::RevealAll;
        }
        if self.page < self.pages.len() - 1 {
            self.page += 1;
            self.revealed = 0;
            self.acc_ms = 0.0;
            return ClickResult::NextPage;
        }
        ClickResult::LineFinished
    }

    /// 当前页的单元行（绘制用）
    pub fn current_units(&self) -> &[Vec<Unit>] {
        &self.pages[self.page]
    }

    /// 兼容旧调用：当前页的纯文本行
    pub fn current_page(&self) -> Vec<String> {
        self.pages[self.page].iter().map(|l| layout::line_text(l)).collect()
    }

    /// (当前显示行号, 该行已显字符数)
    pub fn reveal_pos(&self) -> (usize, usize) {
        let mut left = self.revealed;
        for (i, line) in self.pages[self.page].iter().enumerate() {
            let n = line_chars(line);
            if left <= n {
                return (i, left);
            }
            left -= n + 1;
        }
        (self.pages[self.page].len(), 0)
    }
}

fn line_chars(line: &[Unit]) -> usize {
    line.iter().map(Unit::chars).sum()
}
