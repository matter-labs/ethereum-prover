pub(crate) fn extract_panic_message(err: tokio::task::JoinError) -> String {
    if err.is_panic() {
        let panic = err.into_panic();
        if let Some(s) = panic.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = panic.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "Unknown panic".to_string()
        }
    } else {
        "Task was cancelled".to_string()
    }
}
