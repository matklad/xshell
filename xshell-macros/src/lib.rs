extern crate proc_macro;

use std::iter;

use proc_macro::{Group, Span, TokenStream, TokenTree};

#[doc(hidden)]
#[proc_macro]
pub fn __cmd(macro_arg: TokenStream) -> TokenStream {
    let (cmd, literal) = {
        let mut iter = macro_arg.into_iter();
        let cmd = iter.next().unwrap();
        let literal = iter.next().unwrap();
        assert!(iter.next().is_none());
        (cmd, literal)
    };

    let literal_text = literal.to_string();
    let mut args = shell_lex(literal_text.as_str(), literal.span());

    let mut res = TokenStream::new();

    {
        let (_joined_to_prev, splat, program) = args.next().expect("command line is empty!");
        assert!(!splat);
        res.extend(Some(cmd));
        res.extend(parse_ts("::new"));
        res.extend(program);
    }

    let mut prev_spat = false;
    for (joined_to_prev, splat, arg) in args {
        assert!(!(joined_to_prev && splat));
        if prev_spat && joined_to_prev {
            panic!("can't splat and concat simultaneously")
        }
        prev_spat = splat;

        let method = match (joined_to_prev, splat) {
            (false, false) => ".arg",
            (false, true) => ".args",
            (true, false) => ".__extend_arg",
            (true, true) => panic!("can't splat and concat simultaneously"),
        };

        res.extend(parse_ts(method));
        res.extend(arg);
    }

    res
}

fn shell_lex(cmd: &str, call_site: Span) -> impl Iterator<Item = (bool, bool, TokenStream)> + '_ {
    fn trim_decorations(s: &str) -> &str {
        &s[1..s.len() - 1]
    }

    tokenize(cmd).map(move |token| {
        let mut splat = false;
        let ts = match token.kind {
            TokenKind::Word => parse_ts(&format!("(\"{}\")", token.text)),
            TokenKind::String => parse_ts(&format!("(\"{}\")", trim_decorations(token.text))),
            TokenKind::Interpolation { splat: s } => {
                splat = s;
                let text = trim_decorations(token.text);
                let text = &text[..text.len() - (if splat { "...".len() } else { 0 })];
                assert!(
                    text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                    "can only interpolate variables"
                );
                let ts = if splat { format!("({})", text) } else { format!("(&({}))", text) };
                respan(parse_ts(&ts), call_site)
            }
        };
        (token.joined_to_prev, splat, ts)
    })
}

fn tokenize(cmd: &str) -> impl Iterator<Item = Token<'_>> + '_ {
    let mut cmd = cmd.trim_matches('"');
    iter::from_fn(move || {
        let old_len = cmd.len();
        cmd = cmd.trim_start();
        let joined_to_prev = old_len == cmd.len();
        if cmd.is_empty() {
            return None;
        }
        let (len, kind) = next_token(cmd);
        let token = Token { joined_to_prev, text: &cmd[..len], kind };
        cmd = &cmd[len..];
        Some(token)
    })
}

#[derive(Debug)]
struct Token<'a> {
    joined_to_prev: bool,
    text: &'a str,
    kind: TokenKind,
}
#[derive(Debug)]
enum TokenKind {
    Word,
    String,
    Interpolation { splat: bool },
}

fn next_token(s: &str) -> (usize, TokenKind) {
    if s.starts_with('{') {
        let len = s.find('}').unwrap() + 1;
        let splat = s[..len].ends_with("...}");
        return (len, TokenKind::Interpolation { splat });
    }
    if s.starts_with('\'') {
        let len = s[1..].find('\'').unwrap() + 2;
        return (len, TokenKind::String);
    }
    let len =
        s.find(|it: char| it.is_ascii_whitespace() || it == '\'' || it == '{').unwrap_or(s.len());
    (len, TokenKind::Word)
}

fn respan(ts: TokenStream, span: Span) -> TokenStream {
    let mut res = TokenStream::new();
    for tt in ts {
        let tt = match tt {
            TokenTree::Ident(mut ident) => {
                ident.set_span(ident.span().resolved_at(span));
                TokenTree::Ident(ident)
            }
            TokenTree::Group(group) => {
                TokenTree::Group(Group::new(group.delimiter(), respan(group.stream(), span)))
            }
            _ => tt,
        };
        res.extend(Some(tt))
    }
    res
}

fn parse_ts(s: &str) -> TokenStream {
    s.parse().unwrap()
}
