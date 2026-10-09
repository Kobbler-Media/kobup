use dashmap::DashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Simple in-memory IP-based rate limiter using a sliding window.
pub struct RateLimiter {
    map: DashMap<IpAddr, (u32, Instant)>,
    max_attempts: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        Self {
            map: DashMap::new(),
            max_attempts,
            window,
        }
    }

    /// Check and increment the counter for the given IP.
    /// Returns `true` if under the limit, `false` if rate limited.
    pub fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();

        // Sweep entries outside the window so one-off IPs don't accumulate
        // forever; this runs on every call, so it stays amortized O(1).
        self.map
            .retain(|_, (_, window_start)| now.duration_since(*window_start) <= self.window);

        let mut entry = self.map.entry(ip).or_insert((0, now));
        let (count, window_start) = entry.value_mut();

        if now.duration_since(*window_start) > self.window {
            *count = 1;
            *window_start = now;
            true
        } else if *count < self.max_attempts {
            *count += 1;
            true
        } else {
            false
        }
    }
}

/// Picks the client IP out of a (possibly attacker-extended) X-Forwarded-For
/// chain. `trusted_hops` is how many reverse proxies you actually run in
/// front of this service (default 1, the bundled nginx) — the client IP is
/// the entry that many positions from the *right*, since a client can
/// prepend arbitrary junk to the left of the header but can't add entries
/// after the hops your own infrastructure appends. If the header has fewer
/// entries than `trusted_hops`, or is absent/unparseable, fall back to the
/// actual TCP peer address instead of trusting attacker-controlled input.
pub fn resolve_client_ip(
    forwarded_for: Option<&str>,
    trusted_hops: usize,
    peer_ip: IpAddr,
) -> IpAddr {
    let trusted_hops = trusted_hops.max(1);
    let Some(header) = forwarded_for else {
        return peer_ip;
    };

    let entries: Vec<IpAddr> = header
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<IpAddr>().ok())
        .collect();

    if entries.len() < trusted_hops {
        return peer_ip;
    }

    entries[entries.len() - trusted_hops]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn no_header_falls_back_to_peer_ip() {
        assert_eq!(resolve_client_ip(None, 1, ip("10.0.0.1")), ip("10.0.0.1"));
    }

    #[test]
    fn single_trusted_hop_takes_rightmost_entry() {
        // Client-prepended junk on the left must not be trusted; only the
        // entry our own proxy appended (the rightmost) counts.
        let header = "6.6.6.6, 203.0.113.9";
        assert_eq!(
            resolve_client_ip(Some(header), 1, ip("10.0.0.1")),
            ip("203.0.113.9")
        );
    }

    #[test]
    fn two_trusted_hops_takes_second_from_right() {
        let header = "6.6.6.6, 203.0.113.9, 192.0.2.5";
        assert_eq!(
            resolve_client_ip(Some(header), 2, ip("10.0.0.1")),
            ip("203.0.113.9")
        );
    }

    #[test]
    fn fewer_entries_than_trusted_hops_falls_back_to_peer_ip() {
        let header = "203.0.113.9";
        assert_eq!(
            resolve_client_ip(Some(header), 2, ip("10.0.0.1")),
            ip("10.0.0.1")
        );
    }

    #[test]
    fn unparseable_header_falls_back_to_peer_ip() {
        assert_eq!(
            resolve_client_ip(Some("not-an-ip"), 1, ip("10.0.0.1")),
            ip("10.0.0.1")
        );
    }

    #[test]
    fn check_sweeps_expired_entries() {
        let limiter = RateLimiter::new(5, Duration::from_millis(10));
        assert!(limiter.check(ip("192.0.2.1")));
        std::thread::sleep(Duration::from_millis(20));
        // 192.0.2.1's entry is now outside the window. A check() for a
        // *different* IP should sweep it away rather than let it sit
        // forever, so the map should end up holding only the new entry.
        assert!(limiter.check(ip("192.0.2.2")));
        assert_eq!(limiter.map.len(), 1);
        assert!(limiter.map.contains_key(&ip("192.0.2.2")));
    }
}
