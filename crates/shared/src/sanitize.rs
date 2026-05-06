use std::collections::HashSet;

/// Sanitize HTML content allowing safe tags (for blog posts, rich text)
pub fn sanitize_html(input: &str) -> String {
    let tags: HashSet<&str> = [
        "p",
        "br",
        "b",
        "i",
        "u",
        "strong",
        "em",
        "a",
        "img",
        "ul",
        "ol",
        "li",
        "h1",
        "h2",
        "h3",
        "h4",
        "blockquote",
        "code",
        "pre",
        "span",
        "div",
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        "hr",
        "sub",
        "sup",
    ]
    .into_iter()
    .collect();

    let url_schemes: HashSet<&str> = ["http", "https"].into_iter().collect();

    ammonia::Builder::new()
        .tags(tags)
        .url_schemes(url_schemes)
        .link_rel(Some("noopener noreferrer"))
        .clean(input)
        .to_string()
}

/// Strip all HTML tags — for plain text fields (comments, usernames, etc.)
pub fn sanitize_text(input: &str) -> String {
    ammonia::clean(input)
}

/// Sanitize a search query to prevent injection in ILIKE patterns
pub fn sanitize_search_query(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_html_strips_script() {
        let input = "<p>Hello</p><script>alert('xss')</script>";
        let result = sanitize_html(input);
        assert!(result.contains("<p>Hello</p>"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn test_sanitize_html_allows_safe_tags() {
        let input = "<b>bold</b> <i>italic</i> <a href=\"https://example.com\">link</a>";
        let result = sanitize_html(input);
        assert!(result.contains("<b>bold</b>"));
        assert!(result.contains("<i>italic</i>"));
        assert!(result.contains("<a "));
    }

    #[test]
    fn test_sanitize_text_strips_dangerous_html() {
        let input = "<script>alert('xss')</script>Hello";
        let result = sanitize_text(input);
        assert!(!result.contains("<script>"));
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_sanitize_search_query_escapes_wildcards() {
        let input = "test%_value\\";
        let result = sanitize_search_query(input);
        assert_eq!(result, "test\\%\\_value\\\\");
    }

    #[test]
    fn test_sanitize_search_query_trims_whitespace() {
        let result = sanitize_search_query("  hello  ");
        assert_eq!(result, "hello");
    }
}
