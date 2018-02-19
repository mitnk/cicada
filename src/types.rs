pub type Tokens = Vec<(String, String)>;
pub type Redirection = (String, String, String);

#[derive(Debug)]
pub struct Command {
    pub tokens: Tokens,
    pub redirects: Vec<Redirection>,
}
