use core::marker::PhantomData;
use nom::{error::ErrorKind, error::ParseError, Err, IResult, InputLength, Parser};

pub(crate) fn sign_str_always(n: i32) -> &'static str {
    if n.signum() >= 0 { "+" } else { "-" }
}

pub(crate) fn sign_str_if_neg(n: i32) -> &'static str {
    if n.signum() == -1 { "-" } else { "" }
}

// nom

pub struct Optional<P: Parser<I, O, E>, I, O, E> {
    inner: P,
    phantom: PhantomData<(I, O, E)>,
}

impl<I, O, E, P> Parser<I, Option<O>, E> for Optional<P, I, O, E>
where
    I: Clone,
    E: ParseError<I>,
    P: Parser<I, O, E>,
{
    fn parse(&mut self, input: I) -> IResult<I, Option<O>, E> {
        // nom::combinator::opt;
        let i = input.clone();
        match self.inner.parse(input) {
            Ok((i, o)) => Ok((i, Some(o))),
            Err(Err::Error(_)) => Ok((i, None)),
            Err(e) => Err(e),
        }
    }
}

pub struct Complete<P: Parser<I, O, E>, I, O, E> {
    inner: P,
    phantom: PhantomData<(I, O, E)>,
}
impl<I, O, E, P> Parser<I, O, E> for Complete<P, I, O, E>
where
    I: InputLength,
    E: ParseError<I>,
    P: Parser<I, O, E>,
{
    fn parse(&mut self, input: I) -> IResult<I, O, E> {
        // nom::combinator::all_consuming;
        let (input, res) = self.inner.parse(input)?;
        if input.input_len() == 0 {
            Ok((input, res))
        } else {
            Err(Err::Error(E::from_error_kind(input, ErrorKind::Eof)))
        }
    }
}

pub struct AndIgnore<P: Parser<I, O, E>, G: Parser<I, O2, E>, I, O, O2, E> {
    inner: P,
    to_ignore: G,
    phantom: PhantomData<(I, O, O2, E)>,
}
impl<I, O, O2, E, P, G> Parser<I, O, E> for AndIgnore<P, G, I, O, O2, E>
where
    E: ParseError<I>,
    P: Parser<I, O, E>,
    G: Parser<I, O2, E>,
{
    fn parse(&mut self, input: I) -> IResult<I, O, E> {
        let (input, me) = self.inner.parse(input)?;
        let (input, _) = self.to_ignore.parse(input)?;
        Ok((input, me))
    }
}

pub trait ParserExt<I, O, E>: Parser<I, O, E> {
    /// Equivalent to wrapping a parser with [nom::combinator::opt].
    fn optional(self) -> Optional<Self, I, O, E>
    where
        Self: core::marker::Sized,
    {
        Optional {
            inner: self,
            phantom: Default::default(),
        }
    }
    /// Equivalent to wrapping a parser with [nom::combinator::all_consuming].
    fn complete(self) -> Complete<Self, I, O, E>
    where
        Self: core::marker::Sized,
    {
        Complete {
            inner: self,
            phantom: Default::default(),
        }
    }
    /// Equivalent to following a parser with `let (remain, _) = g(remain)?;`
    fn and_ignore<G, O2>(self, g: G) -> AndIgnore<Self, G, I, O, O2, E>
    where
        G: Parser<I, O2, E>,
        Self: core::marker::Sized,
    {
        AndIgnore {
            inner: self,
            to_ignore: g,
            phantom: Default::default(),
        }
    }
}

impl<T, I, O, E> ParserExt<I, O, E> for T where T: Parser<I, O, E> {}
