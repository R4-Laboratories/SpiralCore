//! Scope the qualification build to exactly one known frame and Call limit.
pub const BODY_LEN: usize = if cfg!(feature = "large-object-fixture") {
    4096
} else {
    257
};
pub const FRAME_LEN: usize = BODY_LEN + 56;
pub const FRAME_SHA: &str = if cfg!(feature = "large-object-fixture") {
    "88cab4487a68269a244ee00c5ffef71868f2fdb3fbc2711cca0a79015cb426f5"
} else {
    "ff26a66df5b1d9de994e60824757686d2af8eb4433b91e74a2f24b279f600044"
};
pub const CALL_INPUT_LIMIT: usize = if cfg!(feature = "large-object-fixture") {
    2 * FRAME_LEN + 256
} else {
    2048
};
