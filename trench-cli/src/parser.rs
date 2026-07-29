use crate::client::{CliError, CliResult};

pub fn parse_one_arg(args: &[&str], usage: &str) -> CliResult<String> {
    if args.len() != 1 {
        return Err(CliError::from(format!("Usage: {usage}")));
    }
    Ok(args[0].to_string())
}

pub fn parse_two_args(args: &[&str], usage: &str) -> CliResult<(String, String)> {
    if args.len() != 2 {
        return Err(CliError::from(format!("Usage: {usage}")));
    }
    Ok((args[0].to_string(), args[1].to_string()))
}

pub fn parse_three_or_more_args(
    args: &[&str],
    usage: &str,
) -> CliResult<(String, String, Vec<u8>)> {
    if args.len() < 3 {
        return Err(CliError::from(format!("Usage: {usage}")));
    }
    let table = args[0].to_string();
    let key = args[1].to_string();
    let value = args[2..].join(" ").into_bytes();
    Ok((table, key, value))
}
