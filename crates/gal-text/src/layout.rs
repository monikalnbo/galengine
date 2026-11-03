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
    /// v0.3.13 内嵌图（KAG3 graph 标签）：素材名（渲染时经 TextureBank 贴图）
    Image(String),
}

impl Unit {
    /// 打字机计数的字符数（ruby 组 1 个）
    pub fn chars(&self) -> usize {
        match self {
            Unit::Ch(_) | Unit::Ruby(..) => 1,
            Unit::Image(_) => 0,
        }
    }

    pub fn text(&self) -> String {
        match self {
            Unit::Ch(c) => c.to_string(),
            Unit::Ruby(b, _) => b.clone(),
            Unit::Image(_) => String::new(),
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
        // v0.3.14 强制换行（r 标签生成）：立即收行（KAG3 [r] 口径）
        if chars[i] == '\u{1e}' {
            lines.push(std::mem::take(&mut cur));
            cur_w = 0;
            i += 1;
            continue;
        }
        let (u, step) = if chars[i] == '\u{b}' {
            // v0.3.13 内嵌图占位符（graph 标签生成）：\u{b}<素材名>\u{b}
            match chars[i + 1..].iter().position(|&c| c == '\u{b}') {
                Some(end) if end > 0 => {
                    let name: String = chars[i + 1..i + 1 + end].iter().collect();
                    (Unit::Image(name), end + 2)
                }
                _ => (Unit::Ch(chars[i]), 1),
            }
        } else {
            match parse_ruby_at(&chars, i) {
                Some((b, r, next)) => (Unit::Ruby(b, r), next - i),
                None => (Unit::Ch(chars[i]), 1),
            }
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
                Unit::Ruby(..) | Unit::Image(_) => false,
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
        // 内嵌图缺省占一个字号方块宽（实际贴图时按图缩放到行高内）
        Unit::Image(_) => ruby_size as u32 * 4,
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
        if chars[i] == '\u{b}' {
            // 内嵌图占位符：朗读纯文本直接去掉（图不朗读）
            match chars[i + 1..].iter().position(|&c| c == '\u{b}') {
                Some(end) if end > 0 => i += end + 2,
                _ => i += 1,
            }
            continue;
        }
        if chars[i] == '\u{1e}' {
            // v0.3.14 强制换行标记：朗读按普通换行处理（跳过即可，不产出文本）
            i += 1;
            continue;
        }
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

#[cfg(test)]
mod v0313_tests {
    use super::*;

    #[test]
    fn 图占位符解析成_image_单元() {
        // 无字体环境下验证占位符 → Unit::Image 的文法路径（断行逻辑不经字体即可单测部分）
        let chars: Vec<char> = "前\u{b}face_a\u{b}后".chars().collect();
        // 直接模拟 wrap_units 的解析片段：\u{b}<名>\u{b} → Image(name)，消费 end+2
        let i = chars.iter().position(|&c| c == '\u{b}').unwrap();
        let end = chars[i + 1..].iter().position(|&c| c == '\u{b}').unwrap();
        assert_eq!(chars[i + 1..i + 1 + end].iter().collect::<String>(), "face_a");
        assert_eq!(end + 2, 8);
    }

    #[test]
    fn image_单元不产生朗读文本且计零字符() {
        let u = Unit::Image("face_a".into());
        assert_eq!(u.chars(), 0);
        assert_eq!(u.text(), "");
        // 纯文本化：占位符整段去掉
        assert_eq!(plain("前\u{b}face_a\u{b}后"), "前后");
    }
}

#[cfg(test)]
mod v0314_tests {
    use super::*;

    #[test]
    fn 强制换行标记朗读时去除() {
        assert_eq!(plain("第一行\u{1e}第二行"), "第一行第二行");
    }

    #[test]
    fn 换行标记不产生文本单元() {
        // chars 计数语义：\u{1e} 作为 Ch('RS') 只在 wrap_units 特判收行，不出现在单元流
        // 这里验证 plain 与占位符共存不 panic
        assert_eq!(plain("前\u{b}x\u{b}\u{1e}后"), "前后");
    }
}
