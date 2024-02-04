use nom::character::complete::digit0;
use nom::error::{make_error, ErrorKind};
use nom::sequence::preceded;
use nom::{
    branch::alt,
    character::complete::{char as char_parser, i32 as i32_parser},
    combinator::opt,
    IResult,
};

use crate::errors::KakeboError;

fn separator_parser(input: &str) -> IResult<&str, ()> {
    alt((char_parser(','), char_parser('.')))(input).map(|(input, _)| (input, ()))
}

fn after_separator_parser(input: &str) -> IResult<&str, i32> {
    let (input, after_digits) = digit0(input)?;
    match after_digits.len() {
        0 => Ok((input, 0)),
        1 => Ok((input, after_digits.parse::<u8>().unwrap() as i32 * 10)),
        2 => Ok((input, after_digits.parse::<u8>().unwrap() as i32)),
        _ => Err(nom::Err::Error(make_error(input, ErrorKind::TooLarge))),
    }
}

fn decimals_parser(input: &str) -> IResult<&str, i32> {
    preceded(separator_parser, after_separator_parser)(input)
}

fn value_parser(input: &str) -> IResult<&str, i32> {
    let (input, value) = opt(i32_parser)(input)?;
    let value = value.unwrap_or(0);
    if value >= i32::MAX / 100 || value <= i32::MIN / 100 {
        return Err(nom::Err::Error(make_error(input, ErrorKind::TooLarge)));
    }
    let (input, decimals) = opt(decimals_parser)(input)?;
    let decimals = decimals.unwrap_or(0);
    Ok((input, value * 100 + decimals))
}

pub fn parse_value(s: &str) -> Result<i32, KakeboError> {
    match value_parser(s) {
        Ok(("", value)) => Ok(value),
        Ok((_, _)) => Err(KakeboError::Parse("Too many characters".to_string())),
        Err(e) => Err(KakeboError::Parse(e.to_string())),
    }
}
