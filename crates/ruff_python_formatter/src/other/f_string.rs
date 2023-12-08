use ruff_python_ast::{FString, FStringElement, FStringExpressionElement};
use ruff_text_size::Ranged;

use crate::prelude::*;
use crate::string::{Quoting, StringPart};

#[derive(Default)]
pub struct FormatFString;

impl FormatNodeRule<FString> for FormatFString {
    fn fmt_fields(&self, item: &FString, f: &mut PyFormatter) -> FormatResult<()> {
        let locator = f.context().locator();
        let parent_docstring_quote_style = f.context().docstring();

        let unprefixed = locator
            .slice(item.range())
            .trim_start_matches(|c| c != '"' && c != '\'');
        let triple_quoted = unprefixed.starts_with(r#"""""#) || unprefixed.starts_with(r"'''");
        let quoting = if item.elements.iter().any(|element| match element {
            FStringElement::Expression(FStringExpressionElement { range, .. }) => {
                let string_content = locator.slice(*range);
                if triple_quoted {
                    string_content.contains(r#"""""#) || string_content.contains("'''")
                } else {
                    string_content.contains(['"', '\''])
                }
            }
            FStringElement::Literal(_) => false,
        }) {
            Quoting::Preserve
        } else {
            Quoting::CanChange
        };

        let result = StringPart::from_source(item.range(), &locator)
            .normalize(
                quoting,
                &locator,
                f.options().quote_style(),
                parent_docstring_quote_style,
            )
            .fmt(f);

        // TODO(dhruvmanila): With PEP 701, comments can be inside f-strings.
        // This is to mark all of those comments as formatted but we need to
        // figure out how to handle them. Note that this needs to be done only
        // after the f-string is formatted, so only for all the non-formatted
        // comments.
        let comments = f.context().comments();
        item.elements.iter().for_each(|value| {
            comments.mark_verbatim_node_comments_formatted(value.into());
        });

        result
    }
}
