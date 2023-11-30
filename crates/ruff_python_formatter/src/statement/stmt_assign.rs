use ruff_formatter::{format_args, write, FormatError};
use ruff_python_ast::{AnyNodeRef, Expr, Operator, StmtAssign};

use crate::builders::parenthesize_if_expands;
use crate::comments::{
    trailing_comments, Comments, LeadingDanglingTrailingComments, SourceComment, SuppressionKind,
};
use crate::context::{NodeLevel, WithNodeLevel};
use crate::expression::parentheses::{
    is_expression_parenthesized, NeedsParentheses, OptionalParentheses, Parentheses, Parenthesize,
};
use crate::expression::{has_own_parentheses, maybe_parenthesize_expression};
use crate::prelude::*;
use crate::preview::is_prefer_splitting_right_hand_side_of_assignments_enabled;
use crate::statement::trailing_semicolon;

#[derive(Default)]
pub struct FormatStmtAssign;

impl FormatNodeRule<StmtAssign> for FormatStmtAssign {
    fn fmt_fields(&self, item: &StmtAssign, f: &mut PyFormatter) -> FormatResult<()> {
        let StmtAssign {
            range: _,
            targets,
            value,
        } = item;

        let (first, rest) = targets.split_first().ok_or(FormatError::syntax_error(
            "Expected at least on assignment target",
        ))?;

        let format_first =
            format_with(|f| write!(f, [first.format(), space(), token("="), space()]));

        if is_prefer_splitting_right_hand_side_of_assignments_enabled(f.context()) {
            if let Some((last, head)) = rest.split_last() {
                format_first.fmt(f)?;

                for target in head {
                    FormatTarget { target }.fmt(f)?;
                }

                FormatAssignmentValue {
                    before_operator: last,
                    operator: AnyAssignmentOperator::Assign,
                    value,
                    statement: item.into(),
                }
                .fmt(f)?;
            } else if has_target_own_parentheses(first, f.context())
                && !is_expression_parenthesized(
                    first.into(),
                    f.context().comments().ranges(),
                    f.context().source(),
                )
            {
                FormatAssignmentValue {
                    before_operator: first,
                    operator: AnyAssignmentOperator::Assign,
                    value,
                    statement: item.into(),
                }
                .fmt(f)?;
            } else {
                format_first.fmt(f)?;
                FormatStatementsLastExpression::new(value, item).fmt(f)?;
            }
        } else {
            write!(f, [format_first, FormatTargets { targets: rest }])?;

            FormatStatementsLastExpression::new(value, item).fmt(f)?;
        }

        if f.options().source_type().is_ipynb()
            && f.context().node_level().is_last_top_level_statement()
            && trailing_semicolon(item.into(), f.context().source()).is_some()
            && matches!(targets.as_slice(), [Expr::Name(_)])
        {
            token(";").fmt(f)?;
        }

        Ok(())
    }

    fn is_suppressed(
        &self,
        trailing_comments: &[SourceComment],
        context: &PyFormatContext,
    ) -> bool {
        SuppressionKind::has_skip_comment(trailing_comments, context.source())
    }
}

struct FormatTarget<'a> {
    target: &'a Expr,
}

impl Format<PyFormatContext<'_>> for FormatTarget<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'_>>) -> FormatResult<()> {
        if has_target_own_parentheses(self.target, f.context())
            && !f.context().comments().has_leading(self.target)
            && !f.context().comments().has_trailing(self.target)
        {
            self.target
                .format()
                .with_options(Parentheses::Never)
                .fmt(f)?;
        } else {
            parenthesize_if_expands(&self.target.format().with_options(Parentheses::Never))
                .fmt(f)?;
        }

        write!(f, [space(), token("="), space()])
    }
}

#[derive(Debug)]
struct FormatTargets<'a> {
    targets: &'a [Expr],
}

