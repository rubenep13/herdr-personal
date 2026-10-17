//! `graph [open]` and `graph close`, with the global options anywhere.
use super::*;
use control::{ControlCommand, GraphCommand, Invocation};

fn parse(args: &[&str]) -> Result<Invocation, CliError> {
    LaunchOptions::parse(args.iter().copied()).map(|options| match options.mode {
        LaunchMode::Control(invocation) => invocation,
        mode => panic!("{args:?} parsed as {mode:?}"),
    })
}

#[test]
fn graph_command_parses_open_and_close() {
    for (args, command) in [
        (&["graph"][..], GraphCommand::Open),
        (&["graph", "open"], GraphCommand::Open),
        (&["graph", "close"], GraphCommand::Close),
    ] {
        assert_eq!(
            parse(args).unwrap().command,
            ControlCommand::Graph(command),
            "{args:?}"
        );
    }
    assert_eq!(
        parse(&["graph", "--json", "close", "--window", "3"]).unwrap(),
        Invocation {
            command: ControlCommand::Graph(GraphCommand::Close),
            json: true,
            window: Some(3),
        }
    );
    assert!(parse(&["graph", "delete"]).is_err());
    assert!(parse(&["graph", "close", "extra"]).is_err());
    assert!(matches!(
        parse(&["graph", "--help"]).unwrap().command,
        ControlCommand::Help(Some("graph"))
    ));
}
