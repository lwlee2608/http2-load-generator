pub mod assert;
pub mod context;
pub mod define;
pub mod function;
pub mod global;
pub mod parser;
pub mod value;
pub mod variable;

pub use crate::script::context::ScriptContext;
// pub use crate::script::function::Function;
pub use crate::script::global::Global;
pub use crate::script::parser::Scripts;
pub use crate::script::value::Value;
pub use crate::script::variable::Variable;

use crate::error::Error;

pub trait Script {
    fn execute(&self, ctx: &mut ScriptContext) -> Result<(), Error>;
}
