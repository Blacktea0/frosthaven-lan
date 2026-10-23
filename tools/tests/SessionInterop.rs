#[path = "../../src/win.rs"]
mod win;
#[path = "../../src/ipc.rs"]
mod ipc;
use std::{io::{BufRead, BufReader, Write}, process::{Command, Stdio}, time::{Duration, Instant}};
fn wait(channel: &ipc::Channel, slot: usize, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if channel.read(slot).as_deref() == Some(expected) { return; }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("Shared-memory response was not received: {expected}");
}
fn main() {
    let fixture = std::env::args().nth(1).expect("managed fixture path");
    let mut child = Command::new(fixture).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    let pid: u32 = line.trim().parse().unwrap();
    let channel = ipc::Channel::create(pid).unwrap();
    writeln!(child.stdin.take().unwrap(), "start").unwrap();
    wait(&channel, ipc::STARTUP, "READY");
    channel.write(ipc::REQUEST, "native-1|status");
    wait(&channel, ipc::RESPONSE, "native-1|网络组件：status\r\n");
    channel.write(ipc::REQUEST, "native-2|exit");
    wait(&channel, ipc::RESPONSE, "native-2|网络组件：exit\r\n");
    assert!(child.wait().unwrap().success());
    println!("PASS: native and managed processes exchange readiness, Unicode commands, and correlated replies without files");
}
