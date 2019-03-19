#[derive(Deserialize)]
pub struct Paging<T> {
    pub data: Vec<T>,
    pub paging: PagingCursor,
}

#[derive(Deserialize)]
pub struct PagingCursor {
    pub cursors: PagingCursorInner,
}

#[derive(Deserialize)]
pub struct PagingCursorInner {
    pub after: String,
    pub before: String,
}

impl<T> IntoIterator for Paging<T> {
    type Item = T;
    type IntoIter = ::std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
