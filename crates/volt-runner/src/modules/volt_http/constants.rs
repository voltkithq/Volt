use std::collections::BTreeMap;
use std::time::Duration;

pub(super) type ResponseHeaders = BTreeMap<String, Vec<String>>;
pub(super) type FetchResult = (i32, ResponseHeaders, String);

pub(super) const HTTP_DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const HTTP_MAX_TIMEOUT_MS: u64 = 120_000;
pub(crate) const HTTP_MAX_RESPONSE_BODY_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const HTTP_MAX_REQUEST_BODY_BYTES: usize = 256 * 1024;
pub(super) const HTTP_MAX_CONCURRENT_REQUESTS: usize = 32;
pub(super) const HTTP_MAX_HEADER_COUNT: usize = 64;
pub(super) const HTTP_MAX_HEADER_NAME_BYTES: usize = 128;
pub(super) const HTTP_MAX_HEADER_VALUE_BYTES: usize = 8 * 1024;
pub(super) const HTTP_MAX_REDIRECTS: usize = 10;
pub(super) const RESPONSE_BODY_PROPERTY: &str = "__voltBody";
