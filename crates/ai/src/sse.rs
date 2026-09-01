//! Minimal Server-Sent-Events frame decoder.
//!
//! Feeds raw byte chunks in, yields complete events. Handles the subset every
//! LLM provider uses: `event:` and (possibly multi-line) `data:` fields,
//! events terminated by a blank line, `\n` or `\r\n` line endings.

/// One decoded SSE event.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SseEvent {
    /// The `event:` field value (empty when absent — provider default "message").
    pub event: String,
    /// The joined `data:` payload (multiple `data:` lines joined with `\n`).
    pub data: String,
}

/// Incremental SSE parser.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buf: String,
    cur_event: String,
    cur_data: Vec<String>,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a chunk of bytes; returns any events completed by this chunk.
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buf.push_str(&String::from_utf8_lossy(chunk));
        let mut out = Vec::new();
        while let Some(nl) = self.buf.find('\n') {
            let mut line = self.buf[..nl].to_string();
            self.buf.drain(..=nl);
            if line.ends_with('\r') {
                line.pop();
            }
            if line.is_empty() {
                if let Some(ev) = self.take_event() {
                    out.push(ev);
                }
                continue;
            }
            if let Some(stripped) = line.strip_prefix(':') {
                let _ = stripped; // comment / keep-alive
                continue;
            }
            let (field, value) = match line.split_once(':') {
                Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
                None => (line.as_str(), ""),
            };
            match field {
                "event" => self.cur_event = value.to_string(),
                "data" => self.cur_data.push(value.to_string()),
                _ => {}
            }
        }
        out
    }

    /// Flush a trailing event that wasn't terminated by a blank line (some
    /// servers close the connection right after the last `data:` line).
    pub fn finish(&mut self) -> Option<SseEvent> {
        if !self.buf.trim().is_empty() {
            let rest = std::mem::take(&mut self.buf);
            for raw in rest.split('\n') {
                let line = raw.trim_end_matches('\r');
                if let Some(v) = line.strip_prefix("data:") {
                    self.cur_data
                        .push(v.strip_prefix(' ').unwrap_or(v).to_string());
                } else if let Some(v) = line.strip_prefix("event:") {
                    self.cur_event = v.strip_prefix(' ').unwrap_or(v).to_string();
                }
            }
        }
        self.take_event()
    }

    fn take_event(&mut self) -> Option<SseEvent> {
        if self.cur_data.is_empty() && self.cur_event.is_empty() {
            return None;
        }
        let ev = SseEvent {
            event: std::mem::take(&mut self.cur_event),
            data: self.cur_data.join("\n"),
        };
        self.cur_data.clear();
        if ev.data.is_empty() && ev.event.is_empty() {
            None
        } else {
            Some(ev)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_events_on_blank_line() {
        let mut d = SseDecoder::new();
        let evs = d.push(b"data: hello\n\ndata: world\n\n");
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].data, "hello");
        assert_eq!(evs[1].data, "world");
    }

    #[test]
    fn handles_split_across_chunks_and_crlf() {
        let mut d = SseDecoder::new();
        assert!(d.push(b"event: message_st").is_empty());
        let evs = d.push(b"art\r\ndata: {\"a\":1}\r\n\r\n");
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].event, "message_start");
        assert_eq!(evs[0].data, "{\"a\":1}");
    }

    #[test]
    fn joins_multiline_data_and_ignores_comments() {
        let mut d = SseDecoder::new();
        let evs = d.push(b": keep-alive\ndata: line1\ndata: line2\n\n");
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].data, "line1\nline2");
    }

    #[test]
    fn finish_flushes_unterminated_event() {
        let mut d = SseDecoder::new();
        assert!(d.push(b"data: [DONE]").is_empty());
        assert_eq!(d.finish().unwrap().data, "[DONE]");
    }
}
