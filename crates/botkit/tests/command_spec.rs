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
    #[command(description = "Renamed command", rename = "renamed")]
    Renamed,
    #[command(description = "Hidden from help", hide)]
    Secret,
}

#[test]
fn help_lists_visible_commands() {
    assert_eq!(
        TestCommand::help(),
        "Test commands:\n\n/hi — Say hi\n/echo — Echo text\n/ping, /p — Ping\n/renamed — Renamed command"
    );
}

#[test]
fn parse_matches_command_and_arguments() {
    assert_eq!(TestCommand::parse("/hi", "bot"), Some(TestCommand::Hi));
    assert_eq!(
        TestCommand::parse("/echo hello world", "bot"),
        Some(TestCommand::Echo("hello world".into()))
    );
    // Hidden commands still parse.
    assert_eq!(
        TestCommand::parse("/secret", "bot"),
        Some(TestCommand::Secret)
    );
    assert_eq!(TestCommand::parse("/nope", "bot"), None);
}

#[test]
fn parse_matches_aliases() {
    assert_eq!(TestCommand::parse("/ping", "bot"), Some(TestCommand::Ping));
    assert_eq!(TestCommand::parse("/p", "bot"), Some(TestCommand::Ping));
}

#[test]
fn parse_respects_explicit_rename() {
    assert_eq!(
        TestCommand::parse("/renamed", "bot"),
        Some(TestCommand::Renamed)
    );
    assert_eq!(TestCommand::parse("/renamed_command", "bot"), None);
}

#[test]
fn parse_handles_bot_mention() {
    assert_eq!(
        TestCommand::parse("/echo@mybot hi", "mybot"),
        Some(TestCommand::Echo("hi".into()))
    );
    assert_eq!(TestCommand::parse("/echo@otherbot hi", "mybot"), None);
}

#[test]
fn menu_lists_visible_primary_commands() {
    let menu = TestCommand::menu();
    let names = menu.iter().map(|m| m.command.as_str()).collect::<Vec<_>>();
    assert_eq!(names, ["/hi", "/echo", "/ping", "/renamed"]);
}
