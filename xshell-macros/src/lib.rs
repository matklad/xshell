//! Private implementation details of `xshell`.

#![deny(missing_debug_implementations)]
#![deny(rust_2018_idioms)]

use std::iter;

use proc_macro::{Group, Span, TokenStream, TokenTree};

#[doc(hidden)]
#[proc_macro]
pub fn __cmd(macro_arg: TokenStream) -> TokenStream {
    try_cmd(macro_arg).unwrap_or_else(|msg| parse_ts(&format!("compile_error!({:?})", msg)))
}

type Result<T> = std::result::Result<T, String>;

fn try_cmd(macro_arg: TokenStream) -> Result<TokenStream> {
    let (cmd, literal) = {
        let mut iter = macro_arg.into_iter();
        let cmd = iter.next().unwrap();
        let literal = iter.next().unwrap();
        assert!(iter.next().is_none());
        (cmd, literal)
    };

    let literal_text = literal.to_string();
    if !(matches!(literal, TokenTree::Literal(_)) && literal_text.starts_with('"')) {
        return Err("expected a plain string literal".to_string());
    }

    let mut args = shell_lex(literal_text.as_str(), literal.span());

    let mut res = TokenStream::new();

    {
        let (_joined_to_prev, splat, program) =
            args.next().ok_or_else(|| "command can't be empty".to_string())??;
        if splat {
            return Err("can't splat program name".to_string());
        }
        res.extend(Some(cmd));
        res.extend(parse_ts("::new"));
        res.extend(program);
    }

    let mut prev_spat = false;
    for arg in args {
        let (joined_to_prev, splat, arg) = arg?;
        if prev_spat && joined_to_prev {
            return Err(format!(
                "can't combine splat with concatenation, add spaces around `{{{}...}}`",
                trim_decorations(&res.into_iter().last().unwrap().to_string()),
            ));
        }
        prev_spat = splat;

        let method = match (joined_to_prev, splat) {
            (false, false) => ".arg",
            (false, true) => ".args",
            (true, false) => ".__extend_arg",
            (true, true) => {
                return Err(format!(
                    "can't combine splat with concatenation, add spaces around `{{{}...}}`",
                    trim_decorations(&arg.to_string()),
                ))
            }
        };

        res.extend(parse_ts(method));
        res.extend(arg);
    }

    Ok(res)
}

fn trim_decorations(s: &str) -> &str {
    &s[1..s.len() - 1]
}

fn shell_lex(
    cmd: &str,
    call_site: Span,
) -> impl Iterator<Item = Result<(bool, bool, TokenStream)>> + '_ {
    tokenize(cmd).map(move |token| {
        let token = token?;
        let mut splat = false;
        let ts = match token.kind {
            TokenKind::Word => parse_ts(&format!("(\"{}\")", token.text)),
            TokenKind::String => parse_ts(&format!("(\"{}\")", trim_decorations(token.text))),
            TokenKind::Interpolation { splat: s } => {
                splat = s;
                let text = trim_decorations(token.text);
                let text = &text[..text.len() - (if splat { "...".len() } else { 0 })];
                if !(text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
                    return Err(format!(
                        "can only interpolate simple variables, got this expression instead: `{}`",
                        text
                    ));
                }
                let ts = if splat { format!("({})", text) } else { format!("(&({}))", text) };
                respan(parse_ts(&ts), call_site)
            }
        };
        Ok((token.joined_to_prev, splat, ts))
    })
}

/// Like trim_matches except only trims a maximum of 1 match
fn strip_matches<'a>(s: &'a str, pattern: &str) -> &'a str {
    s.strip_prefix(pattern).unwrap_or(s).strip_suffix(pattern).unwrap_or(s)
}

fn tokenize(cmd: &str) -> impl Iterator<Item = Result<Token<'_>>> + '_ {
    let mut cmd = strip_matches(cmd, "\"");

    iter::from_fn(move || {
        let old_len = cmd.len();
        cmd = cmd.trim_start();
        let joined_to_prev = old_len == cmd.len();
        if cmd.is_empty() {
            return None;
        }
        let (len, kind) = match next_token(cmd) {
            Ok(it) => it,
            Err(err) => {
                cmd = "";
                return Some(Err(err));
            }
        };
        let token = Token { joined_to_prev, text: &cmd[..len], kind };
        cmd = &cmd[len..];
        Some(Ok(token))
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

fn next_token(s: &str) -> Result<(usize, TokenKind)> {
    if s.starts_with('{') {
        let len = s.find('}').ok_or_else(|| "unclosed `{` in command".to_string())? + 1;
        let splat = s[..len].ends_with("...}");
        return Ok((len, TokenKind::Interpolation { splat }));
    }
    if s.starts_with('\'') {
        let len = s[1..].find('\'').ok_or_else(|| "unclosed `'` in command".to_string())? + 2;
        return Ok((len, TokenKind::String));
    }
    let len =
        s.find(|it: char| it.is_ascii_whitespace() || it == '\'' || it == '{').unwrap_or(s.len());
    Ok((len, TokenKind::Word))
}

fn respan(ts: TokenStream, span: Span) -> TokenStream {
    let mut res = TokenStream::new();
    for tt in ts {
        let tt = match tt {
            TokenTree::Ident(mut ident) => {
                ident.set_span(ident.span().resolved_at(span).located_at(span));
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
