//! 剧本语法体系子模块（docs/20 第四节，26 条指令）
pub mod command;
pub mod expr;
pub mod interp;
pub mod lexer;
pub mod vars;

#[cfg(test)]
mod core_tests;
#[cfg(test)]
mod expr_tests;
