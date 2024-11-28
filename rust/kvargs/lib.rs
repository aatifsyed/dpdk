pub mod parsing {
    use std::cmp;

    use nom::{
        branch::alt,
        bytes::{
            complete::{is_not, tag, take_till1},
            streaming::take_until,
        },
        combinator::{recognize, rest, verify},
        multi::fold_many0,
        sequence::{delimited, separated_pair},
        IResult, Parser,
    };

    type ParseResult<'a, O = &'a [u8]> = IResult<&'a [u8], O>;

    pub fn kv(input: &[u8]) -> ParseResult<(&[u8], Option<&[u8]>)> {
        alt((
            separated_pair(key, tag(&b"="[..]), value).map(|(k, v)| (k, Some(v))),
            key.map(|k| (k, None)),
        ))(input)
    }

    fn key(input: &[u8]) -> ParseResult<'_> {
        take_till1(|it| it == b',' || it == b'=')(input)
    }

    fn value(input: &[u8]) -> ParseResult<'_> {
        recognize(fold_many0(alt((list, bare)), || (), |(), _el| ()))(input)
    }

    fn list(input: &[u8]) -> ParseResult<'_> {
        delimited(tag(&b"["[..]), is_not(&b"]"[..]), tag(&b"]"[..]))(input)
    }

    fn bare<'a>(input: &'a [u8]) -> ParseResult<'a> {
        verify(
            |input: &'a [u8]| match (
                take_until::<_, _, ()>(&b","[..])(input),
                take_until::<_, _, ()>(&b"["[..])(input),
            ) {
                (Ok(by_comma), Ok(by_bracket)) => {
                    Ok(cmp::min_by_key(by_comma, by_bracket, |(_, val)| val.len()))
                }
                (Ok(ok), Err(_)) | (Err(_), Ok(ok)) => Ok(ok),
                (Err(_), Err(_)) => rest(input),
            },
            |it: &[u8]| !it.is_empty(),
        )(input)
    }

    pub fn callback_separated<I, O, E, SepO, ParseP, SepP>(
        mut parser: ParseP,
        mut separator: SepP,
        mut fold: impl FnMut(O),
    ) -> impl FnMut(I) -> IResult<I, (), E>
    where
        ParseP: Parser<I, O, E>,
        SepP: Parser<I, SepO, E>,
        I: Clone,
    {
        move |input: I| -> IResult<I, (), E> {
            let (mut input, first) = parser.parse(input)?;
            fold(first);
            while let Ok((after_sep, _sep)) = separator.parse(input.clone()) {
                let (next_input, item) = parser.parse(after_sep)?;
                fold(item);
                input = next_input;
            }
            Ok((input, ()))
        }
    }
}
