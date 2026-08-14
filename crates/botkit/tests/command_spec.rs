//! End-to-end tests for the `#[derive(CommandSpec)]` macro. This is an
//! integration test so `::botkit::` paths in the generated code resolve.

use botkit::CommandSpec;

#[derive(botkit::CommandSpec, Clone, Debug, PartialEq)]
#[command(rename_rule = "snake_case", description = "Test commands:")]
enum TestCommand {
    #[command(description = "Say hi")]
    Hi,
    #[command(description = "Echo text")]
    Echo(String),
    #[command(description = "Ping", aliases = "p")]
    Ping,
    #[command(description = "Hidden from help", hide)]
    Secret,
}

#[test]
fn help_lists_visible_commands() {
    assert_eq!(
        TestCommand::help(),
        "Test commands:\n\n/hi — Say hi\n/echo — Echo text\n/ping, /p — Ping"
    );
}

#[test]
fn parse_matches_command_and_arguments() {
    use botkit::__private::BotCommands;

    assert_eq!(TestCommand::parse("/hi", "bot").unwrap(), TestCommand::Hi);
    assert_eq!(
        TestCommand::parse("/echo hello world", "bot").unwrap(),
        TestCommand::Echo("hello world".into())
    );
    // Hidden commands still parse.
    assert_eq!(
        TestCommand::parse("/secret", "bot").unwrap(),
        TestCommand::Secret
    );
    assert!(TestCommand::parse("/nope", "bot").is_err());
}

#[test]
fn parse_matches_aliases() {
    use botkit::__private::BotCommands;

    assert_eq!(
        TestCommand::parse("/ping", "bot").unwrap(),
        TestCommand::Ping
    );
    assert_eq!(TestCommand::parse("/p", "bot").unwrap(), TestCommand::Ping);
}

#[test]
fn parse_handles_bot_mention() {
    use botkit::__private::BotCommands;

    assert_eq!(
        TestCommand::parse("/echo@mybot hi", "mybot").unwrap(),
        TestCommand::Echo("hi".into())
    );
    assert!(TestCommand::parse("/echo@otherbot hi", "mybot").is_err());
}

#[test]
fn menu_lists_visible_primary_commands() {
    use botkit::__private::BotCommands;

    let menu = TestCommand::bot_commands();
    let names = menu.iter().map(|c| c.command.as_str()).collect::<Vec<_>>();
    assert_eq!(names, ["/hi", "/echo", "/ping"]);
}
