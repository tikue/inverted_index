#[derive(Debug)]
pub enum Query<'a> {
    Match(&'a str),
    And(&'a [Query<'a>]),
    Or(&'a [Query<'a>]),
}
