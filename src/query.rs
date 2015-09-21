#[derive(Debug)]
pub enum Query<'a> {
    And(&'a Query<'a>, &'a Query<'a>),
    Or(&'a Query<'a>, &'a Query<'a>),
    Match(&'a str)
}
