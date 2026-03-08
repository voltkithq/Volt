use std::time::Duration;

pub(super) fn log_command_observability(
    trace_id: u64,
    queue_delay: Duration,
    processing_duration: Duration,
) {
    if processing_duration >= Duration::from_millis(50) {
        log::warn!(
            "Slow command trace={trace_id} queue_delay_ms={} processing_ms={}",
            queue_delay.as_millis(),
            processing_duration.as_millis()
        );
    } else {
        log::debug!(
            "Command trace={trace_id} queue_delay_ms={} processing_ms={}",
            queue_delay.as_millis(),
            processing_duration.as_millis()
        );
    }
}
