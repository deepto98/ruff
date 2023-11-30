# Don't parenthesize the value because the target's trailing comma forces it to split.
a[
    aaaaaaa,
    b,
] = cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc #  comment

# Parenthesize the value, but don't duplicate the comment.
a[
    aaaaaaa,
    b
] = cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc #  comment

# Format both as flat, but don't loos the comment.
a[
    aaaaaaa,
    b
] = bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb #  comment

#######################################################
# Test the case where a parenthesized value now fits:
a[
    aaaaaaa,
    b
] = (
    cccccccc #  comment
)

# Doesn't use `BestFit` because the target always breaks because of the trailing comma
a[
    aaaaaaa,
    b,
] = (
    cccccccc #  comment
)

# Doesn't use `BestFit` because the target always breaks because of the trailing comma
# The group breaks because of its comments
a[
    aaaaaaa,
    b
] = (
    # leading comment
    b
) = (
    cccccccc #  comment
)


a[bbbbbbbbbbbbbbbbbb] = ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc

# Does not double-parenthesize tuples
(
    first_item,
    second_item,
) = some_looooooooong_module.some_loooooog_function_name(
    first_argument, second_argument, third_argument
)


# Preserve parentheses around the first target
(
    req["ticket"]["steps"]["step"][0]["tasks"]["task"]["fields"]["field"][
        "access_request"
    ]["destinations"]["destination"][0]["ip_address"]
) = dst

(
    req["ticket"]["steps"]["step"][0]["tasks"]["task"]["fields"]["field"][
        "access_request"
    ]["destinations"]["destination"][0]["ip_address"]
) += dst
