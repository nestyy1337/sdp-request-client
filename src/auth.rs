#[derive(Clone, Debug)]
pub enum Credentials {
    Basic { username: String, password: String },
    Token { token: String },
}
