#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Credentials {
    /// Unimplemented
    Basic { username: String, password: String },
    /// Bearer token authentication
    Token { token: String },
}
