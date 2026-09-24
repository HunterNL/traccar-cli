pub async fn send_notification(title: &str, content: &str) {
    let handle = &mut notify_rust::Notification::new()
        .summary(title)
        .body(content)
        .timeout(0)
        .hint(notify_rust::Hint::Transient(false))
        .show_async()
        .await;
    if let Err(e) = handle {
        eprintln!("Error sending notification: {e}");
    }
}
