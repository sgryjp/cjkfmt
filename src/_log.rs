#[cfg(test)]
macro_rules! test_log {
    ($($arg:tt)*) => {
        println!("[TEST_LOG] {}", format!($($arg)*));
    }
}

#[cfg(not(test))]
macro_rules! test_log {
    ($($arg:tt)*) => {};
}

pub(crate) use test_log;
