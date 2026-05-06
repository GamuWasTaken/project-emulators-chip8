use std::iter::Peekable;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum CliToken<'a> {
    Flag(&'a str),  // -p | --parse
    Value(&'a str), // test.ch8
}
impl<'a> From<&'a str> for CliToken<'a> {
    fn from(value: &'a str) -> Self {
        let cli_token = match value.starts_with("-") {
            true => Self::Flag(value),
            false => Self::Value(value),
        };
        cli_token
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum CliArg<'a> {
    Flag(&'a str),           // -p
    Value(&'a str, &'a str), // -f file.ch8
    DefaultValue(&'a str),   // file.ch8
}

// Structure
pub struct Tokenize<'a, T>(T)
where
    T: Iterator<Item = &'a str>;

impl<'a, T> Iterator for Tokenize<'a, T>
where
    T: Iterator<Item = &'a str>,
{
    type Item = CliToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.next()?.into())
    }
}

// Extension Trait
pub trait Tokenizable<'a, T>
where
    T: Iterator<Item = &'a str>,
{
    fn tokenize(self) -> Tokenize<'a, T>;
}

impl<'a, I> Tokenizable<'a, I> for I
where
    I: Iterator<Item = &'a str>,
{
    fn tokenize(self) -> Tokenize<'a, I> {
        Tokenize(self)
    }
}

// Structure
pub struct Parse<'a, T>(Peekable<T>)
where
    T: Iterator<Item = CliToken<'a>>;

impl<'a, T> Iterator for Parse<'a, T>
where
    T: Iterator<Item = CliToken<'a>>,
{
    type Item = CliArg<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use CliToken::*;

        let next = self.0.next()?;

        let value = match next {
            Flag(_) => self.0.peek(),
            Value(str) => return Some(Self::Item::DefaultValue(str)),
        };

        match value {
            // Theres no next or next is a flag
            None | Some(Flag(_)) => Some(Self::Item::Flag(next.as_str())),
            // Next is a value
            Some(Value(_)) => Some(Self::Item::Value(
                next.as_str(),
                self.0.next().unwrap().as_str(),
            )), // Peek garanties unwrap wont panic
        }
    }
}

// Extension trait
pub trait Parseable<'a, Tokens>
where
    Tokens: Iterator<Item = CliToken<'a>>,
{
    fn parse(self) -> Parse<'a, Tokens>;
}

impl<'a, T> Parseable<'a, T> for T
where
    T: Iterator<Item = CliToken<'a>>,
{
    fn parse(self) -> Parse<'a, T> {
        Parse(self.peekable())
    }
}
