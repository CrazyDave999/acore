
pub trait StepByOne {
    fn next(&self) -> Self;
}
pub struct NumIter<T> {
    cur: T,
    end: T,
}
impl<T> Iterator for NumIter<T>
where
    T: StepByOne + Copy + PartialOrd,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.end {
            let ret = self.cur;
            self.cur = self.cur.next();
            Some(ret)
        } else {
            None
        }
    }
}
pub struct NumRange<T> {
    l: T,
    r: T,
}
impl<T> NumRange<T> {
    pub fn new(l: T, r: T) -> Self {
        Self { l, r }
    }
}
impl<T> IntoIterator for NumRange<T>
where
    T: StepByOne + Copy + PartialOrd,
{
    type Item = T;
    type IntoIter = NumIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        NumIter { cur: self.l, end: self.r }
    }
}