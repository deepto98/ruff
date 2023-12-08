use ruff_formatter::{FormatOwnedWithRule, FormatRefWithRule};
use ruff_python_ast::FStringPart;

use crate::prelude::*;

#[derive(Default)]
pub struct FormatFStringPart;

impl FormatRule<FStringPart, PyFormatContext<'_>> for FormatFStringPart {
    fn fmt(&self, item: &FStringPart, f: &mut PyFormatter) -> FormatResult<()> {
        match item {
            FStringPart::Literal(string_literal) => string_literal.format().fmt(f),
            FStringPart::FString(f_string) => f_string.format().fmt(f),
        }
    }
}

impl<'ast> AsFormat<PyFormatContext<'ast>> for FStringPart {
    type Format<'a> = FormatRefWithRule<'a, FStringPart, FormatFStringPart, PyFormatContext<'ast>>;

    fn format(&self) -> Self::Format<'_> {
        FormatRefWithRule::new(self, FormatFStringPart)
    }
}

impl<'ast> IntoFormat<PyFormatContext<'ast>> for FStringPart {
    type Format = FormatOwnedWithRule<FStringPart, FormatFStringPart, PyFormatContext<'ast>>;

    fn into_format(self) -> Self::Format {
        FormatOwnedWithRule::new(self, FormatFStringPart)
    }
}
