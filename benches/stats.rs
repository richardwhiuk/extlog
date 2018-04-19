//! Benchmarks for stats
//!

#[macro_use]
extern crate bencher;
extern crate erased_serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;
#[macro_use]
extern crate slog_extlog;
#[macro_use]
extern crate slog_extlog_derive;
extern crate tokio_core;

use tokio_core::reactor::Core;

use bencher::Bencher;
use slog_extlog::stats::*;

const CRATE_LOG_NAME: &str = "SLOG_STATS_BENCH";

define_stats! {
    SLOG_TEST_STATS = {
        // Some simple counters
        TestCount(Counter, "Test counter", []),
        TestFooGauge(Gauge, "Test gauge of foo", []),
        TestCountGrouped(Counter,
                         "Test counter grouped by name", ["name"]),
        TestCountDoubleGrouped(Counter,
                               "Test counter grouped by name and error",
                               ["name", "error"])
    }
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "1", Text = "Some foo", Level = "Info")]
#[StatTrigger(StatName = "TestCount", Action = "Incr", Value = "1")]
//LCOV_EXCL_START
struct FooCountLog;

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "1", Text = "Some foo", Level = "Info")]
#[StatTrigger(StatName = "TestFooGauge", Action = "Incr", ValueFrom = "self.foo")]
//LCOV_EXCL_START
struct FooIncrLog {
    foo: u32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "2", Text = "Some foo lost", Level = "Info")]
#[StatTrigger(StatName = "TestFooGauge", Action = "Decr", ValueFrom = "self.foo")]
struct FooDecrLog {
    foo: u32,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "3", Text = "A string of text", Level = "Warning")]
#[StatTrigger(StatName = "TestCountGrouped", Action = "Incr", Value = "1")]
struct ThirdExternalLog {
    #[StatGroup(StatName = "TestCountGrouped")]
    name: String,
}

#[derive(ExtLoggable, Clone, Serialize)]
#[LogDetails(Id = "4", Text = "Some more irrelevant text", Level = "Info")]
#[StatTrigger(StatName = "TestCountDoubleGrouped", Action = "Incr", Value = "1")]
struct FourthExternalLog {
    #[StatGroup(StatName = "TestCountDoubleGrouped")]
    name: String,
    foo_count: u32,
    #[StatGroup(StatName = "TestCountDoubleGrouped")]
    error: u8,
}
//LCOV_EXCL_STOP

fn setup_logger() -> StatisticsLogger<DefaultStatisticsLogFormatter> {
    // Use the same tokio core for speed.
    let core = Core::new().expect("Failed to initialize tokio core");
    StatisticsLogger::new(
        slog::Logger::root(slog::Discard, o!()),
        StatsConfigBuilder::new(DefaultStatisticsLogFormatter)
            .with_stats(SLOG_TEST_STATS)
            .with_core(core.handle())
            .fuse(), // LCOV_EXCL_LINE Kcov bug?
    )
}

// Benchmark a log that increments a counter unconditionally.
fn single_count_log(bench: &mut Bencher) {
    let logger = setup_logger();
    bench.iter(|| {
        xlog!(logger, FooCountLog);
    })
}

// Benchmark a log that increments a gauge by an amount in the log.
fn gauge_incr_log(bench: &mut Bencher) {
    let logger = setup_logger();
    bench.iter(|| {
        xlog!(logger, FooIncrLog { foo: 123 });
    })
}

// Benchmark a pair of logs: the first increments a gauge by an amount in the log, the
// second decrements it by an amount in the second log.
fn gauge_incr_decr_log(bench: &mut Bencher) {
    let logger = setup_logger();
    bench.iter(|| {
        xlog!(logger, FooIncrLog { foo: 42 });
        xlog!(logger, FooIncrLog { foo: 27 });
    })
}

// Increment a counter grouped by a value in the log, where each log uses the same value.
fn single_grouped_counter_one_bucket(bench: &mut Bencher) {
    let logger = setup_logger();
    let name = "my_name".to_string();
    bench.iter(|| {
        xlog!(logger, ThirdExternalLog { name: name.clone() });
    })
}

// Increment a counter grouped by a value in the log, where each log uses a new value.
fn single_grouped_counter_multi_bucket(bench: &mut Bencher) {
    let logger = setup_logger();
    let mut idx = 0;
    bench.iter(|| {
        xlog!(
            logger,
            ThirdExternalLog {
                name: format!("name-{}", idx),
            }
        );
        idx += 1;
    })
}

// Increment a counter grouped by two value in the logs, where each log uses the same value for
// the first group and a different value for the second.
fn double_grouped_counter(bench: &mut Bencher) {
    let logger = setup_logger();
    let name = "my_name".to_string();
    let mut idx = 0;
    bench.iter(|| {
        xlog!(
            logger,
            FourthExternalLog {
                name: name.clone(),
                error: idx,
                foo_count: 4242,
            }
        );
        idx += 1;
    })
}

benchmark_group!(
    benches,
    single_count_log,
    gauge_incr_log,
    gauge_incr_decr_log,
    single_grouped_counter_one_bucket,
    single_grouped_counter_multi_bucket,
    double_grouped_counter
);
benchmark_main!(benches);
