extern crate async_i3ipc;
extern crate env_logger;
extern crate notify_rust;
extern crate log;
extern crate url;
extern crate serde;
extern crate serde_json;
extern crate fuzzy_matcher;

pub mod browser;
pub mod message;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
