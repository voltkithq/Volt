use crate::command;

pub(super) const MAX_COMMANDS_PER_TICK: usize = 64;

pub(super) fn drain_command_batch(
    cmd_rx: &std::sync::mpsc::Receiver<command::CommandEnvelope>,
    max_commands: usize,
    mut process_command: impl FnMut(command::CommandEnvelope) -> bool,
) -> (usize, bool, bool) {
    let mut processed = 0;

    while processed < max_commands {
        let Ok(envelope) = cmd_rx.try_recv() else {
            return (processed, false, false);
        };
        processed += 1;
        if process_command(envelope) {
            return (processed, false, true);
        }
    }

    (processed, true, false)
}

#[cfg(test)]
mod tests {
    use super::{MAX_COMMANDS_PER_TICK, drain_command_batch};
    use crate::command::{AppCommand, CommandEnvelope};
    use std::sync::mpsc;
    use std::time::Instant;

    fn envelope(trace_id: u64) -> CommandEnvelope {
        CommandEnvelope {
            trace_id,
            enqueued_at: Instant::now(),
            command: AppCommand::Quit,
        }
    }

    #[test]
    fn test_drain_command_batch_limits_work_per_tick() {
        let (tx, rx) = mpsc::channel();
        for trace_id in 0..=MAX_COMMANDS_PER_TICK as u64 {
            tx.send(envelope(trace_id)).unwrap();
        }
        drop(tx);

        let (processed, reached_batch_limit, should_shutdown) =
            drain_command_batch(&rx, MAX_COMMANDS_PER_TICK, |_envelope| false);

        assert_eq!(processed, MAX_COMMANDS_PER_TICK);
        assert!(reached_batch_limit);
        assert!(!should_shutdown);
    }

    #[test]
    fn test_drain_command_batch_stops_when_channel_is_empty() {
        let (tx, rx) = mpsc::channel();
        for trace_id in 0..3_u64 {
            tx.send(envelope(trace_id)).unwrap();
        }
        drop(tx);

        let (processed, reached_batch_limit, should_shutdown) =
            drain_command_batch(&rx, MAX_COMMANDS_PER_TICK, |_envelope| false);

        assert_eq!(processed, 3);
        assert!(!reached_batch_limit);
        assert!(!should_shutdown);
    }

    #[test]
    fn test_drain_command_batch_stops_when_command_requests_shutdown() {
        let (tx, rx) = mpsc::channel();
        tx.send(envelope(1)).unwrap();
        tx.send(envelope(2)).unwrap();
        drop(tx);

        let (processed, reached_batch_limit, should_shutdown) =
            drain_command_batch(&rx, MAX_COMMANDS_PER_TICK, |envelope| {
                envelope.trace_id == 1
            });

        assert_eq!(processed, 1);
        assert!(!reached_batch_limit);
        assert!(should_shutdown);
    }
}
