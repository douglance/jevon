//! Shared plumbing for the command crates.
//!
//! Two things every command needs and none should repeat: building a client
//! from the environment, and turning an SDK error into the structured failure
//! the CLI reports. Error codes are stable strings because agents branch on
//! them, so they are derived from the error's kind rather than its message.

mod input;
mod mcp;

pub use input::{STDIN, lines, questions, text};
pub use mcp::{read_only, read_only_remote};

use std::sync::Arc;

use typesafe_sdk_client::Client;
use typesafe_sdk_config::Builder;
use typesafe_sdk_env::Process;
use typesafe_sdk_error::{ApiErrorKind, Error};
use typesafe_sdk_http::Reqwest;

/// Builds a client from the environment.
///
/// # Errors
/// Returns [`Error::Invalid`] when no API key is configured, or a connection
/// error if the TLS backend cannot start.
pub fn client() -> Result<Client, Error> {
    let config = Builder::new().build(&Process)?;
    let transport = Arc::new(Reqwest::new()?);
    Ok(Client::with_transport(config, transport))
}

/// The stable code a failure is reported under.
///
/// These names are part of the CLI's contract: a caller that retries on
/// `RATE_LIMIT` or re-prompts on `AUTHENTICATION_ERROR` depends on them not
/// drifting when a message is reworded.
#[must_use]
pub fn code_for(error: &Error) -> &'static str {
    match error {
        Error::Timeout { .. } => "TIMEOUT",
        Error::Connection { .. } => "CONNECTION_ERROR",
        Error::UserAbort { .. } => "CANCELLED",
        Error::Invalid(_) => "VALIDATION_ERROR",
        Error::Api(api) => api_code(api.kind),
    }
}

/// The code for each API failure, as data rather than branches.
const API_CODES: &[(ApiErrorKind, &str)] = &[
    (ApiErrorKind::BadRequest, "BAD_REQUEST"),
    (ApiErrorKind::Authentication, "AUTHENTICATION_ERROR"),
    (ApiErrorKind::PermissionDenied, "PERMISSION_DENIED"),
    (ApiErrorKind::NotFound, "NOT_FOUND"),
    (ApiErrorKind::UnprocessableEntity, "UNPROCESSABLE_ENTITY"),
    (ApiErrorKind::RateLimit, "RATE_LIMIT"),
    (ApiErrorKind::InternalServer, "INTERNAL_SERVER_ERROR"),
];

fn api_code(kind: ApiErrorKind) -> &'static str {
    API_CODES
        .iter()
        .find(|&&(candidate, _)| candidate == kind)
        .map_or("API_ERROR", |&(_, code)| code)
}

/// Whether trying the same command again could plausibly succeed.
#[must_use]
pub fn retryable(error: &Error) -> bool {
    match error {
        Error::Timeout { .. } | Error::Connection { .. } => true,
        Error::Api(api) => matches!(
            api.kind,
            ApiErrorKind::RateLimit | ApiErrorKind::InternalServer
        ),
        Error::UserAbort { .. } | Error::Invalid(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{code_for, retryable};
    use typesafe_sdk_error::{Body, Error};
    use typesafe_sdk_headers::Headers;

    fn api(status: u16) -> Error {
        Error::from_response(status, Body::Empty, &Headers::new())
    }

    #[test]
    fn every_status_maps_to_a_stable_code() {
        let cases = [
            (400, "BAD_REQUEST"),
            (401, "AUTHENTICATION_ERROR"),
            (403, "PERMISSION_DENIED"),
            (404, "NOT_FOUND"),
            (422, "UNPROCESSABLE_ENTITY"),
            (429, "RATE_LIMIT"),
            (503, "INTERNAL_SERVER_ERROR"),
            (418, "API_ERROR"),
        ];
        for (status, expected) in cases {
            assert_eq!(code_for(&api(status)), expected, "status {status}");
        }
    }

    #[test]
    fn local_failures_have_their_own_codes() {
        assert_eq!(
            code_for(&Error::Invalid("bad".to_owned())),
            "VALIDATION_ERROR"
        );
        assert_eq!(code_for(&Error::Timeout { timeout_ms: 1 }), "TIMEOUT");
        assert_eq!(code_for(&Error::connection("reset")), "CONNECTION_ERROR");
        assert_eq!(code_for(&Error::aborted()), "CANCELLED");
    }

    #[test]
    fn only_transient_failures_are_worth_repeating() {
        assert!(retryable(&api(429)));
        assert!(retryable(&api(503)));
        assert!(retryable(&Error::Timeout { timeout_ms: 1 }));
        assert!(!retryable(&api(400)));
        assert!(!retryable(&api(401)));
        assert!(!retryable(&Error::Invalid("bad".to_owned())));
    }
}
