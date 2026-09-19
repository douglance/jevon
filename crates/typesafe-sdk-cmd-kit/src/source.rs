//! Where a command's items come from, stated rather than assumed.
//!
//! Reading standard input when nobody asked for it is safe only on a transport
//! that owns standard input, and the CLI is the only one that does. Over MCP
//! stdio that stream carries JSON-RPC frames, so an implicit `read_to_string`
//! blocks the tool call forever and then starts eating the protocol — measured,
//! not theorised: a frame sent two seconds later still got a reply and one sent
//! four seconds later did not.
//!
//! Nothing in `incurs` distinguishes the two. `TypedContext.request` is
//! populated only for the HTTP transport, and `IsTerminal` cannot tell an MCP
//! pipe from `cat items.txt | jev classify`. Since the transport cannot be
//! detected, the source has to be declared.

use typesafe_sdk_error::{Error, Result};

use crate::input::{items as read_items, lines};

/// Where the items are coming from.
#[derive(Debug, PartialEq, Eq)]
pub enum Items {
    /// Passed as arguments. The only source an MCP client can use.
    Inline(Vec<String>),
    /// A JSON array in a file, or on stdin when the path is `-`.
    File(String),
    /// One item per line on standard input.
    StdinLines,
}

/// Picks the one source that was asked for.
///
/// # Errors
/// Returns [`Error::Invalid`] when more than one source is given, or none.
pub fn resolve(inline: &[String], file: Option<&str>, stdin: bool) -> Result<Items> {
    match (inline.is_empty(), file, stdin) {
        (true, None, false) => Err(Error::Invalid(NONE.to_owned())),
        (false, None, false) => Ok(Items::Inline(inline.to_vec())),
        (true, Some(path), false) => Ok(Items::File(path.to_owned())),
        (true, None, true) => Ok(Items::StdinLines),
        _ => Err(Error::Invalid(MANY.to_owned())),
    }
}

/// Reads whichever source was chosen. The only caller that touches stdin.
///
/// # Errors
/// Returns [`Error::Invalid`] when the source holds no items, or a file cannot
/// be read or parsed.
pub fn read(source: Items) -> Result<Vec<String>> {
    match source {
        Items::Inline(items) => inline(items),
        Items::File(path) => read_items(&path),
        Items::StdinLines => lines(),
    }
}

/// Inline items, blank ones dropped, the way the other two sources treat them.
fn inline(items: Vec<String>) -> Result<Vec<String>> {
    let kept: Vec<String> = items.into_iter().filter(|i| !i.trim().is_empty()).collect();
    if kept.is_empty() {
        return Err(Error::Invalid("`--items` held no items".to_owned()));
    }
    Ok(kept)
}

/// Said when no source was given. Names all three, because the right one
/// depends on how the command was reached and the caller knows that and we do
/// not.
const NONE: &str = "No items were given. Pass --items to supply them directly, \
                    --items-file to read a JSON array, or --stdin to read one per \
                    line from standard input. Standard input is never read unless \
                    --stdin is passed, because over MCP that stream carries the \
                    protocol.";

/// Said when several were. Names all three again so the caller can see which
/// pair collided without re-reading the help.
const MANY: &str = "Pass exactly one of --items, --items-file or --stdin.";

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "a test that cannot fail loudly is not a test"
)]
mod tests {
    use super::{Items, resolve};

    fn err(inline: &[&str], file: Option<&str>, stdin: bool) -> String {
        let owned: Vec<String> = inline.iter().map(|s| (*s).to_owned()).collect();
        resolve(&owned, file, stdin).unwrap_err().to_string()
    }

    #[test]
    fn each_source_resolves_to_itself() {
        assert_eq!(
            resolve(&["a".to_owned()], None, false).unwrap(),
            Items::Inline(vec!["a".to_owned()])
        );
        assert_eq!(
            resolve(&[], Some("x.json"), false).unwrap(),
            Items::File("x.json".to_owned())
        );
        assert_eq!(resolve(&[], None, true).unwrap(), Items::StdinLines);
    }

    /// The whole point of the change: silence is not a request for stdin.
    /// Over MCP that stream is the protocol, so reading it hangs the call.
    #[test]
    fn nothing_given_is_an_error_rather_than_a_silent_read_of_stdin() {
        let message = err(&[], None, false);
        for flag in ["--items", "--items-file", "--stdin"] {
            assert!(message.contains(flag), "{flag} missing from: {message}");
        }
    }

    /// Every pair collides, and the message names the alternatives rather than
    /// only the one that lost.
    #[test]
    fn two_sources_are_refused_whichever_two() {
        for (inline, file, stdin) in [
            (&["a"][..], Some("x.json"), false),
            (&["a"][..], None, true),
            (&[][..], Some("x.json"), true),
            (&["a"][..], Some("x.json"), true),
        ] {
            let message = err(inline, file, stdin);
            assert!(message.contains("exactly one"), "{message}");
        }
    }
}
