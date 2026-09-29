//! Send assistant notifications to the connected desktop interface.

pub fn notify(title: &str, message: &str) {
    let (primary, detail) = message
        .split_once('\n')
        .map_or((message, None), |(primary, detail)| (primary, Some(detail)));

    #[cfg(feature = "jarvis_app")]
    if crate::ipc::has_clients() {
        crate::ipc::send(crate::ipc::IpcEvent::Notification {
            title: title.to_owned(),
            primary: primary.to_owned(),
            detail: detail.map(str::to_owned),
        });
        crate::ipc::send(crate::ipc::IpcEvent::RevealWindow);
        return;
    }
    log::warn!("JARVIS GUI is not connected; notification not shown: {} — {}", title, primary);
    let _ = detail;
}
