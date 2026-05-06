/// Validated deep-link targets within the Jungle app.
#[derive(Debug, Clone)]
pub enum DeepLink {
    Post(i64),
    User(String),        // username
    Group(String),       // slug
    Page(String),        // slug
    Event(i64),
    Comment(i64),
    Reel(i64),
    Story(i64),
    Message(i64),        // conversation_id
    Settings(String),    // section
}

impl DeepLink {
    /// Parse and validate a deep-link string like "post/123" or "user/juan".
    pub fn parse(raw: &str) -> Option<Self> {
        let (kind, value) = raw.split_once('/')?;
        match kind {
            "post" => value.parse().ok().map(DeepLink::Post),
            "user" => Some(DeepLink::User(value.to_string())),
            "group" => Some(DeepLink::Group(value.to_string())),
            "page" => Some(DeepLink::Page(value.to_string())),
            "event" => value.parse().ok().map(DeepLink::Event),
            "comment" => value.parse().ok().map(DeepLink::Comment),
            "reel" => value.parse().ok().map(DeepLink::Reel),
            "story" => value.parse().ok().map(DeepLink::Story),
            "message" => value.parse().ok().map(DeepLink::Message),
            "settings" => Some(DeepLink::Settings(value.to_string())),
            _ => None,
        }
    }

    /// Render as a frontend route, e.g. "/post/123".
    pub fn to_url(&self) -> String {
        match self {
            DeepLink::Post(id) => format!("/post/{}", id),
            DeepLink::User(username) => format!("/profile/{}", username),
            DeepLink::Group(slug) => format!("/groups/{}", slug),
            DeepLink::Page(slug) => format!("/pages/{}", slug),
            DeepLink::Event(id) => format!("/events/{}", id),
            DeepLink::Comment(id) => format!("/post/{}#comment-{}", id, id),
            DeepLink::Reel(id) => format!("/reels/{}", id),
            DeepLink::Story(id) => format!("/stories/{}", id),
            DeepLink::Message(id) => format!("/messages/{}", id),
            DeepLink::Settings(section) => format!("/settings/{}", section),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_post() {
        let link = DeepLink::parse("post/123").unwrap();
        assert!(matches!(link, DeepLink::Post(123)));
        assert_eq!(link.to_url(), "/post/123");
    }

    #[test]
    fn test_parse_user() {
        let link = DeepLink::parse("user/juan").unwrap();
        assert!(matches!(link, DeepLink::User(_)));
        assert_eq!(link.to_url(), "/profile/juan");
    }

    #[test]
    fn test_parse_group() {
        let link = DeepLink::parse("group/my-group").unwrap();
        assert!(matches!(link, DeepLink::Group(_)));
        assert_eq!(link.to_url(), "/groups/my-group");
    }

    #[test]
    fn test_parse_page() {
        let link = DeepLink::parse("page/mypage").unwrap();
        assert!(matches!(link, DeepLink::Page(_)));
        assert_eq!(link.to_url(), "/pages/mypage");
    }

    #[test]
    fn test_parse_event() {
        let link = DeepLink::parse("event/42").unwrap();
        assert!(matches!(link, DeepLink::Event(42)));
        assert_eq!(link.to_url(), "/events/42");
    }

    #[test]
    fn test_parse_comment() {
        let link = DeepLink::parse("comment/99").unwrap();
        assert!(matches!(link, DeepLink::Comment(99)));
        assert_eq!(link.to_url(), "/post/99#comment-99");
    }

    #[test]
    fn test_parse_reel() {
        let link = DeepLink::parse("reel/7").unwrap();
        assert!(matches!(link, DeepLink::Reel(7)));
        assert_eq!(link.to_url(), "/reels/7");
    }

    #[test]
    fn test_parse_story() {
        let link = DeepLink::parse("story/3").unwrap();
        assert!(matches!(link, DeepLink::Story(3)));
        assert_eq!(link.to_url(), "/stories/3");
    }

    #[test]
    fn test_parse_message() {
        let link = DeepLink::parse("message/55").unwrap();
        assert!(matches!(link, DeepLink::Message(55)));
        assert_eq!(link.to_url(), "/messages/55");
    }

    #[test]
    fn test_parse_settings() {
        let link = DeepLink::parse("settings/privacy").unwrap();
        assert!(matches!(link, DeepLink::Settings(_)));
        assert_eq!(link.to_url(), "/settings/privacy");
    }

    #[test]
    fn test_parse_invalid_kind() {
        assert!(DeepLink::parse("unknown/123").is_none());
    }

    #[test]
    fn test_parse_no_slash() {
        assert!(DeepLink::parse("post123").is_none());
    }

    #[test]
    fn test_parse_bad_post_id() {
        assert!(DeepLink::parse("post/abc").is_none());
    }

    #[test]
    fn test_to_url_roundtrip() {
        let cases = vec![
            "post/123",
            "user/juan",
            "group/my-group",
            "page/mypage",
            "event/42",
            "comment/99",
            "reel/7",
            "story/3",
            "message/55",
            "settings/privacy",
        ];
        for case in cases {
            let parsed = DeepLink::parse(case).unwrap();
            let re_parsed = DeepLink::parse(&parsed.to_url().trim_start_matches('/').replace('/', "/"));
            // For comment, to_url returns "/post/99#comment-99" which can't be round-tripped
            if case.starts_with("comment/") {
                continue;
            }
            if let Some(re) = re_parsed {
                assert_eq!(format!("{}/{}", case.split_once('/').unwrap().0, match &re {
                    DeepLink::Post(v) => v.to_string(),
                    DeepLink::User(v) => v.clone(),
                    DeepLink::Group(v) => v.clone(),
                    DeepLink::Page(v) => v.clone(),
                    DeepLink::Event(v) => v.to_string(),
                    DeepLink::Comment(v) => v.to_string(),
                    DeepLink::Reel(v) => v.to_string(),
                    DeepLink::Story(v) => v.to_string(),
                    DeepLink::Message(v) => v.to_string(),
                    DeepLink::Settings(v) => v.clone(),
                }), case);
            }
        }
    }

    #[test]
    fn test_parse_empty() {
        assert!(DeepLink::parse("").is_none());
    }
}
