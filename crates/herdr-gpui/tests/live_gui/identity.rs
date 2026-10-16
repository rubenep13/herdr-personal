use super::*;

#[test]
fn explicitly_selected_baseline_uses_the_same_isolated_environment() {
    let sandbox = Sandbox::new();
    let command = gui_command(&sandbox, |name| match name {
        "HERDR_TEST_CLIENT_BINARY" => Some("/test/baseline-client".into()),
        _ => None,
    });
    assert_eq!(command.get_program(), "/test/baseline-client");
    assert!(
        !command
            .get_envs()
            .any(|(key, _)| key == "HERDR_TEST_CLIENT_BINARY")
    );
    assert!(
        command
            .get_envs()
            .any(|(key, value)| key == "HOME" && value == Some(sandbox.dir.as_os_str()))
    );
}
