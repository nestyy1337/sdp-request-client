#[derive(Clone, Debug)]
pub enum Credentials {
    /// Unimplemented
    Basic { username: String, password: String },
    /// Bearer token authentication
    Token { token: String },
}
