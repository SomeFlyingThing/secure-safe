use std::path::PathBuf;
use crate::assert_unchecked;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ParsingError {
    MissingArg(usize),
    NoArgs,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ParsedArgs {
    Restore(PathBuf),
    Delete(PathBuf),
    Add(PathBuf),
}

///this trait is made to be used on vectors it auto handles emptiness
trait EmptyArgs<T> {
    fn get_args(&self, pos: usize) -> Result<T, ParsingError>;
} 


impl<T> EmptyArgs<T> for Vec<T>
where
    T: Clone,
{
    #[inline]
    fn get_args(&self, pos: usize) -> Result<T, ParsingError> {

        //CHECK: to always be with the len prechecked
        unsafe {
            assert_unchecked(pos > 0);
        }

        //if empty
        let Some(item) = self.get(pos) else {
            eprintln!("missing arg in position {pos}");
            return Err(ParsingError::MissingArg(pos));
        };

        Ok(item.clone())
    }
}

pub fn parse() -> Result<ParsedArgs, ParsingError> {
    let full_args: Vec<String> = std::env::args().collect();

    let Some(args) = full_args.get(1) else {
        eprintln!("no args were provided");
        return Err(ParsingError::NoArgs);
    };

    match args.as_str() {
        "add" => Ok(ParsedArgs::Add(PathBuf::from(full_args.get_args(2)?))),
        "delete" => Ok(ParsedArgs::Delete(PathBuf::from(full_args.get_args(2)?))),
        "restore" => Ok(ParsedArgs::Restore(PathBuf::from(full_args.get_args(2)?))),
        _ => unreachable!(),
    }
}
