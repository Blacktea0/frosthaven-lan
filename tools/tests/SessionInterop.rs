#[path = "../../src/win.rs"]
mod win;
#[path = "../../src/ipc.rs"]
mod ipc;
#[path = "../../src/trace.rs"]
mod trace;
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
    let records = trace::Records::default();
    let _collector = trace::Collector::start(pid, records.clone()).unwrap();
    writeln!(child.stdin.take().unwrap(), "start").unwrap();
    wait(&channel, ipc::STARTUP, "READY");
    channel.write(ipc::REQUEST, "native-1|status");
    wait(&channel, ipc::RESPONSE, "native-1|网络组件：status\r\n");
    channel.write(ipc::REQUEST, "native-2|exit");
    wait(&channel, ipc::RESPONSE, "native-2|网络组件：exit\r\n");
    assert!(child.wait().unwrap().success());
    let expected = format!("MANAGED-LONG-BEGIN\n{}\nMANAGED-LONG-END", "异常调用栈\n".repeat(10000));
    assert!(records.snapshot().iter().any(|line| line.ends_with(&expected)), "Managed diagnostics must not be truncated to the control-slot size");
    assert!(records.snapshot().iter().any(|line| line.contains("MANAGED ERROR") && line.contains("inner failure") && line.contains("outer failure")), "Nested managed exceptions must retain their full ToString output");
    println!("PASS: full managed diagnostics and nested exceptions survive cross-process transfer");
    println!("PASS: native and managed processes exchange readiness, Unicode commands, and correlated replies without files");
}
