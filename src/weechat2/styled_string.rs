use super::Weechat2;
use itertools::{Itertools, Position};
use std::{collections::Bound, ops::RangeBounds};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Style {
    Bold,
    Underline,
    Italic,
    Reset,
    Color(String),
}

impl Style {
    pub fn color(color: &str) -> Self {
        Self::Color(color.to_owned())
    }

    fn style(&self) -> &str {
        match self {
            Style::Bold => Weechat2::color("bold"),
            Style::Underline => Weechat2::color("underline"),
            Style::Italic => Weechat2::color("italic"),
            Style::Reset => Weechat2::color("reset"),
            Style::Color(color) => Weechat2::color(color),
        }
    }

    fn try_unstyle(&self) -> Option<&str> {
        match self {
            Style::Bold => Some(Weechat2::color("-bold")),
            Style::Underline => Some(Weechat2::color("-underline")),
            Style::Italic => Some(Weechat2::color("-italic")),
            Style::Reset | Style::Color(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Op {
    PushStyle(Style),
    PopStyle(Style),
    Literal(String),
    Newline,
}

// Encapsulates style stack
struct StyleState {
    stack: Vec<Style>,
}

impl StyleState {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn contains(&self, style: &Style) -> bool {
        self.stack.contains(style)
    }

    pub fn push(&mut self, style: Style) {
        self.stack.push(style);
    }
    pub fn pop(&mut self, style: &Style) {
        let idx = self
            .stack
            .iter()
            .rposition(|x| x == style)
            .expect("to find requested style");
        self.stack.remove(idx);
    }

    pub fn unique_styles(&self) -> Vec<Style> {
        let mut stack = self.stack.to_owned();
        stack.sort();
        stack.dedup();
        stack
    }

    pub fn style(&self) -> String {
        let stack = self.unique_styles();

        let mut out = String::new();
        for style in stack {
            out.push_str(style.style());
        }
        out
    }

    pub fn unstyle(&self) -> String {
        let stack = self.unique_styles();

        let mut out = String::new();
        for style in stack {
            match style.try_unstyle() {
                Some(unstyle) => {
                    out.push_str(unstyle);
                },
                // If we can't exactly clear a style we will need to use a reset, so just short circuit
                // to return only the reset
                None => return Weechat2::color("reset").into(),
            };
        }
        out
    }
}

#[derive(Clone, Debug)]
pub struct StyledString {
    ops: Vec<Op>,
}

impl From<String> for StyledString {
    fn from(content: String) -> Self {
        let mut out = Self::new();
        out.push_str(&content);
        out
    }
}

impl From<&str> for StyledString {
    fn from(content: &str) -> Self {
        let mut out = Self::new();
        out.push_str(content);
        out
    }
}

impl Default for StyledString {
    fn default() -> Self {
        Self::new()
    }
}

impl StyledString {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    fn push_op(&mut self, op: Op) -> &mut Self {
        self.ops.push(op);
        self
    }

    /// Returns the minimal list of styles currently active
    fn current_styles(&self) -> Vec<Style> {
        let mut stack = StyleState::new();
        for op in &self.ops {
            match op {
                Op::PushStyle(style) => stack.push(style.clone()),
                Op::PopStyle(style) => stack.pop(style),
                Op::Literal(_) | Op::Newline => {},
            }
        }

        stack.unique_styles()
    }

    pub fn push_style(&mut self, style: Style) -> &mut Self {
        self.push_op(Op::PushStyle(style));
        self
    }

    pub fn pop_style(&mut self, style: Style) -> &mut Self {
        self.push_op(Op::PopStyle(style));
        self
    }

    pub fn push_str(&mut self, str: &str) -> &mut Self {
        for line in str.split('\n').with_position() {
            if matches!(line, Position::Middle(_) | Position::Last(_)) {
                self.push_op(Op::Newline);
            }
            self.push_op(Op::Literal(line.into_inner().to_owned()));
        }
        self
    }

    pub fn push_styled_str(&mut self, style: Style, str: &str) -> &mut Self {
        self.push_op(Op::PushStyle(style.clone()));
        self.push_str(str);
        self.push_op(Op::PopStyle(style));
        self
    }

    /// Add some other possibly styled text without it affecting the current styling
    pub fn absorb(&mut self, other: Self) -> &mut Self {
        // add other string
        self.ops.extend(other.ops.clone());
        // clear style from other string
        self.ops
            .extend(other.current_styles().into_iter().map(Op::PopStyle));
        self
    }

    /// Adds a styled string to the end of this string
    pub fn append(&mut self, other: Self) -> &mut Self {
        self.ops.extend(other.ops);
        self
    }

    pub fn build(&self) -> String {
        let mut out = String::new();

        let mut stack = StyleState::new();

        for op in &self.ops {
            match op {
                Op::Literal(text) => {
                    out.push_str(text);
                },
                Op::PushStyle(style) => {
                    if !stack.contains(style) {
                        out.push_str(style.style());
                    }
                    stack.push(style.clone());
                },
                Op::PopStyle(style) => {
                    stack.pop(style);
                    if !stack.contains(style) {
                        match style.try_unstyle() {
                            Some(unstyle) => {
                                out.push_str(unstyle);
                            },
                            None => {
                                out.push_str(Style::Reset.style());
                                out.push_str(&stack.style());
                            },
                        }
                    }
                },
                Op::Newline => {
                    out.push('\n');
                    out.push_str(&stack.style());
                },
            }
        }

        out + &stack.unstyle()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    // TODO: Make this an iterator
    pub fn lines(self) -> Vec<StyledString> {
        let mut out = Vec::new();

        let mut tmp = StyledString::new();
        for token in self.ops {
            match token {
                Op::PushStyle(_) | Op::PopStyle(_) => {
                    tmp.push_op(token);
                },
                Op::Literal(text) => {
                    debug_assert!(!text.contains('\n'));
                    tmp.push_op(Op::Literal(text));
                },
                Op::Newline => {
                    out.push(tmp.clone());
                    let styles_to_keep = tmp.current_styles();
                    tmp = StyledString::new();
                    tmp.ops
                        .extend(styles_to_keep.into_iter().map(Op::PushStyle));
                },
            }
        }

        if !tmp.is_empty() {
            out.push(tmp);
        }

        out
    }

    pub fn find(&self, substr: &str) -> Option<usize> {
        let mut offset = 0;
        for op in &self.ops {
            if let Op::Literal(text) = op {
                if let Some(pos) = text.find(substr) {
                    return Some(pos + offset);
                }

                offset += text.len();
            }
        }

        None
    }

    pub fn slice(self, range: impl RangeBounds<usize>) -> Self {
        let start = match range.start_bound() {
            Bound::Included(&idx) => idx,
            Bound::Excluded(&idx) => idx + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&idx) => idx + 1,
            Bound::Excluded(&idx) => idx,
            Bound::Unbounded => usize::MAX,
        };

        let mut out_ops = Vec::new();
        let mut offset = 0;
        for op in self.ops {
            match op {
                Op::PushStyle(_) | Op::PopStyle(_) => {
                    out_ops.push(op);
                },
                Op::Literal(text) => {
                    if start > text.len() + offset {
                        offset += text.len();
                        out_ops.push(Op::Literal(text));
                        continue;
                    }
                    out_ops.push(Op::Literal(
                        text.get((start.saturating_sub(offset))..(end - offset).min(text.len()))
                            .unwrap_or_default()
                            .to_owned(),
                    ));
                    if end > text.len() + offset {
                        offset += text.len();
                        continue;
                    }
                    break;
                },
                Op::Newline => {
                    out_ops.push(op);
                    offset += 1;
                },
            }
        }

        Self { ops: out_ops }
    }
}

#[cfg(test)]
mod test {
    use super::{Op, Style, StyledString};

    #[test]
    fn find() {
        let mut string = StyledString::new();
        string.push_style(Style::Bold);
        string.push_str("**");
        string.push_style(Style::Italic);
        string.push_str("_Hi___");
        string.pop_style(Style::Italic);
        string.push_str(" there__**");
        string.pop_style(Style::Bold);

        assert_eq!(string.find("Hi"), Some(3));
    }

    #[test]
    fn slice() {
        let mut string = StyledString::new();
        string.push_style(Style::Bold);
        string.push_str("**");
        string.push_style(Style::Italic);
        string.push_str("_Hi___");
        string.pop_style(Style::Italic);
        string.push_str(" there__**");
        string.pop_style(Style::Bold);

        let mut target = StyledString::new();
        target.push_style(Style::Bold);
        target.push_str("**");
        target.push_style(Style::Italic);
        target.push_str("_");
        assert_eq!(string.slice(0..3).build(), target.build());
    }

    #[test]
    fn newline() {
        let mut string = StyledString::new();
        string.push_str("Foo\nBar");

        assert_eq!(
            Vec::from(string.ops),
            vec![
                Op::Literal("Foo".into()),
                Op::Newline,
                Op::Literal("Bar".into()),
            ]
        );

        let mut string = StyledString::new();
        string.push_str("Foo\nBar\nBaz");

        assert_eq!(
            Vec::from(string.ops),
            vec![
                Op::Literal("Foo".into()),
                Op::Newline,
                Op::Literal("Bar".into()),
                Op::Newline,
                Op::Literal("Baz".into()),
            ]
        );
    }

    #[test]
    fn lines() {
        let mut string = StyledString::new();
        string
            .push_str("Foo\nBar")
            .push_style(Style::Bold)
            .push_str("Baz\nSpam\nEggs");

        let target: Vec<String> = vec!["Foo", "BarboldBaz-bold", "boldSpam-bold", "boldEggs-bold"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(
            string
                .lines()
                .iter()
                .map(StyledString::build)
                .collect::<Vec<_>>(),
            target
        );
    }

    #[test]
    fn multiline() {
        // weechat does not continue styles after newline chars, so we must apply the style state to the beginning of lines
        let mut string = StyledString::new();
        string
            .push_str("Foo\nBar")
            .push_style(Style::Bold)
            .push_str("Baz\nSpam\nEggs");

        assert_eq!(string.build(), "Foo\nBarboldBaz\nboldSpam\nboldEggs-bold");
    }

    #[test]
    fn string_absorption() {
        let mut string = StyledString::new();
        string.push_style(Style::Italic).push_str("[prefix]");

        let mut inner_string = StyledString::new();
        inner_string
            .push_style(Style::Bold)
            .push_style(Style::Color("red".to_owned()))
            .push_str("[inner]");

        string.absorb(inner_string);

        string
            .push_str("[middle]")
            .push_style(Style::Bold)
            .push_str("[suffix]");

        assert_eq!(
            string.build(),
            "italic[prefix]boldred[inner]-boldresetitalic[middle]bold[suffix]-bold-italic"
        );
    }

    #[test]
    fn string_absoption_nested() {
        let mut string = StyledString::new();
        string
            .push_style(Style::Bold)
            .push_style(Style::Italic)
            .push_str("[prefix]");

        let mut inner_string = StyledString::new();
        inner_string.push_style(Style::Bold).push_str("[inner]");

        string.absorb(inner_string);

        string
            .push_str("[middle]")
            .push_style(Style::Bold)
            .push_str("[suffix]");

        assert_eq!(
            string.build(),
            "bolditalic[prefix][inner][middle][suffix]-bold-italic"
        );
    }

    #[test]
    fn string_append() {
        let mut string = StyledString::new();
        string.push_style(Style::Italic);
        string.push_str("[first]");

        let mut second_string = StyledString::new();
        second_string.push_style(Style::Bold);
        second_string.push_str("[second]");

        string.append(second_string);

        assert_eq!(string.build(), "italic[first]bold[second]-bold-italic");
    }

    #[test]
    fn built_string_cleanup() {
        // Basic case
        let mut string = StyledString::new();
        string.push_style(Style::Italic);
        string.push_str("text");

        assert_eq!(string.build(), "italictext-italic");

        // Colors are not "trivially unstyle-able"
        let mut string = StyledString::new();
        string.push_style(Style::Color("red".to_owned()));
        string.push_str("text");

        assert_eq!(string.build(), "redtextreset");

        // "non-trivially unstyle-able" states should short circuit
        let mut string = StyledString::new();
        string.push_style(Style::Italic);
        string.push_style(Style::Color("red".to_owned()));
        string.push_str("text");

        assert_eq!(string.build(), "italicredtextreset");
    }

    #[test]
    fn repeats() {
        let mut string = StyledString::new();
        string.push_str("[prefix]");
        string.push_style(Style::Bold);
        string.push_str("[one_bold]");
        string.push_style(Style::Bold);
        string.push_str("[two_bold]");
        string.pop_style(Style::Bold);
        string.push_str("[one_bold]");
        string.pop_style(Style::Bold);
        string.push_str("[suffix]");

        assert_eq!(
            &string.build(),
            "[prefix]bold[one_bold][two_bold][one_bold]-bold[suffix]"
        )
    }

    mod misc {
        use crate::weechat2::{Style, StyledString};

        #[test]
        fn nested() {
            let mut string = StyledString::new();
            string.push_style(Style::Bold);
            string.push_str("**");
            string.push_style(Style::Italic);
            string.push_str("_Hi___");
            string.pop_style(Style::Italic);
            string.push_str(" there__**");
            string.pop_style(Style::Bold);

            assert_eq!(&string.build(), "bold**italic_Hi___-italic there__**-bold");
        }

        #[test]
        fn resets_inner() {
            let mut string = StyledString::new();
            string.push_style(Style::Bold);
            string.push_str("**");
            let mut inner = StyledString::new();
            inner.push_style(Style::Italic);
            inner.push_str("_Hi___");
            inner.pop_style(Style::Italic);
            string.append(inner);
            string.push_str(" there__**");
            string.pop_style(Style::Bold);

            assert_eq!(&string.build(), "bold**italic_Hi___-italic there__**-bold");
        }

        #[test]
        fn extend() {
            let mut string = StyledString::new();
            string.push_str("[prefix]");
            string.push_style(Style::Bold);
            string.push_str("[outer]");
            let mut other = StyledString::new();
            other.push_style(Style::Italic);
            other.push_str("[other]");
            string.append(other.clone());

            assert_eq!(
                &string.build(),
                "[prefix]bold[outer]italic[other]-bold-italic"
            );
        }

        #[test]
        fn newline() {
            let mut string = StyledString::new();
            string.push_str("[prefix]");
            string.push_style(Style::Bold);
            string.push_str("[bold]\n[still bold]");
            string.pop_style(Style::Bold);

            assert_eq!(&string.build(), "[prefix]bold[bold]\nbold[still bold]-bold");
        }

        #[test]
        fn single_newline() {
            let mut string = StyledString::new();
            string.push_str("[prefix]");
            string.push_style(Style::Bold);
            string.push_str("[bold]");
            string.push_str("\n");
            string.push_str("[still bold]");
            string.pop_style(Style::Bold);

            assert_eq!(&string.build(), "[prefix]bold[bold]\nbold[still bold]-bold");
        }
    }
}
