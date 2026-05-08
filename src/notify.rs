pub async fn send_notification(title: &str, content: &str) {
    let handle = notify_rust::Notification::new()
        .summary(title)
        .body(content)
        .show_async()
        .await;
    if let Err(e) = handle {
        eprintln!("Error sending notification: {e}")
    }
}
