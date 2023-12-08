use ruff_formatter::FormatRuleWithOptions;
use ruff_python_ast::AnyNodeRef;
use ruff_python_ast::ExprBytesLiteral;

use crate::comments::SourceComment;
use crate::expression::expr_string_literal::is_multiline_string;
use crate::expression::parentheses::{
    in_parentheses_only_group, NeedsParentheses, OptionalParentheses,
};
use crate::prelude::*;
use crate::string::{AnyString, FormatStringContinuation, StringLayout};

#[derive(Default)]
pub struct FormatExprBytesLiteral {
    layout: StringLayout,
}

impl FormatRuleWithOptions<ExprBytesLiteral, PyFormatContext<'_>> for FormatExprBytesLiteral {
    type Options = StringLayout;

    fn with_options(mut self, options: Self::Options) -> Self {
        self.layout = options;
        self
    }
}

impl FormatNodeRule<ExprBytesLiteral> for FormatExprBytesLiteral {
    fn fmt_fields(&self, item: &ExprBytesLiteral, f: &mut PyFormatter) -> FormatResult<()> {
        let ExprBytesLiteral { value, .. } = item;

        match self.layout {
            StringLayout::DocString => unreachable!("`ExprBytesLiteral` cannot be a docstring"),
            StringLayout::Default => match value.as_slice() {
                [] => unreachable!("Empty `ExprBytesLiteral`"),
                [bytes_literal] => bytes_literal.format().fmt(f),
                _ => in_parentheses_only_group(&FormatStringContinuation::new(
                    &AnyString::BytesLiteral(item),
                ))
                .fmt(f),
            },
            StringLayout::ImplicitConcatenatedStringInBinaryLike => {
                FormatStringContinuation::new(&AnyString::BytesLiteral(item)).fmt(f)
            }
        }
    }

    fn fmt_dangling_comments(
        &self,
        _dangling_comments: &[SourceComment],
        _f: &mut PyFormatter,
    ) -> FormatResult<()> {
        // Handled as part of `fmt_fields`
        Ok(())
    }
}

impl NeedsParentheses for ExprBytesLiteral {
    fn needs_parentheses(
        &self,
        _parent: AnyNodeRef,
        context: &PyFormatContext,
    ) -> OptionalParentheses {
        if self.value.is_implicit_concatenated() {
            OptionalParentheses::Multiline
        } else if is_multiline_string(self.into(), context.source()) {
            OptionalParentheses::Never
        } else {
            OptionalParentheses::BestFit
        }
    }
}
