use ruff_formatter::FormatRuleWithOptions;
use ruff_python_ast::StringLiteral;
use ruff_text_size::Ranged;

use crate::prelude::*;
use crate::string::{docstring, Quoting, StringLayout, StringPart};
use crate::QuoteStyle;

#[derive(Default)]
pub struct FormatStringLiteral {
    layout: StringLayout,
}

impl FormatRuleWithOptions<StringLiteral, PyFormatContext<'_>> for FormatStringLiteral {
    type Options = StringLayout;

    fn with_options(mut self, options: Self::Options) -> Self {
        self.layout = options;
        self
    }
}

impl FormatNodeRule<StringLiteral> for FormatStringLiteral {
    fn fmt_fields(&self, item: &StringLiteral, f: &mut PyFormatter) -> FormatResult<()> {
        let locator = f.context().locator();
        let parent_docstring_quote_style = f.context().docstring();

        let quote_style = if self.layout.is_docstring() {
            // Per PEP 8 and PEP 257, always prefer double quotes for docstrings
            QuoteStyle::Double
        } else {
            f.options().quote_style()
        };

        let normalized = StringPart::from_source(item.range(), &locator).normalize(
            Quoting::CanChange,
            &locator,
            quote_style,
            parent_docstring_quote_style,
        );

        if self.layout.is_docstring() {
            docstring::format(&normalized, f)
        } else {
            normalized.fmt(f)
        }
    }
}
