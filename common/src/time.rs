// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use chrono::prelude::*;

pub fn unix_now() -> u64 {
    let d = ::std::time::UNIX_EPOCH.elapsed().unwrap();
    d.as_secs() * 1_000 + u64::from(d.subsec_millis())
}

pub fn ms_to_minute_scale(ms: u64) -> u64 {
    (ms as f64 / 1_000.0 / 60.0).trunc() as u64
}

pub fn get_latest_finalized_minute(
    time: u64,
    check_timeout: u32,
    chain_block_interval: u32,
) -> u64 {
    ms_to_minute_scale(time - (check_timeout as u64 * chain_block_interval as u64 * 1000)) - 1
}

pub fn get_readable_time_from_millisecond(timestamp: u64) -> String {
    Utc.timestamp_opt((timestamp / 1000) as i64, 0)
        .unwrap()
        .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

pub fn get_readable_time_from_minute(timestamp: u64) -> String {
    Utc.timestamp_opt((timestamp * 60) as i64, 0)
        .unwrap()
        .with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap())
        .format("%Y-%m-%d %H:%M")
        .to_string()
}
