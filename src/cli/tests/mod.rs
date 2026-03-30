#[cfg(test)]
mod tests {
    use clap::Parser;

    #[test]
    fn test_parse_add_command() {
        let cli = crate::cli::Cli::parse_from(["doclib", "add"]);
        assert!(matches!(cli.command, crate::cli::Commands::Add));
    }

    #[test]
    fn test_parse_list_command() {
        let cli = crate::cli::Cli::parse_from(["doclib", "list"]);
        assert!(matches!(cli.command, crate::cli::Commands::List));
    }

    #[test]
    fn test_parse_search_command() {
        let cli = crate::cli::Cli::parse_from(["doclib", "search"]);
        assert!(matches!(cli.command, crate::cli::Commands::Search));
    }

    #[test]
    fn test_no_command_shows_help() {
        let result = crate::cli::Cli::try_parse_from(["doclib"]);
        assert!(result.is_err());
    }
}
