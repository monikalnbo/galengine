//! command 单元测试

#[cfg(test)]
mod tests {
    use super::super::types::Command;
    use crate::lexer::LineKind;

    fn cmd(name: &str, rest: &str) -> Result<Command, String> {
        Command::parse(&LineKind::Command { name: name.into(), rest: rest.into() }, "t.ks:1")
    }

    #[test]
    fn 基本指令解析() {
        assert!(
            matches!(cmd("bg", "a 800").unwrap(), Command::Bg { storage, fade_ms: 800 } if storage == "a")
        );
        assert!(
            matches!(cmd("char", "0 hide").unwrap(), Command::Char { layer: 0, ref storage, x: None, y: None } if storage == "hide")
        );
        assert!(cmd("char", "3 x").is_err());
        assert!(
            matches!(cmd("flag", "a +2").unwrap(), Command::Flag { var, op: '+', val: 2 } if var == "a")
        );
        assert!(matches!(cmd("flag", "a -1").unwrap(), Command::Flag { op: '-', val: 1, .. }));
        assert!(
            matches!(cmd("reach", "hide").unwrap(), Command::Reach { ref storage, .. } if storage == "hide")
        );
        assert!(matches!(
            cmd("window_fx", "title restore").unwrap(),
            Command::WindowFxTitleRestore
        ));
        assert!(matches!(cmd("shutdown", "").unwrap(), Command::Shutdown(300)));
    }

    #[test]
    fn set表达式与desktop内容含空格() {
        match cmd("set", "a = 1 + 2").unwrap() {
            Command::Set { var, expr } => {
                assert_eq!(var, "a");
                assert_eq!(expr, "1 + 2");
            }
            _ => panic!(),
        }
        match cmd("desktop_write", "letter.txt|还记得吗。{hero}").unwrap() {
            Command::DesktopWrite { file, content } => {
                assert_eq!(file, "letter.txt");
                assert_eq!(content, "还记得吗。{hero}");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn 未知指令中文报错() {
        assert_eq!(cmd("foo", "").unwrap_err(), "t.ks:1：未知指令「foo」");
    }
}
