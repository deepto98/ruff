use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::call_path::collect_call_path;
use ruff_python_ast::{self as ast, Expr};
use ruff_python_semantic::{BindingKind, ScopeKind};
use ruff_source_file::OneIndexed;
use ruff_text_size::Ranged;

use crate::checkers::ast::Checker;
/// ## What it does
/// Checks for the presence of unused `self` parameter in methods definitions.
///
/// ## Why is this bad?
/// Unused `self` parameters are usually a sign of a method that could be
/// replaced by a function, class method, or static method.
///
/// ## Example
/// ```python
/// class Person:
///     def greeting(self):
///         print("Greetings friend!")
/// ```
///
/// Use instead:
/// ```python
/// def greeting():
///     print("Greetings friend!")
/// ```
#[violation]
pub struct AccessMemberBeforeDefinition {
    member: String,
    // line: OneIndexed,
} 

impl Violation for AccessMemberBeforeDefinition {
    #[derive_message_formats]
    fn message(&self) -> String {
        let AccessMemberBeforeDefinition { member } = self;
        format!("Access to member `{member}` before its definition line (todo: add line`")
    }
}

pub(crate) fn access_member_before_definition(checker: &mut Checker, expr: &Expr) {
    let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = expr else {
        return;
    };
    
    checker.diagnostics.push(Diagnostic::new(
        AccessMemberBeforeDefinition {
            member: attr.to_string(),
        },
        expr.range(),
    ));
}