impl Format<PyFormatContext<'_>> for FormatTargets<'_> {
    fn fmt(&self, f: &mut PyFormatter) -> FormatResult<()> {
        if let Some((first, rest)) = self.targets.split_first() {
            let comments = f.context().comments();

            let parenthesize = if comments.has_leading(first) || comments.has_trailing(first) {
                ParenthesizeTarget::Always
            } else if has_target_own_parentheses(first, f.context()) {
                ParenthesizeTarget::Never
            } else {
                ParenthesizeTarget::IfBreaks
            };

            let group_id = if parenthesize == ParenthesizeTarget::Never {
                Some(f.group_id("assignment_parentheses"))
            } else {
                None
            };

            let format_first = format_with(|f: &mut PyFormatter| {
                let mut f = WithNodeLevel::new(NodeLevel::Expression(group_id), f);
                match parenthesize {
                    ParenthesizeTarget::Always => {
                        write!(f, [first.format().with_options(Parentheses::Always)])
                    }
                    ParenthesizeTarget::Never => {
                        write!(f, [first.format().with_options(Parentheses::Never)])
                    }
                    ParenthesizeTarget::IfBreaks => {
                        write!(
                            f,
                            [
                                if_group_breaks(&token("(")),
                                soft_block_indent(&first.format().with_options(Parentheses::Never)),
                                if_group_breaks(&token(")"))
                            ]
                        )
                    }
                }
            });

            write!(
                f,
                [group(&format_args![
                    format_first,
                    space(),
                    token("="),
                    space(),
                    FormatTargets { targets: rest }
                ])
                .with_group_id(group_id)]
            )
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParenthesizeTarget {
    Always,
    Never,
    IfBreaks,
}

/// Formats the last expression in statements that start with a keyword (like `return`) or after an operator (assignments).
///
/// It avoids parenthesizing unsplittable values (like `None`, `True`, `False`, Names, a subset of strings) just to make
/// the trailing comment fit and inlines a trailing comment if the value itself exceeds the configured line width:
///
/// The implementation formats the statement's and value's trailing end of line comments:
/// * after the expression if the expression needs no parentheses (necessary or the `expand_parent` makes the group never fit).
/// * inside the parentheses if the expression exceeds the line-width.
///
/// ```python
/// a = loooooooooooooooooooooooooooong # with_comment
/// b = (
///     short # with_comment
/// )
/// ```
///
/// Which gets formatted to:
///
/// ```python
/// # formatted
/// a = (
///     loooooooooooooooooooooooooooong # with comment
/// )
/// b = short # with comment
/// ```
///
/// The long name gets parenthesized because it exceeds the configured line width and the trailing comma of the
/// statement gets formatted inside (instead of outside) the parentheses.
///
/// The `short` name gets unparenthesized because it fits into the configured line length, regardless of whether
/// the comment exceeds the line width or not.
///
/// This logic isn't implemented in [`place_comment`] by associating trailing statement comments to the expression because
/// doing so breaks the suite empty lines formatting that relies on trailing comments to be stored on the statement.
pub(super) struct FormatStatementsLastExpression<'a> {
    expression: &'a Expr,
    parent: AnyNodeRef<'a>,
}

impl<'a> FormatStatementsLastExpression<'a> {
    pub(super) fn new<P: Into<AnyNodeRef<'a>>>(expression: &'a Expr, parent: P) -> Self {
        Self {
            expression,
            parent: parent.into(),
        }
    }
}

impl Format<PyFormatContext<'_>> for FormatStatementsLastExpression<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'_>>) -> FormatResult<()> {
        let can_inline_comment = LayoutRequest::CanInlineComments
            .resolve(self.expression, self.parent, f.context())
            .should_inline_comments();

        if !can_inline_comment {
            return maybe_parenthesize_expression(
                self.expression,
                self.parent,
                Parenthesize::IfBreaks,
            )
            .fmt(f);
        }

        let comments = f.context().comments().clone();
        let expression_comments = comments.leading_dangling_trailing(self.expression);

        if let Some(inline_comments) =
            OptionalParenthesesInlinedComments::new(&expression_comments, self.parent, &comments)
        {
            BestFitParenthesizeWithInlineComments {
                inline_comments,
                expression: self.expression,
            }
            .fmt(f)
        } else {
            // Preserve the parentheses if the expression has any leading or trailing comments,
            // same as `maybe_parenthesize_expression`
            self.expression
                .format()
                .with_options(Parentheses::Always)
                .fmt(f)
        }
    }
}

