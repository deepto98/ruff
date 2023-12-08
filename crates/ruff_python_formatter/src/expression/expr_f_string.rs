use memchr::memchr2;

use ruff_formatter::FormatResult;
use ruff_formatter::FormatRuleWithOptions;
use ruff_python_ast::AnyNodeRef;
use ruff_python_ast::ExprFString;

use crate::comments::SourceComment;
use crate::expression::parentheses::{
    in_parentheses_only_group, NeedsParentheses, OptionalParentheses,
};
use crate::prelude::*;
use crate::string::{AnyString, FormatStringContinuation, StringLayout};

#[derive(Default)]
pub struct FormatExprFString {
    layout: StringLayout,
}

impl FormatRuleWithOptions<ExprFString, PyFormatContext<'_>> for FormatExprFString {
    type Options = StringLayout;

    fn with_options(mut self, options: Self::Options) -> Self {
        self.layout = options;
        self
    }
}

impl FormatNodeRule<ExprFString> for FormatExprFString {
    fn fmt_fields(&self, item: &ExprFString, f: &mut PyFormatter) -> FormatResult<()> {
        let ExprFString { value, .. } = item;

        match self.layout {
            StringLayout::DocString => unreachable!("`ExprFString` cannot be a docstring"),
            StringLayout::Default => match value.as_slice() {
                [] => unreachable!("Empty `ExprFString`"),
                [f_string_part] => f_string_part.format().fmt(f),
                _ => in_parentheses_only_group(&FormatStringContinuation::new(
                    &AnyString::FString(item),
                ))
                .fmt(f),
            },
            StringLayout::ImplicitConcatenatedStringInBinaryLike => {
                FormatStringContinuation::new(&AnyString::FString(item)).fmt(f)
            }
        }
    }

    fn fmt_dangling_comments(
        &self,
        _dangling_node_comments: &[SourceComment],
        _f: &mut PyFormatter,
    ) -> FormatResult<()> {
        // Handled as part of `fmt_fields`
        Ok(())
    }
}

impl NeedsParentheses for ExprFString {
    fn needs_parentheses(
        &self,
        _parent: AnyNodeRef,
        context: &PyFormatContext,
    ) -> OptionalParentheses {
        if self.value.is_implicit_concatenated() {
            OptionalParentheses::Multiline
        } else if memchr2(b'\n', b'\r', context.source()[self.range].as_bytes()).is_none() {
            OptionalParentheses::BestFit
        } else {
            OptionalParentheses::Never
        }
    }
}
