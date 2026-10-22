//! Share one stdin reader between the startup prompt and the final console pause.
use std::sync::{
    mpsc::{sync_channel, Receiver},
    Mutex, OnceLock,
};

fn events() -> &'static Mutex<Receiver<bool>> {
    static EVENTS: OnceLock<Mutex<Receiver<bool>>> = OnceLock::new();
    EVENTS.get_or_init(|| {
        let (sender, receiver) = sync_channel(1);
        std::thread::spawn(move || loop {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if sender.send(line.trim().is_empty()).is_err() {
                        break;
                    }
                }
            }
        });
        Mutex::new(receiver)
    })
}

pub fn enter_pressed() -> bool {
    matches!(events().lock().unwrap().try_recv(), Ok(true))
}

pub fn wait_for_enter() {
    let receiver = events().lock().unwrap();
    while let Ok(enter) = receiver.recv() {
        if enter {
            break;
        }
    }
}
