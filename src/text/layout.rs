//! 中文断行与分页：按字宽断行 + 行首禁则（标点不落行首）+ ASCII 词不拆。

use sdl2::ttf::Font;

/// 行首禁则字符（不可出现在行首）
const NO_LINE_START: &str = "，。、！？；：…—～％‰”』」）〕》〉·.,!?;:)]}%'>";

/// 断行：逐字累加宽度，超宽即断；禁则字符/ASCII 连串时把行尾字符一并下移。
pub fn wrap(text: &str, font: &Font, max_width: u32) -> Vec<String> {
    let char_w = |ch: char| font.size_of_char(ch).map(|(w, _)| w).unwrap_or(0);
    let mut lines: Vec<String> = Vec::new();
    let mut cur: Vec<char> = Vec::new();
    let mut cur_w = 0u32;

    for ch in text.chars() {
        let w = char_w(ch);
        if cur_w + w > max_width && !cur.is_empty() {
            let mut cut = cur.len();
            let glue = NO_LINE_START.contains(ch)
                || (ch.is_ascii_alphanumeric()
                    && cur.last().is_some_and(|p| p.is_ascii_alphanumeric()));
            if glue && cut > 1 {
                cut -= 1; // 行尾字符随禁则头/英文串下移
            }
            let mut rest = cur.split_off(cut);
            rest.push(ch);
            cur_w = rest.iter().map(|&c| char_w(c)).sum();
            let line: String = cur.drain(..).collect();
            lines.push(line);
            cur = rest;
        } else {
            cur.push(ch);
            cur_w += w;
        }
    }
    if !cur.is_empty() {
        lines.push(cur.into_iter().collect());
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// 分页：per_page 行一页
pub fn paginate(lines: Vec<String>, per_page: usize) -> Vec<Vec<String>> {
    lines
        .into_iter()
        .fold(Vec::<Vec<String>>::new(), |mut acc, line| {
            match acc.last_mut() {
                Some(page) if page.len() < per_page => page.push(line),
                _ => acc.push(vec![line]),
            }
            acc
        })
}
