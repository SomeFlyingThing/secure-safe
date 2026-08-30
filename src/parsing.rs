use std::path::PathBuf;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ParsingError {
    UnknownArgs,
    MissingArg(usize),
    NoArgs,
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum ParsedArgs {
    Restore(String),
    WatchDir(PathBuf),
    Delete(String),
    Add(PathBuf),
    About,
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
        "delete" => Ok(ParsedArgs::Delete(full_args.get_args(2)?)),
        "restore" => Ok(ParsedArgs::Restore(full_args.get_args(2)?)),
        "about" => Ok(ParsedArgs::About),
        "watchd" => Ok(ParsedArgs::WatchDir(PathBuf::from(full_args.get_args(2)?))),
        _ => {
            eprintln!("unknown command");
            Err(ParsingError::UnknownArgs)
        },
    }
}
