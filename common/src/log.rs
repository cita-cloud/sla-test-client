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

use tracing::Level;
use tracing_subscriber::{
    fmt::{format::Writer, time::FormatTime},
    EnvFilter,
};

pub fn set_log(filter: &str) {
    struct LocalTimer;
    impl FormatTime for LocalTimer {
        fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
            write!(w, "{}", chrono::Local::now().format("%m-%d %T%.3f"))
        }
    }
    let filter = EnvFilter::try_new(filter).unwrap();
    // tracing 初始化
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_timer(LocalTimer)
        .with_thread_ids(true)
        .with_env_filter(filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
