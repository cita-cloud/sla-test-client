pub fn unix_now() -> u64 {
    let d = ::std::time::UNIX_EPOCH.elapsed().unwrap();
    d.as_secs() * 1_000 + u64::from(d.subsec_millis())
}

pub fn ms_to_minute_scale(ms: u64) -> u64 {
    (ms as f64 / 1_000.0 / 60.0).trunc() as u64
}
