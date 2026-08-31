use std::sync::atomic::{AtomicU64, Ordering};

/// Process-lifetime counters, exposed at `GET /metrics` in Prometheus text
/// format. Never records the code or the secret — only request outcomes.
#[derive(Default)]
pub struct Metrics {
    pub webhook_ok: AtomicU64,
    pub webhook_unauthorized: AtomicU64,
    pub webhook_invalid: AtomicU64,
    pub webhook_error: AtomicU64,
    pub codes_ok: AtomicU64,
    pub codes_unauthorized: AtomicU64,
    pub codes_error: AtomicU64,
    pub codes_stored: AtomicU64,
}

impl Metrics {
    pub fn render(&self) -> String {
        let g = |a: &AtomicU64| a.load(Ordering::Relaxed);
        let mut s = String::new();
        s.push_str("# HELP webhook_requests_total POST /webhook requests by outcome.\n");
        s.push_str("# TYPE webhook_requests_total counter\n");
        s.push_str(&format!("webhook_requests_total{{outcome=\"ok\"}} {}\n", g(&self.webhook_ok)));
        s.push_str(&format!("webhook_requests_total{{outcome=\"unauthorized\"}} {}\n", g(&self.webhook_unauthorized)));
        s.push_str(&format!("webhook_requests_total{{outcome=\"invalid\"}} {}\n", g(&self.webhook_invalid)));
        s.push_str(&format!("webhook_requests_total{{outcome=\"error\"}} {}\n", g(&self.webhook_error)));
        s.push_str("# HELP codes_requests_total GET /codes requests by outcome.\n");
        s.push_str("# TYPE codes_requests_total counter\n");
        s.push_str(&format!("codes_requests_total{{outcome=\"ok\"}} {}\n", g(&self.codes_ok)));
        s.push_str(&format!("codes_requests_total{{outcome=\"unauthorized\"}} {}\n", g(&self.codes_unauthorized)));
        s.push_str(&format!("codes_requests_total{{outcome=\"error\"}} {}\n", g(&self.codes_error)));
        s.push_str("# HELP codes_stored_total Non-duplicate codes stored.\n");
        s.push_str("# TYPE codes_stored_total counter\n");
        s.push_str(&format!("codes_stored_total {}\n", g(&self.codes_stored)));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_reports_counters_and_zeros() {
        let m = Metrics::default();
        m.webhook_ok.fetch_add(2, Ordering::Relaxed);
        m.codes_stored.fetch_add(2, Ordering::Relaxed);
        let out = m.render();
        assert!(out.contains("webhook_requests_total{outcome=\"ok\"} 2"), "{out}");
        assert!(out.contains("codes_stored_total 2"), "{out}");
        assert!(out.contains("# TYPE webhook_requests_total counter"), "{out}");
        // untouched counters still render as 0
        assert!(out.contains("webhook_requests_total{outcome=\"error\"} 0"), "{out}");
    }
}
