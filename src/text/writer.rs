//! 打字机状态机：逐字显示；点击三段语义（补全 → 翻页 → 行完）。

use sdl2::ttf::Font;

use crate::text::layout;

#[derive(Debug, PartialEq)]
pub enum ClickResult {
    RevealAll,
    NextPage,
    LineFinished,
}

pub struct Typewriter {
    pages: Vec<Vec<String>>,
    page: usize,
    revealed: usize,
    acc_ms: f32,
    pub interval_ms: f32,
}

impl Typewriter {
    pub fn new(interval_ms: f32) -> Self {
        Self { pages: vec![vec![String::new()]], page: 0, revealed: 0, acc_ms: 0.0, interval_ms }
    }

    pub fn set_text(&mut self, text: &str, font: &Font, max_width: u32, lines_per_page: usize) {
        let lines = layout::wrap(text, font, max_width);
        self.pages = layout::paginate(lines, lines_per_page.max(1));
        self.page = 0;
        self.revealed = 0;
        self.acc_ms = 0.0;
    }

    pub fn clear(&mut self) {
        self.pages = vec![vec![String::new()]];
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

    /// 当前页总字符数（换行计 1）
    fn page_chars(&self) -> usize {
        self.pages[self.page].iter().map(|l| l.chars().count()).sum::<usize>()
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

    pub fn current_page(&self) -> &[String] {
        &self.pages[self.page]
    }

    /// (当前显示行号, 该行已显字符数)
    pub fn reveal_pos(&self) -> (usize, usize) {
        let mut left = self.revealed;
        for (i, line) in self.current_page().iter().enumerate() {
            let n = line.chars().count();
            if left <= n {
                return (i, left);
            }
            left -= n + 1;
        }
        (self.current_page().len(), 0)
    }
}
