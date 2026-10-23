#![allow(dead_code)]
//! Length-framed, acknowledged diagnostics. No runtime files or text truncation.
use crate::win::*;
use std::os::windows::io::AsRawHandle;
use std::{
    io, ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[derive(Default, Clone)]
pub struct Records(Arc<Mutex<Vec<String>>>);
impl Records {
    pub fn record(&self, message: impl AsRef<str>) {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        self.0.lock().unwrap().push(format!(
            "[{}.{:03}] {}",
            elapsed.as_secs(),
            elapsed.subsec_millis(),
            message.as_ref()
        ));
    }
    pub fn snapshot(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}
struct Pipe(Handle);
impl Drop for Pipe {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn name(pid: u32) -> Vec<u16> {
    wide(&format!(r"\\.\pipe\FrosthavenLan-Diagnostics-{pid}"))
}
#[repr(C)]
struct SecurityAttributes {
    length: u32,
    descriptor: Handle,
    inherit: i32,
}
struct Descriptor(Handle);
impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}
fn server(pid: u32) -> io::Result<Pipe> {
    // Only this Windows user and SYSTEM may connect. An explicit medium label lets
    // a standard game report errors to the same user's elevated launcher.
    let sid = current_user_sid()?;
    let sddl = wide(&format!("D:P(A;;GA;;;SY)(A;;GA;;;{sid})S:(ML;;NW;;;ME)"));
    let mut descriptor = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let descriptor = Descriptor(descriptor);
    let mut attributes = SecurityAttributes {
        length: std::mem::size_of::<SecurityAttributes>() as u32,
        descriptor: descriptor.0,
        inherit: 0,
    };
    let pipe = unsafe {
        CreateNamedPipeW(
            name(pid).as_ptr(),
            3 | 0x00080000,
            8,
            1,
            4096,
            65536,
            100,
            (&mut attributes as *mut SecurityAttributes).cast(),
        )
    };
    if pipe as isize == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(Pipe(pipe))
    }
}
fn read_exact(pipe: Handle, bytes: &mut [u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        let mut count = 0;
        let ok = unsafe {
            ReadFile(
                pipe,
                bytes[offset..].as_mut_ptr(),
                (bytes.len() - offset).min(65536) as u32,
                &mut count,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        offset += count as usize;
    }
    Ok(())
}
fn write_all(pipe: Handle, bytes: &[u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        let mut count = 0;
        let ok = unsafe {
            WriteFile(
                pipe,
                bytes[offset..].as_ptr(),
                (bytes.len() - offset).min(65536) as u32,
                &mut count,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if count == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        offset += count as usize;
    }
    Ok(())
}
pub fn send(pid: u32, message: &str) -> io::Result<()> {
    let path = name(pid);
    let mut handle = unsafe {
        CreateFileW(
            path.as_ptr(),
            0xc0000000,
            0,
            ptr::null_mut(),
            3,
            0,
            ptr::null_mut(),
        )
    };
    if handle as isize == -1 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(231) {
            return Err(error);
        }
        if unsafe { WaitNamedPipeW(path.as_ptr(), 1000) } == 0 {
            return Err(io::Error::last_os_error());
        }
        handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                0xc0000000,
                0,
                ptr::null_mut(),
                3,
                0,
                ptr::null_mut(),
            )
        };
        if handle as isize == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    let pipe = Pipe(handle);
    write_all(pipe.0, &(message.len() as u64).to_le_bytes())?;
    write_all(pipe.0, message.as_bytes())?;
    let mut ack = [0];
    read_exact(pipe.0, &mut ack)?;
    if ack[0] != 1 {
        return Err(io::Error::other("Diagnostic record was not acknowledged"));
    }
    Ok(())
}
pub struct Collector {
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Collector {
    pub fn start(pid: u32, records: Records) -> io::Result<Self> {
        // Reserve the name before injection, so clients never race initial pipe creation.
        let first = server(pid)?;
        let raw = first.0 as usize;
        std::mem::forget(first);
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let worker = thread::spawn(move || {
            let pipe = Pipe(raw as Handle);
            while !done.load(Ordering::SeqCst) {
                let connected = unsafe { ConnectNamedPipe(pipe.0, ptr::null_mut()) } != 0;
                if !connected && io::Error::last_os_error().raw_os_error() != Some(535) {
                    if !done.load(Ordering::SeqCst) {
                        records.record(format!(
                            "Diagnostic connection failed: {:?}",
                            io::Error::last_os_error()
                        ));
                    }
                    break;
                }
                let result = (|| -> io::Result<()> {
                    let mut length = [0; 8];
                    read_exact(pipe.0, &mut length)?;
                    let count = usize::try_from(u64::from_le_bytes(length))
                        .map_err(|_| io::Error::other("Diagnostic message length overflow"))?;
                    let mut bytes = Vec::new();
                    bytes.try_reserve_exact(count).map_err(io::Error::other)?;
                    bytes.resize(count, 0);
                    read_exact(pipe.0, &mut bytes)?;
                    records.record(String::from_utf8(bytes).map_err(io::Error::other)?);
                    // Acknowledge only after the complete text is available to the console.
                    write_all(pipe.0, &[1])?;
                    unsafe {
                        FlushFileBuffers(pipe.0);
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    if !done.load(Ordering::SeqCst) {
                        records.record(format!("Diagnostic transport failed: {error:?}"));
                    }
                }
                unsafe {
                    DisconnectNamedPipe(pipe.0);
                }
            }
        });
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }
}
impl Drop for Collector {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !worker.is_finished() && Instant::now() < deadline {
                unsafe {
                    CancelSynchronousIo(worker.as_raw_handle());
                }
                thread::sleep(Duration::from_millis(5));
            }
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
    }
}
#[derive(Default)]
pub struct Diagnostics {
    pub records: Records,
    collector: Option<Collector>,
}
impl Diagnostics {
    pub fn record(&self, message: impl AsRef<str>) {
        self.records.record(message);
    }
    pub fn connect(&mut self, pid: u32) -> Result<(), Box<dyn std::error::Error>> {
        self.record(format!(
            "Creating same-user diagnostic pipe for game PID {pid}"
        ));
        self.collector = Some(
            Collector::start(pid, self.records.clone())
                .map_err(|error| context(format!("Create diagnostic pipe for PID {pid}"), error))?,
        );
        self.record(format!("Diagnostic collector ready for game PID {pid}"));
        Ok(())
    }
    pub fn stop(&mut self) {
        self.collector = None;
    }
    pub fn report(&self, error: &dyn std::error::Error) -> String {
        let mut text = format!("Full error: {error}\nDebug: {error:?}\n");
        let mut source = error.source();
        while let Some(cause) = source {
            text.push_str(&format!("Caused by: {cause}\nDebug: {cause:?}\n"));
            source = cause.source();
        }
        text.push_str("\nDiagnostic trace:\n");
        for line in self.records.snapshot() {
            text.push_str(&line);
            text.push('\n');
        }
        text
    }
}
#[derive(Debug)]
struct Context {
    operation: String,
    source: Box<dyn std::error::Error>,
}
impl std::fmt::Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.operation, self.source)
    }
}
impl std::error::Error for Context {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}
pub fn context(
    operation: impl Into<String>,
    source: impl Into<Box<dyn std::error::Error>>,
) -> Box<dyn std::error::Error> {
    Box::new(Context {
        operation: operation.into(),
        source: source.into(),
    })
}
pub fn os_error_code(mut error: &(dyn std::error::Error + 'static)) -> Option<i32> {
    loop {
        if let Some(error) = error.downcast_ref::<io::Error>() {
            if let Some(code) = error.raw_os_error() {
                return Some(code);
            }
        }
        error = error.source()?;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_unicode_records_are_not_truncated() {
        let pid = unsafe { GetCurrentProcessId() };
        let records = Records::default();
        let _collector = Collector::start(pid, records.clone()).unwrap();
        let text = format!("STACK BEGIN\n{}\nSTACK END", "错误与调用栈\n".repeat(10000));
        send(pid, &text).unwrap();
        assert!(records.snapshot().iter().any(|line| line.ends_with(&text)));
    }
    #[test]
    fn original_os_errors_and_traces_survive_formatting() {
        let diagnostics = Diagnostics::default();
        diagnostics.record("OpenProcess PID 42");
        let error = context("OpenProcess failed", io::Error::from_raw_os_error(5));
        let report = diagnostics.report(error.as_ref());
        assert!(report.contains("OpenProcess failed"));
        assert!(report.contains("code: 5"));
        assert!(report.contains("Caused by:"));
        assert!(report.contains("PID 42"));
    }
    #[test]
    fn nested_context_preserves_os_error_classification_and_api_details() {
        let error = context(
            "Waiting for Mono; PID=42",
            context(
                "CreateToolhelp32Snapshot failed; flags=0x18",
                io::Error::from_raw_os_error(24),
            ),
        );
        assert_eq!(os_error_code(error.as_ref()), Some(24));
        let report = Diagnostics::default().report(error.as_ref());
        assert!(report.contains("CreateToolhelp32Snapshot"));
        assert!(report.contains("code: 24"));
        assert!(report.contains("PID=42"));
        assert_eq!(
            os_error_code(
                context("Loading game modules", io::Error::from_raw_os_error(299)).as_ref()
            ),
            Some(299)
        );
        let error = context(
            "QueryFullProcessImageNameW failed",
            io::Error::from_raw_os_error(31),
        );
        assert_eq!(os_error_code(error.as_ref()), Some(31));
        assert_eq!(
            os_error_code(context("plain error", "no OS code").as_ref()),
            None
        );
    }
}
