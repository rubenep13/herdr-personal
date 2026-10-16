#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    path::Path,
    process::{Command, Stdio},
};

#[cfg(unix)]
use std::{
    thread,
    time::{Duration, Instant},
};

fn command(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_herdr-personal-dev"));
    command
        .env_clear()
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

#[test]
fn gui_settings_are_separate_while_daemon_config_remains_shared() {
    let home = tempfile::tempdir().unwrap();
    let output = command(home.path())
        .args(["config", "path", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let paths: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        paths["managed"].as_str().unwrap(),
        home.path()
            .join("config/herdr-personal-dev/config-gpui.toml")
            .to_str()
            .unwrap()
    );
    assert_eq!(
        paths["local"].as_str().unwrap(),
        home.path()
            .join("config/herdr-personal-dev/config-gpui.local.toml")
            .to_str()
            .unwrap()
    );
    assert_eq!(
        paths["herdr"].as_str().unwrap(),
        home.path()
            .join("config/herdr/config.toml")
            .to_str()
            .unwrap()
    );
    assert!(
        !home.path().join("config").exists(),
        "path inspection must not migrate/write config"
    );
}

#[cfg(unix)]
#[test]
fn control_client_connects_only_to_the_personal_namespace() {
    use std::os::unix::net::UnixListener;
    let home = tempfile::tempdir().unwrap();
    let listen = |dir: &str| {
        let path = home.path().join("state").join(dir);
        std::fs::create_dir_all(&path).unwrap();
        let listener = UnixListener::bind(path.join("control.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        listener
    };
    let upstream = listen("herdr/gpui");
    let personal = listen("herdr-personal-dev");
    let mut child = command(home.path())
        .args(["status", "--json"])
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut own = false;
    let mut other = false;
    loop {
        if let Ok((stream, _)) = personal.accept() {
            own = true;
            drop(stream);
        }
        if let Ok((stream, _)) = upstream.accept() {
            other = true;
            drop(stream);
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("control client did not exit");
        }
        thread::sleep(Duration::from_millis(5));
    }
    let _ = child.wait_with_output().unwrap();
    assert!(!other, "personal CLI reached the upstream control socket");
    assert!(own, "personal CLI did not use its own control socket");
}

#[test]
fn explicit_skill_install_preserves_the_upstream_skill() {
    let home = tempfile::tempdir().unwrap();
    let upstream = home
        .path()
        .join(".agents/skills/herdr-gpui-browser/SKILL.md");
    std::fs::create_dir_all(upstream.parent().unwrap()).unwrap();
    let original = "<!-- herdr-gpui-managed-skill v1 -->\nupstream skill\n";
    std::fs::write(&upstream, original).unwrap();
    let output = command(home.path())
        .args(["integrations", "install-skill"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read_to_string(upstream).unwrap(), original);
    let personal = home
        .path()
        .join(".agents/skills/herdr-personal-dev-browser/SKILL.md");
    let skill = std::fs::read_to_string(personal).unwrap();
    assert!(skill.contains("name: herdr-personal-dev-browser"));
}
