use ruff_formatter::FormatRuleWithOptions;
use ruff_python_ast::AnyNodeRef;
use ruff_python_ast::ExprStringLiteral;
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::comments::SourceComment;
use crate::expression::parentheses::{
    in_parentheses_only_group, NeedsParentheses, OptionalParentheses,
};
use crate::prelude::*;
use crate::string::{
    AnyString, FormatStringContinuation, StringLayout, StringPrefix, StringQuotes,
};


#[derive(Default)]
pub struct FormatExprStringLiteral {
    layout: StringLayout,
}

impl FormatRuleWithOptions<ExprStringLiteral, PyFormatContext<'_>> for FormatExprStringLiteral {
    type Options = StringLayout;

    fn with_options(mut self, options: Self::Options) -> Self {
        self.layout = options;
        self
    }
}

impl FormatNodeRule<ExprStringLiteral> for FormatExprStringLiteral {
    fn fmt_fields(&self, item: &ExprStringLiteral, f: &mut PyFormatter) -> FormatResult<()> {
        let ExprStringLiteral { value, .. } = item;

        let _parent_docstring_quote_style = f.context().docstring();
        let _locator = f.context().locator();

        match self.layout {
            StringLayout::Default | StringLayout::DocString => match value.as_slice() {
                [] => unreachable!("Empty `ExprStringLiteral`"),
                [string_literal] => string_literal.format().with_options(self.layout).fmt(f),
                _ => in_parentheses_only_group(&FormatStringContinuation::new(
                    &AnyString::StringLiteral(item),
                ))
                .fmt(f),
            },
            StringLayout::ImplicitConcatenatedStringInBinaryLike => {
                FormatStringContinuation::new(&AnyString::StringLiteral(item)).fmt(f)
            }
        }
    }

    fn fmt_dangling_comments(
        &self,
        _dangling_comments: &[SourceComment],
        _f: &mut PyFormatter,
    ) -> FormatResult<()> {
        Ok(())
    }
}

impl NeedsParentheses for ExprStringLiteral {
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

pub(super) fn is_multiline_string(expr: AnyNodeRef, source: &str) -> bool {
    if expr.is_expr_string_literal() || expr.is_expr_bytes_literal() {
        let contents = &source[expr.range()];
        let prefix = StringPrefix::parse(contents);
        let quotes =
            StringQuotes::parse(&contents[TextRange::new(prefix.text_len(), contents.text_len())]);

        quotes.is_some_and(StringQuotes::is_triple)
            && memchr::memchr2(b'\n', b'\r', contents.as_bytes()).is_some()
    } else {
        false
    }
}
