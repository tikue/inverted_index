#[derive(Debug)]
pub enum Query<'a> {
    Match(&'a str),
    And(&'a Query<'a>, &'a Query<'a>),
}
