mod builtin;
mod descriptor;
mod execute;
mod id;
mod invocation;
mod registry;
mod validation;

pub use builtin::RustBuiltinCommand;
#[allow(unused_imports)]
pub use descriptor::{CommandArgumentDescriptor, CommandDescriptor, CommandHandler, CommandSource};
#[allow(unused_imports)]
pub use execute::apply_builtin_command;
pub use id::CommandId;
pub use invocation::{CommandInvocation, editor_command_to_invocation};
pub use registry::CommandRegistry;

#[cfg(test)]
mod tests;
