//! gal-script —— 剧本层：DSL 词法/指令/表达式/变量/解释器 + 演出层状态（Stage）

pub mod command;
pub mod expr;
pub mod interp;
pub mod lexer;
pub mod stage;
pub mod vars;

pub use interp::{BacklogItem, Interp, RunState, SysEvent};
pub use stage::{Stage, StageSnap};
pub use vars::{Value, Vars};