/// Formats the last expression in statements that start with a keyword (like `return`) or after an operator (assignments).
///
/// It avoids parenthesizing unsplittable values (like `None`, `True`, `False`, Names, a subset of strings) just to make
/// the trailing comment fit and inlines a trailing comment if the value itself exceeds the configured line width:
///
/// The implementation formats the statement's and value's trailing end of line comments:
/// * after the expression if the expression needs no parentheses (necessary or the `expand_parent` makes the group never fit).
/// * inside the parentheses if the expression exceeds the line-width.
///
/// ```python
/// a = loooooooooooooooooooooooooooong # with_comment
/// b = (
///     short # with_comment
/// )
/// ```
///
/// Which gets formatted to:
///
/// ```python
/// # formatted
/// a = (
///     loooooooooooooooooooooooooooong # with comment
/// )
/// b = short # with comment
/// ```
///
/// The long name gets parenthesized because it exceeds the configured line width and the trailing comma of the
/// statement gets formatted inside (instead of outside) the parentheses.
///
/// The `short` name gets unparenthesized because it fits into the configured line length, regardless of whether
/// the comment exceeds the line width or not.
///
/// This logic isn't implemented in [`place_comment`] by associating trailing statement comments to the expression because
/// doing so breaks the suite empty lines formatting that relies on trailing comments to be stored on the statement.
pub(super) struct FormatAssignmentValue<'a> {
    /// The expression that comes right before the assignment operator. This is either
    /// the last target, or the annotated assignment type annotation.
    pub(super) before_operator: &'a Expr,

    /// The assignment operator. Either `Assign` (`=`) or the operator used by the augmented assignment statement.
    pub(super) operator: AnyAssignmentOperator,

    /// The value assigned to the target(s)
    pub(super) value: &'a Expr,

    /// The assignment statement.
    pub(super) statement: AnyNodeRef<'a>,
}

impl Format<PyFormatContext<'_>> for FormatAssignmentValue<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'_>>) -> FormatResult<()> {
        let format_before_operator = format_with(|f: &mut PyFormatter| {
            // Preserve parentheses around targets with comments.
            if f.context().comments().has_leading(self.before_operator)
                || f.context().comments().has_trailing(self.before_operator)
            {
                self.before_operator.format().fmt(f)
            }
            // Never parenthesize targets that come with their own parentheses, e.g. don't parenthesize lists or dictionary literals.
            else if has_target_own_parentheses(self.before_operator, f.context()) {
                self.before_operator
                    .format()
                    .with_options(Parentheses::Never)
                    .fmt(f)
            } else {
                parenthesize_if_expands(
                    &self
                        .before_operator
                        .format()
                        .with_options(Parentheses::Never),
                )
                .fmt(f)
            }
        });

        let layout_response =
            LayoutRequest::UsesBestFitLayout.resolve(self.value, self.statement, f.context());

        if !layout_response.uses_best_fit() {
            return write!(
                f,
                [
                    format_before_operator,
                    space(),
                    self.operator,
                    space(),
                    maybe_parenthesize_expression(
                        self.value,
                        self.statement,
                        Parenthesize::IfBreaks
                    )
                ]
            );
        }

        // Don't inline comments for attribute and call expressions for black compatibility
        let should_inline_comments = layout_response.should_inline_comments();

        let comments = f.context().comments().clone();
        let expression_comments = comments.leading_dangling_trailing(self.value);
        let inline_comments = if should_inline_comments {
            OptionalParenthesesInlinedComments::new(&expression_comments, self.statement, &comments)
        } else if expression_comments.has_leading() || expression_comments.has_trailing_own_line() {
            None
        } else {
            Some(OptionalParenthesesInlinedComments::default())
        };

        let Some(inline_comments) = inline_comments else {
            // Preserve the parentheses if the expression has any leading or trailing own line comments
            // same as `maybe_parenthesize_expression`
            return write!(
                f,
                [
                    format_before_operator,
                    space(),
                    self.operator,
                    space(),
                    self.value.format().with_options(Parentheses::Always)
                ]
            );
        };

        // Prevent inline comments to be formatted as part of the expression.
        inline_comments.mark_formatted();

        let mut last_target = format_before_operator.memoized();

