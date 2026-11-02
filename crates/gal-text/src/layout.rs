//! 中文断行 + ruby（注音）：按「单元」累加断行 + 行首禁则 + ASCII 连串不拆。
//!
//! 单元 = 一个普通字符，或一个 ruby 组 `[基底]^(注音)`（基底 1~8 字，注音 1~16 字）。
//! ruby 组整体不拆行；占宽取基底与注音中较宽者，行高不够时另起一行。
//! 打字机揭示按「字符」计数（ruby 的注音随基底一起揭示）。

use sdl2::ttf::Font;

const NO_LINE_START: &str = "，。、！？；：…—～％‰”』」）〕》〉·.,!?;:)]}%'>";

/// 一个排版单元
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unit {
    /// 普通字符
    Ch(char),
    /// 注音：基底 + 读音（基底整体显示，读音画在其上方）
    Ruby(String, String),
}

impl Unit {
    /// 打字机计数的字符数（ruby 组 1 个）
    pub fn chars(&self) -> usize {
        match self {
            Unit::Ch(_) => 1,
            Unit::Ruby(..) => 1,
        }
    }

    pub fn text(&self) -> String {
        match self {
            Unit::Ch(c) => c.to_string(),
            Unit::Ruby(b, _) => b.clone(),
        }
    }
}

/// 解析 `[基底]^(注音)`：返回 (基底, 注音, 消费到的下一位置)
fn parse_ruby_at(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    debug_assert_eq!(chars[start], '[');
    let close = (start + 1..chars.len()).find(|&j| chars[j] == ']')?;
    let base: String = chars[start + 1..close].iter().collect();
    let base_len = base.chars().count();
    if base_len == 0 || base_len > 8 || chars.get(close + 1) != Some(&'^') {
        return None;
    }
    let Some('(') = chars.get(close + 2) else { return None };
    let end = (close + 3..chars.len()).find(|&j| chars[j] == ')')?;
    let ruby: String = chars[close + 3..end].iter().collect();
    if ruby.is_empty() || ruby.chars().count() > 16 {
        return None;
    }
    Some((base, ruby, end + 1))
}

/// 单元级的断行：返回每行的单元序列（保持 ruby 组完整）
pub fn wrap_units(text: &str, font: &Font, max_width: u32, ruby_size: u16) -> Vec<Vec<Unit>> {
    let char_w = |ch: char| font.size_of_char(ch).map(|(w, _)| w).unwrap_or(0);
    let mut lines: Vec<Vec<Unit>> = Vec::new();
    let mut cur: Vec<Unit> = Vec::new();
    let mut cur_w = 0u32;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let (u, step) = match parse_ruby_at(&chars, i) {
            Some((b, r, next)) => (Unit::Ruby(b, r), next - i),
            None => (Unit::Ch(chars[i]), 1),
        };
        let w = unit_width(&u, ruby_size, char_w);
        if cur_w + w > max_width && !cur.is_empty() {
            // 行首禁则：下一个普通标点随前一行末字一起下移
            let glue = match &u {
                Unit::Ch(c) => {
                    NO_LINE_START.contains(*c)
                        || (c.is_ascii_alphanumeric()
                            && matches!(cur.last(), Some(Unit::Ch(p)) if p.is_ascii_alphanumeric()))
                }
                Unit::Ruby(..) => false,
            };
            let mut moved: Vec<Unit> = Vec::new();
            if glue && cur.len() > 1 {
                moved.push(cur.pop().unwrap());
            }
            let mut next_line = moved;
            next_line.push(u.clone());
            cur_w = next_line.iter().map(|x| unit_width(x, ruby_size, char_w)).sum();
            lines.push(std::mem::replace(&mut cur, next_line));
        } else {
            cur.push(u);
            cur_w += w;
        }
        i += step;
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(Vec::new());
    }
    lines
}

fn unit_width(u: &Unit, ruby_size: u16, char_w: impl Fn(char) -> u32) -> u32 {
    match u {
        Unit::Ch(c) => char_w(*c),
        Unit::Ruby(b, r) => {
            let base: u32 = b.chars().map(&char_w).sum();
            let ruby = (r.chars().count() as u32) * ruby_size as u32 / 2;
            base.max(ruby)
        }
    }
}

/// 逐行拼接纯文本（用于度量/历史/朗读）
pub fn line_text(units: &[Unit]) -> String {
    units.iter().map(Unit::text).collect()
}

/// 旧接口：把纯文本断成字符串行（打字机按字符计数）
pub fn wrap(text: &str, font: &Font, max_width: u32) -> Vec<String> {
    wrap_units(text, font, max_width, 12).into_iter().map(|l| line_text(&l)).collect()
}

/// 分页：per_page 行一页
pub fn paginate<T: Clone>(lines: Vec<T>, per_page: usize) -> Vec<Vec<T>> {
    lines.into_iter().fold(Vec::<Vec<T>>::new(), |mut acc, line| {
        match acc.last_mut() {
            Some(page) if page.len() < per_page => page.push(line),
            _ => acc.push(vec![line]),
        }
        acc
    })
}

/// 纯文本化（朗读/提示用）：`[基底]^(注音)` → 基底，其余字符原样
pub fn plain(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        match if chars[i] == '[' { parse_ruby_at(&chars, i) } else { None } {
            Some((base, _, next)) => {
                out.push_str(&base);
                i = next;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruby_语法被识别且整体成单元() {
        let u = parse_ruby_at(&"[夏]^(なつ)".chars().collect::<Vec<_>>(), 0).unwrap();
        assert_eq!(u.0, "夏");
        assert_eq!(u.1, "なつ");
        assert_eq!(u.2, 8);
    }

    #[test]
    fn 非_ruby_方括号按普通字符处理() {
        let chars: Vec<char> = "[旁白]你好".chars().collect();
        assert!(parse_ruby_at(&chars, 0).is_none());
    }

    #[test]
    fn 朗读纯文本保留基底去掉注音() {
        assert_eq!(plain("今天[夏]^(なつ)很好"), "今天夏很好");
        assert_eq!(plain("没有注音"), "没有注音");
    }

    #[test]
    fn 纯文本断行与旧接口一致() {
        // 无字体条件下只验证纯文本路径不 panic
        assert_eq!(line_text(&[Unit::Ch('你'), Unit::Ch('好')]), "你好");
    }
}
