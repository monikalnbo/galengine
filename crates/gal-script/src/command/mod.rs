//! 全 26 条指令的枚举与解析（一处看全所有语法）。ctx 为「文件:行号」中文错误定位。

mod parse;
mod types;

#[cfg(test)]
mod tests;

pub use types::Command;