        // Avoid using the `best fit` layout if it is known that the last target breaks
        // This is mainly a perf improvement that avoids an additional memoization
        // and using the costly `BestFit` layout if it is already known that the left breaks,
        // because it would always pick the last best fitting variant.
        if last_target.inspect(f)?.will_break() {
            // Format the value without any parentheses. The check above guarantees that it never has leading comments.
            return write!(
                f,
                [
                    last_target,
                    space(),
                    self.operator,
                    space(),
                    self.value.format().with_options(Parentheses::Never),
                    inline_comments
                ]
            );
        }

        let format_value = self
            .value
            .format()
            .with_options(Parentheses::Never)
            .memoized();

        // Try to fit the last assignment target and the value on a single line:
        // ```python
        // a = b = c
        // ```
        let format_flat = format_with(|f| {
            write!(
                f,
                [
                    last_target,
                    space(),
                    self.operator,
                    space(),
                    format_value,
                    inline_comments
                ]
            )
        });

        // Don't break the last assignment target but parenthesize the value to see if it fits
        // ```python
        // a["bbbbb"] = (
        //      c
        // )
        // ```
        let format_parenthesize_value = format_with(|f| {
            write!(
                f,
                [
                    last_target,
                    space(),
                    self.operator,
                    space(),
                    token("("),
                    block_indent(&format_args![format_value, inline_comments]),
                    token(")")
                ]
            )
        });

        // Fall back to parenthesizing (or splitting) the last target part if we can't make the value
        // fit. Don't parenthesize the value to avoid unnecessary parentheses.
        // ```python
        // a[
        //      "bbbbb"
        // ] = c
        // ```
        let format_split_left = format_with(|f| {
            write!(
                f,
                [
                    last_target,
                    space(),
                    self.operator,
                    space(),
                    format_value,
                    inline_comments
                ]
            )
        });

        // Call expression have one extra layout.
        if self.value.is_call_expr() {
            best_fitting![
                format_flat,
                // Avoid parenthesizing the call expression if the `(` fit on the line
                format_args![
                    last_target,
                    space(),
                    self.operator,
                    space(),
                    group(&format_value).should_expand(true),
                ],
                format_parenthesize_value,
                format_split_left
            ]
            .fmt(f)
        } else {
            best_fitting![format_flat, format_parenthesize_value, format_split_left].fmt(f)
        }
    }
}

#[derive(Debug, Default)]
struct OptionalParenthesesInlinedComments<'a> {
    expression: &'a [SourceComment],
    statement: &'a [SourceComment],
}

impl<'a> OptionalParenthesesInlinedComments<'a> {
    fn new(
        expression_comments: &LeadingDanglingTrailingComments<'a>,
        statement: AnyNodeRef<'a>,
        comments: &'a Comments<'a>,
    ) -> Option<Self> {
        if expression_comments.has_leading() || expression_comments.has_trailing_own_line() {
            return None;
        }

        let statement_trailing_comments = comments.trailing(statement);
        let after_end_of_line = statement_trailing_comments
            .partition_point(|comment| comment.line_position().is_end_of_line());
        let (stmt_inline_comments, _) = statement_trailing_comments.split_at(after_end_of_line);

        let after_end_of_line = expression_comments
            .trailing
            .partition_point(|comment| comment.line_position().is_end_of_line());

        let (expression_inline_comments, trailing_own_line_comments) =
            expression_comments.trailing.split_at(after_end_of_line);

        debug_assert!(trailing_own_line_comments.is_empty(), "The method should have returned early if the expression has trailing own line comments");

        Some(OptionalParenthesesInlinedComments {
            expression: expression_inline_comments,
            statement: stmt_inline_comments,
        })
    }

    fn is_empty(&self) -> bool {
        self.expression.is_empty() && self.statement.is_empty()
    }

    fn iter_comments(&self) -> impl Iterator<Item = &'a SourceComment> {
        self.expression.iter().chain(self.statement)
    }

    fn mark_formatted(&self) {
        for comment in self.expression {
            comment.mark_formatted();
        }
    }
}

impl Format<PyFormatContext<'_>> for OptionalParenthesesInlinedComments<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'_>>) -> FormatResult<()> {
        for comment in self.iter_comments() {
            comment.mark_unformatted();
        }

        write!(
            f,
            [
                trailing_comments(self.expression),
                trailing_comments(self.statement)
            ]
        )
    }
}

