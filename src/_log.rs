#[cfg(test)]
macro_rules! log {
    ($($arg:tt)*) => {
        println!("[LOG] {}", format!($($arg)*));
    }
}

#[cfg(not(test))]
macro_rules! log {
    ($($arg:tt)*) => {};
}

pub(crate) use log;
