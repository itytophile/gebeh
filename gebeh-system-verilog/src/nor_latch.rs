#[derive(Debug)]
struct NorLatch<'a> {
    s: &'a str,
    r: &'a str,
    q: Option<&'a str>,
    q_n: Option<&'a str>,
}