#[derive(Copy, Clone, Debug)]
pub(super) enum AnyAssignmentOperator {
    Assign,
    AugAssign(Operator),
}

impl Format<PyFormatContext<'_>> for AnyAssignmentOperator {
    fn fmt(&self, f: &mut Formatter<PyFormatContext<'_>>) -> FormatResult<()> {
        match self {
            AnyAssignmentOperator::Assign => token("=").fmt(f),
            AnyAssignmentOperator::AugAssign(operator) => {
                write!(f, [operator.format(), token("=")])
            }
        }
    }
}

struct BestFitParenthesizeWithInlineComments<'a> {
    expression: &'a Expr,
    inline_comments: OptionalParenthesesInlinedComments<'a>,
}

impl Format<PyFormatContext<'_>> for BestFitParenthesizeWithInlineComments<'_> {
    fn fmt(&self, f: &mut Formatter<PyFormatContext>) -> FormatResult<()> {
        let group_id = f.group_id("optional_parentheses");

        let f = &mut WithNodeLevel::new(NodeLevel::Expression(Some(group_id)), f);

        best_fit_parenthesize(&format_with(|f| {
            self.inline_comments.mark_formatted();

            self.expression
                .format()
                .with_options(Parentheses::Never)
                .fmt(f)?;

            if !self.inline_comments.is_empty() {
                // If the expressions exceeds the line width, format the comments in the parentheses
                if_group_breaks(&self.inline_comments).fmt(f)?;
            }

            Ok(())
        }))
        .with_group_id(Some(group_id))
        .fmt(f)?;

        if !self.inline_comments.is_empty() {
            // If the line fits into the line width, format the comments after the parenthesized expression
            if_group_fits_on_line(&self.inline_comments)
                .with_group_id(Some(group_id))
                .fmt(f)?;
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum LayoutRequest {
    CanInlineComments,
    UsesBestFitLayout,
}

impl LayoutRequest {
    fn resolve<'a>(
        self,
        expression: &'a Expr,
        parent: AnyNodeRef,
        context: &PyFormatContext,
    ) -> LayoutResponse<'a> {
        let result = match expression {
            Expr::Name(_)
            | Expr::NoneLiteral(_)
            | Expr::NumberLiteral(_)
            | Expr::BooleanLiteral(_) => true,
            Expr::StringLiteral(string) => {
                string.needs_parentheses(parent, context) == OptionalParentheses::BestFit
            }
            Expr::BytesLiteral(bytes) => {
                bytes.needs_parentheses(parent, context) == OptionalParentheses::BestFit
            }
            Expr::FString(fstring) => {
                fstring.needs_parentheses(parent, context) == OptionalParentheses::BestFit
            }
            Expr::Attribute(attribute) if self == LayoutRequest::UsesBestFitLayout => {
                attribute.needs_parentheses(parent, context) == OptionalParentheses::BestFit
            }
            Expr::Call(call) if self == LayoutRequest::UsesBestFitLayout => {
                call.needs_parentheses(parent, context) == OptionalParentheses::BestFit
            }
            _ => false,
        };

        if result {
            match self {
                LayoutRequest::CanInlineComments => LayoutResponse::InlineComments,
                LayoutRequest::UsesBestFitLayout => LayoutResponse::UsesBestFit(expression),
            }
        } else {
            LayoutResponse::Other
        }
    }
}

#[derive(Copy, Clone)]
enum LayoutResponse<'a> {
    Other,
    InlineComments,
    UsesBestFit(&'a Expr),
}

impl LayoutResponse<'_> {
    fn uses_best_fit(self) -> bool {
        matches!(self, LayoutResponse::UsesBestFit(_))
    }

    fn should_inline_comments(self) -> bool {
        match self {
            LayoutResponse::InlineComments => true,
            LayoutResponse::UsesBestFit(Expr::Attribute(_) | Expr::Call(_)) => false,
            LayoutResponse::UsesBestFit(_) => true,
            LayoutResponse::Other => false,
        }
    }
}

pub(super) fn has_target_own_parentheses(target: &Expr, context: &PyFormatContext) -> bool {
    matches!(target, Expr::Tuple(_)) || has_own_parentheses(target, context).is_some()
}
